//! A workbook's dates and times (docs/adr/0223 §1): an Excel number in a
//! date or time format is read as the ISO text of what Excel shows, so a
//! sheet's dates come in as dates and not as day counts.
//!
//! Which formats are dates is openpyxl's rule (`is_date_format`): the first
//! section of the format, its quoted text and its bracketed parts left out
//! (but [h], [hh], [m], [mm], [s], [ss]), holds d, m, h, y or s not straight
//! after _ or \. A format that starts with an elapsed time ([h], [m], [s],
//! either case) stays a number. Of a date format, d or y (or an m with no h
//! and no s: a month) make it a date, h or s a time, both a date and time.
//!
//! The day count is Excel's 1900 system (day 1 is 1 January 1900; below 60,
//! where Excel counts a 29 February 1900 that was not, one day later:
//! openpyxl's) or its 1904 system (day 0 is 1 January 1904, the workbook's
//! `date1904`). A value below 1 is a time of day; a date's time is the
//! fraction of its day rounded to the second (23:59:59.5 the next day's
//! 00:00:00), a date alone its day. A negative value, or one past
//! 9999-12-31, stays the number written.

/// What a date format shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Clock {
    Date,
    Time,
    DateTime,
}

/// The format of Excel's built-in number formats that show dates or times.
pub(super) fn builtin(id: u32) -> Option<&'static str> {
    Some(match id {
        14 => "mm-dd-yy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "m/d/yy h:mm",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mmss.0",
        _ => return None,
    })
}

/// The format's first section without its quoted text and its bracketed
/// parts, [h], [hh], [m], [mm], [s] and [ss] kept.
fn stripped(first: &str) -> String {
    let mut out = String::with_capacity(first.len());
    let mut rest = first;
    while let Some(c) = rest.chars().next() {
        match c {
            '"' => {
                // Up to the next quote (none: the rest kept, as the regex leaves it).
                match rest[1..].find('"') {
                    Some(end) => rest = &rest[end + 2..],
                    None => {
                        out.push_str(rest);
                        break;
                    }
                }
            }
            '[' => match rest.find(']') {
                Some(end) => {
                    let inner = &rest[1..end];
                    if matches!(inner, "h" | "hh" | "m" | "mm" | "s" | "ss") {
                        out.push_str(&rest[..=end]);
                    }
                    rest = &rest[end + 1..];
                }
                None => {
                    out.push_str(rest);
                    break;
                }
            },
            _ => {
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out
}

/// What a number format shows of a date, if it is a date format.
pub(super) fn clock(format: &str) -> Option<Clock> {
    let first = format.split(';').next().unwrap_or("");
    let lower = first.to_ascii_lowercase();
    if ["[h]", "[hh]", "[m]", "[mm]", "[s]", "[ss]"]
        .iter()
        .any(|e| lower.starts_with(e))
    {
        return None;
    }
    let kept: Vec<char> = stripped(first).chars().collect();
    let (mut date, mut time, mut month) = (false, false, false);
    for (i, c) in kept.iter().enumerate() {
        if i > 0 && matches!(kept[i - 1], '_' | '\\') {
            continue;
        }
        match c.to_ascii_lowercase() {
            'd' | 'y' => date = true,
            'h' | 's' => time = true,
            'm' => month = true,
            _ => {}
        }
    }
    let date = date || (month && !time);
    match (date, time) {
        (true, true) => Some(Clock::DateTime),
        (true, false) => Some(Clock::Date),
        (false, true) => Some(Clock::Time),
        (false, false) => None,
    }
}

/// Days from 1970-01-01 of a day of the proleptic Gregorian calendar (Howard Hinnant's).
pub(super) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The day of `days` from 1970-01-01.
pub(super) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if m <= 2 {
            yoe + era * 400 + 1
        } else {
            yoe + era * 400
        },
        m,
        d,
    )
}

/// The day of Excel's last date, 9999-12-31, in the 1900 system.
const LAST: f64 = 2_958_465.0;

/// The ISO text of `value` (a cell's number as written) under `clock`; none
/// when it is no such date (then the number stays as written).
pub(super) fn text(value: &str, clock: Clock, date1904: bool) -> Option<String> {
    let v: f64 = value.trim().parse().ok()?;
    if !v.is_finite() || !(0.0..=LAST + 1.0).contains(&v) {
        return None;
    }
    let whole = v.floor();
    let mut day = whole as i64;
    let mut secs = ((v - whole) * 86_400.0).round() as i64;
    if secs >= 86_400 {
        secs -= 86_400;
        if clock != Clock::Date {
            day += 1;
        }
    }
    let time = format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60);
    if clock == Clock::Time || (v < 1.0 && day == 0) {
        return Some(time);
    }
    let epoch = if date1904 {
        days_from_civil(1904, 1, 1)
    } else {
        days_from_civil(1899, 12, 30) + i64::from(v > 0.0 && v < 60.0)
    };
    let (y, m, d) = civil_from_days(epoch + day);
    if y > 9999 {
        return None;
    }
    Some(match clock {
        Clock::Date => format!("{y:04}-{m:02}-{d:02}"),
        _ => format!("{y:04}-{m:02}-{d:02}T{time}"),
    })
}

/// The day of an ISO date `YYYY-MM-DD` (year 1 to 9999), from 1970-01-01;
/// none for a text that names no day of the calendar.
pub(crate) fn iso_day(date: &str) -> Option<i64> {
    let b = date.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| {
        let t = date.get(r)?;
        t.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| t.parse::<i64>().ok())?
    };
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let days = days_from_civil(y, m, d);
    (y >= 1 && civil_from_days(days) == (y, m, d)).then_some(days)
}

/// Excel's day count of an ISO date `YYYY-MM-DD` in the 1900 system, from
/// 1 March 1900 (before it Excel's count is a day off); none for another text.
pub(crate) fn serial(date: &str) -> Option<i64> {
    let serial = iso_day(date)? - days_from_civil(1899, 12, 30);
    (serial >= 61).then_some(serial)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_formats_are_told_apart() {
        for (fmt, want) in [
            ("General", None),
            ("0.00", None),
            ("#,##0.00 [$TL-41F]", None),
            ("0.00\" m²\"", None),
            ("dd.mm.yyyy", Some(Clock::Date)),
            ("mmm-yy", Some(Clock::Date)),
            ("[$-41F]d mmmm yyyy;@", Some(Clock::Date)),
            ("h:mm", Some(Clock::Time)),
            ("mm:ss", Some(Clock::Time)),
            ("hh:mm:ss AM/PM", Some(Clock::Time)),
            ("dd.mm.yyyy hh:mm", Some(Clock::DateTime)),
            ("[h]:mm:ss", None),
            ("[MM]:ss", None),
            ("\\d0", None),
            ("_d0.0", None),
            ("[Red]0.00", None),
        ] {
            assert_eq!(clock(fmt), want, "{fmt}");
        }
        assert_eq!(builtin(14).and_then(clock), Some(Clock::Date));
        assert_eq!(builtin(22).and_then(clock), Some(Clock::DateTime));
        assert_eq!(builtin(46).and_then(clock), None);
        assert_eq!(builtin(2), None);
    }

    #[test]
    fn day_counts_are_excels() {
        let t = |v: &str, c: Clock| text(v, c, false);
        assert_eq!(t("1", Clock::Date).as_deref(), Some("1900-01-01"));
        assert_eq!(t("59", Clock::Date).as_deref(), Some("1900-02-28"));
        assert_eq!(t("60", Clock::Date).as_deref(), Some("1900-02-28"));
        assert_eq!(t("61", Clock::Date).as_deref(), Some("1900-03-01"));
        assert_eq!(t("45566", Clock::Date).as_deref(), Some("2024-10-01"));
        assert_eq!(
            t("45566.75", Clock::DateTime).as_deref(),
            Some("2024-10-01T18:00:00")
        );
        assert_eq!(
            t("45566.999999", Clock::DateTime).as_deref(),
            Some("2024-10-02T00:00:00")
        );
        assert_eq!(
            t("45566.999999", Clock::Date).as_deref(),
            Some("2024-10-01")
        );
        assert_eq!(t("45566.5", Clock::Time).as_deref(), Some("12:00:00"));
        assert_eq!(t("0.25", Clock::Date).as_deref(), Some("06:00:00"));
        assert_eq!(t("2958465", Clock::Date).as_deref(), Some("9999-12-31"));
        assert_eq!(t("-1", Clock::Date), None);
        assert_eq!(t("3000000", Clock::Date), None);
        assert_eq!(t("abc", Clock::Date), None);
        assert_eq!(text("0", Clock::Date, true).as_deref(), Some("00:00:00"));
        assert_eq!(text("1", Clock::Date, true).as_deref(), Some("1904-01-02"));
        assert_eq!(
            text("44104", Clock::Date, true).as_deref(),
            Some("2024-10-01")
        );
    }

    #[test]
    fn serials_go_back_to_dates() {
        for d in ["1900-03-01", "2000-02-29", "2024-10-01", "9999-12-31"] {
            let s = serial(d).expect("a date");
            assert_eq!(text(&s.to_string(), Clock::Date, false).as_deref(), Some(d));
        }
        assert_eq!(serial("1900-02-28"), None);
        assert_eq!(serial("2023-02-29"), None);
        assert_eq!(serial("2024-1-01"), None);
    }
}
