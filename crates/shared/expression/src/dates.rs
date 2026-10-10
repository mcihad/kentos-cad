//! Tarih ve saat işlevleri (docs/adr/0214 §2.2): dates are ISO text as the
//! drawing keeps them (`YYYY-AA-GG`, `YYYY-AA-GGTSS:DD:ss[.mmm]`), read and
//! written by the time core (`kentos_geometry_core::time`, docs/adr/0210 §3)
//! and worked in whole milliseconds and calendar integers, never in floats
//! that could round. Zone-less wall time: a zone given in the text is turned
//! to UTC when it is read.

use kentos_geometry_core::jsmath::js_round;
use kentos_geometry_core::time::{self, civil_from_days, days_from_civil, days_in};

use crate::js::text::fold_turkish;
use crate::library::Func;
use crate::scalar::{R, Scratch, V, as_text, to_number};

const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;
/// The time core's years.
const YEARS: std::ops::RangeInclusive<i64> = 1..=9999;

/// A unit of `tarih_ekle` and `tarih_farkı`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
}

impl Unit {
    /// A unit's name, Turkish with or without its letters, or English (one or many).
    pub fn read(name: &str) -> Option<Unit> {
        Some(match fold_turkish(crate::js::text::trim(name)).as_str() {
            "YIL" | "YEAR" | "YEARS" => Unit::Year,
            "AY" | "MONTH" | "MONTHS" => Unit::Month,
            "HAFTA" | "WEEK" | "WEEKS" => Unit::Week,
            "GUN" | "DAY" | "DAYS" => Unit::Day,
            "SAAT" | "HOUR" | "HOURS" => Unit::Hour,
            "DAKIKA" | "MINUTE" | "MINUTES" => Unit::Minute,
            "SANIYE" | "SECOND" | "SECONDS" => Unit::Second,
            _ => return None,
        })
    }

    /// Milliseconds of a fixed unit (months and years are the calendar's).
    fn ms(self) -> Option<i64> {
        match self {
            Unit::Week => Some(WEEK),
            Unit::Day => Some(DAY),
            Unit::Hour => Some(HOUR),
            Unit::Minute => Some(MINUTE),
            Unit::Second => Some(SECOND),
            Unit::Year | Unit::Month => None,
        }
    }
}

/// A moment read from a value: its milliseconds and whether it was a date
/// alone (no time written). None when the value is no date.
pub fn moment(v: V) -> Option<(i64, bool)> {
    let V::Text(text) = v else {
        return None;
    };
    let t = time::read(text).moment()?;
    // Blanks trimmed as the time core trims them: a time follows a `T` or a space.
    let date_only = !text.trim_matches([' ', '\t']).contains(['T', ' ']);
    Some((t as i64, date_only))
}

/// A moment's parts: year, month, day and the milliseconds into the day.
fn parts(t: i64) -> (i64, i64, i64, i64) {
    let (y, m, d) = civil_from_days(t.div_euclid(DAY));
    (y, m, d, t.rem_euclid(DAY))
}

/// A whole number in `lo..=hi`, or None.
fn whole(v: V, lo: i64, hi: i64) -> Option<i64> {
    let x = to_number(v)?;
    (x.fract() == 0.0 && x >= lo as f64 && x <= hi as f64).then_some(x as i64)
}

/// A moment written: a date when `date`, else a date and a time (the time
/// core's rule: midnight writes as the date alone).
fn write(out: &mut String, t: i64, date: bool) -> R<'static> {
    let (y, ..) = parts(t);
    if !YEARS.contains(&y) {
        return R::V(V::Null);
    }
    out.push_str(&time::write(t as f64, date));
    R::Made
}

/// `tarih(yıl, ay, gün)` or `tarih(metin)`.
fn date(args: &[V], out: &mut String) -> R<'static> {
    if args.len() == 1 {
        let Some((t, _)) = moment(args[0]) else {
            return R::V(V::Null);
        };
        return write(out, t.div_euclid(DAY) * DAY, true);
    }
    let (Some(y), Some(m), Some(d)) = (
        args.first().and_then(|&v| whole(v, 1, 9999)),
        args.get(1).and_then(|&v| whole(v, 1, 12)),
        args.get(2).and_then(|&v| whole(v, 1, 31)),
    ) else {
        return R::V(V::Null);
    };
    if d > days_in(y, m) {
        return R::V(V::Null);
    }
    write(out, days_from_civil(y, m, d) * DAY, true)
}

/// `tarih_saat(yıl, ay, gün, saat, dakika, saniye)` or `tarih_saat(metin)`.
fn date_time(args: &[V], out: &mut String) -> R<'static> {
    if args.len() == 1 {
        let Some((t, _)) = moment(args[0]) else {
            return R::V(V::Null);
        };
        return write(out, t, false);
    }
    let field = |i: usize, lo: i64, hi: i64| match args.get(i) {
        None => Some(0),
        Some(&v) => whole(v, lo, hi),
    };
    let (Some(y), Some(m), Some(d)) = (
        args.first().and_then(|&v| whole(v, 1, 9999)),
        args.get(1).and_then(|&v| whole(v, 1, 12)),
        args.get(2).and_then(|&v| whole(v, 1, 31)),
    ) else {
        return R::V(V::Null);
    };
    let (Some(h), Some(mi)) = (field(3, 0, 23), field(4, 0, 59)) else {
        return R::V(V::Null);
    };
    let sec = match args.get(5) {
        None => Some(0.0),
        Some(&v) => to_number(v).filter(|x| (0.0..60.0).contains(x)),
    };
    let Some(sec) = sec else {
        return R::V(V::Null);
    };
    if d > days_in(y, m) {
        return R::V(V::Null);
    }
    let ms = js_round(sec * 1000.0) as i64;
    write(
        out,
        days_from_civil(y, m, d) * DAY + h * HOUR + mi * MINUTE + ms,
        false,
    )
}

/// ISO 8601's week of a day: weeks start on Monday, the first holds the year's first Thursday.
fn iso_week(days: i64) -> i64 {
    // 1970-01-01 was a Thursday: Monday is 0.
    let weekday = (days + 3).rem_euclid(7);
    let thursday = days - weekday + 3;
    let (y, ..) = civil_from_days(thursday);
    let first = days_from_civil(y, 1, 1);
    (thursday - first) / 7 + 1
}

/// Calendar months from `a` to `b`, whole: one less while `b`'s day and
/// time come before `a`'s (one more going back).
fn months_between(a: i64, b: i64) -> i64 {
    let (ya, ma, da, ta) = parts(a);
    let (yb, mb, db, tb) = parts(b);
    let mut months = (yb * 12 + mb) - (ya * 12 + ma);
    if months > 0 && (db, tb) < (da, ta) {
        months -= 1;
    } else if months < 0 && (db, tb) > (da, ta) {
        months += 1;
    }
    months
}

/// `t` plus `k` calendar months (a day past the month's end is its last day).
fn add_months(t: i64, k: i64) -> i64 {
    let (y, m, d, into) = parts(t);
    let total = y * 12 + (m - 1) + k;
    let (y2, m2) = (total.div_euclid(12), total.rem_euclid(12) + 1);
    let d2 = d.min(days_in(y2, m2));
    days_from_civil(y2, m2, d2) * DAY + into
}

const MONTHS: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
    "Kasım", "Aralık",
];
const MONTHS_SHORT: [&str; 12] = [
    "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara",
];
const DAYS: [&str; 7] = [
    "Pazartesi",
    "Salı",
    "Çarşamba",
    "Perşembe",
    "Cuma",
    "Cumartesi",
    "Pazar",
];
const DAYS_SHORT: [&str; 7] = ["Pzt", "Sal", "Çar", "Per", "Cum", "Cmt", "Paz"];

/// `tarih_biçimle`'s tokens, longest first.
const TOKENS: [&str; 16] = [
    "YYYY", "AAAA", "GGGG", "AAA", "GGG", "YY", "AA", "GG", "SS", "DD", "ss", "A", "G", "S", "D",
    "s",
];

/// A moment through a format (§2.2): tokens replaced, text in single quotes as it is.
fn format(out: &mut String, t: i64, fmt: &str) {
    let (y, m, d, into) = parts(t);
    let weekday = (t.div_euclid(DAY) + 3).rem_euclid(7) as usize;
    let (h, mi, sec) = (into / HOUR, into % HOUR / MINUTE, into % MINUTE / SECOND);
    let mut rest = fmt;
    while let Some(c) = rest.chars().next() {
        if c == '\'' {
            // Quoted text; '' inside is one quote.
            let body = &rest[1..];
            match body.find('\'') {
                Some(0) => {
                    out.push('\'');
                    rest = &body[1..];
                }
                Some(k) => {
                    out.push_str(&body[..k]);
                    rest = &body[k + 1..];
                }
                None => {
                    out.push_str(body);
                    rest = "";
                }
            }
            continue;
        }
        let Some(tok) = TOKENS.iter().find(|k| rest.starts_with(**k)) else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
            continue;
        };
        let two = |out: &mut String, n: i64| out.push_str(&format!("{n:02}"));
        match *tok {
            "YYYY" => out.push_str(&format!("{y:04}")),
            "YY" => two(out, y.rem_euclid(100)),
            "AAAA" => out.push_str(MONTHS[(m - 1) as usize]),
            "AAA" => out.push_str(MONTHS_SHORT[(m - 1) as usize]),
            "AA" => two(out, m),
            "A" => out.push_str(&m.to_string()),
            "GGGG" => out.push_str(DAYS[weekday]),
            "GGG" => out.push_str(DAYS_SHORT[weekday]),
            "GG" => two(out, d),
            "G" => out.push_str(&d.to_string()),
            "SS" => two(out, h),
            "S" => out.push_str(&h.to_string()),
            "DD" => two(out, mi),
            "D" => out.push_str(&mi.to_string()),
            "ss" => two(out, sec),
            _ => out.push_str(&sec.to_string()),
        }
        rest = &rest[tok.len()..];
    }
}

/// A date and time function (`f` one of them) on its arguments.
pub fn call(f: Func, args: &[V], out: &mut String, s: &mut Scratch) -> R<'static> {
    let arg = |i: usize| args.get(i).copied().unwrap_or(V::Null);
    match f {
        Func::Date => date(args, out),
        Func::DateTime => date_time(args, out),
        Func::Year
        | Func::Month
        | Func::Day
        | Func::Hour
        | Func::Minute
        | Func::Second
        | Func::Week
        | Func::Weekday
        | Func::DayOfYear => {
            let Some((t, _)) = moment(arg(0)) else {
                return R::V(V::Null);
            };
            let (y, m, d, into) = parts(t);
            let days = t.div_euclid(DAY);
            R::V(V::Num(match f {
                Func::Year => y as f64,
                Func::Month => m as f64,
                Func::Day => d as f64,
                Func::Hour => (into / HOUR) as f64,
                Func::Minute => (into % HOUR / MINUTE) as f64,
                Func::Second => (into % MINUTE) as f64 / 1000.0,
                Func::Week => iso_week(days) as f64,
                Func::Weekday => ((days + 3).rem_euclid(7) + 1) as f64,
                _ => (days - days_from_civil(y, 1, 1) + 1) as f64,
            }))
        }
        Func::DateAdd => {
            let Some((t, date_only)) = moment(arg(0)) else {
                return R::V(V::Null);
            };
            let (Some(n), Some(unit)) = (to_number(arg(1)), Unit::read(as_text(arg(2), &mut s.b)))
            else {
                return R::V(V::Null);
            };
            let r = match unit.ms() {
                Some(ms) => {
                    let delta = js_round(n * ms as f64);
                    if !delta.is_finite() || delta.abs() > 4e14 {
                        return R::V(V::Null);
                    }
                    t + delta as i64
                }
                None => {
                    let k = js_round(n * if unit == Unit::Year { 12.0 } else { 1.0 });
                    if k.abs() > 200_000.0 {
                        return R::V(V::Null);
                    }
                    add_months(t, k as i64)
                }
            };
            // A date stays a date when whole days are added to it.
            let keeps = date_only && r.rem_euclid(DAY) == 0;
            write(out, r, keeps)
        }
        Func::DateDiff => {
            let (Some((a, _)), Some((b, _))) = (moment(arg(0)), moment(arg(1))) else {
                return R::V(V::Null);
            };
            let Some(unit) = Unit::read(as_text(arg(2), &mut s.c)) else {
                return R::V(V::Null);
            };
            R::V(V::Num(match unit.ms() {
                Some(ms) => (b - a) as f64 / ms as f64,
                None => {
                    let months = months_between(a, b);
                    if unit == Unit::Year {
                        (months / 12) as f64
                    } else {
                        months as f64
                    }
                }
            }))
        }
        Func::DateFormat => {
            let Some((t, _)) = moment(arg(0)) else {
                return R::V(V::Null);
            };
            if arg(1) == V::Null {
                return R::V(V::Null);
            }
            let fmt = as_text(arg(1), &mut s.b).to_owned();
            format(out, t, &fmt);
            R::Made
        }
        _ => R::V(V::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(f: Func, args: &[V]) -> String {
        let mut out = String::new();
        match call(f, args, &mut out, &mut Scratch::default()) {
            R::Made => out,
            R::V(V::Null) => "boş".into(),
            R::V(V::Num(x)) => x.to_string(),
            r => format!("{r:?}"),
        }
    }

    #[test]
    fn dates_are_made_read_and_written() {
        assert_eq!(
            text(Func::Date, &[V::Num(2024.0), V::Num(2.0), V::Num(29.0)]),
            "2024-02-29"
        );
        assert_eq!(
            text(Func::Date, &[V::Num(2023.0), V::Num(2.0), V::Num(29.0)]),
            "boş"
        );
        assert_eq!(
            text(Func::Date, &[V::Text("05.03.2021 14:30")]),
            "2021-03-05"
        );
        assert_eq!(
            text(
                Func::DateTime,
                &[
                    V::Num(2021.0),
                    V::Num(3.0),
                    V::Num(5.0),
                    V::Num(14.0),
                    V::Num(30.0),
                    V::Num(5.5)
                ]
            ),
            "2021-03-05T14:30:05.500"
        );
        assert_eq!(text(Func::Week, &[V::Text("2021-01-03")]), "53");
        assert_eq!(text(Func::Week, &[V::Text("2021-01-04")]), "1");
        assert_eq!(text(Func::Weekday, &[V::Text("2026-10-10")]), "6");
        assert_eq!(text(Func::DayOfYear, &[V::Text("2024-12-31")]), "366");
        assert_eq!(
            text(
                Func::DateAdd,
                &[V::Text("2024-01-31"), V::Num(1.0), V::Text("ay")]
            ),
            "2024-02-29"
        );
        assert_eq!(
            text(
                Func::DateAdd,
                &[V::Text("2024-01-31"), V::Num(36.0), V::Text("saat")]
            ),
            "2024-02-01T12:00:00"
        );
        assert_eq!(
            text(
                Func::DateDiff,
                &[V::Text("2024-01-31"), V::Text("2024-02-29"), V::Text("ay")]
            ),
            "0"
        );
        assert_eq!(
            text(
                Func::DateDiff,
                &[V::Text("2020-02-29"), V::Text("2024-02-28"), V::Text("yıl")]
            ),
            "3"
        );
        assert_eq!(
            text(
                Func::DateFormat,
                &[
                    V::Text("2026-10-10T09:05:07"),
                    V::Text("GGGG, G AAAA YYYY 'saat' SS:DD")
                ]
            ),
            "Cumartesi, 10 Ekim 2026 saat 09:05"
        );
    }
}
