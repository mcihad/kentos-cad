//! Zamansal katmanların zamanı (docs/adr/0210 §3–§5): an attribute's text
//! read as a moment, a moment written and shown, an object's time from its
//! start and end texts, whether it shows in the time slider's window, and the
//! slider's steps (the calendar's months and years among them).
//!
//! A moment is a whole number of milliseconds since 1970-01-01 00:00 (the
//! proleptic Gregorian calendar backwards), held in an `f64`: every moment of
//! the years 1–9999 is an integer far below 2^53, so it is exact and compares
//! exactly. The calendar is worked in integers (`i64`), never in floats.

use crate::jsmath::{js_max, js_min};

/// Milliseconds in a second, a minute, an hour, a day and a week.
const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;

/// The most positions the slider takes (§5).
pub const MAX_POSITIONS: i64 = 100_000;

/// An attribute's text read as a moment (§3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Read {
    /// No text (blanks only).
    Empty,
    /// A moment in milliseconds.
    Moment(f64),
    /// Text that is not a moment.
    Unreadable,
}

impl Read {
    pub fn moment(self) -> Option<f64> {
        match self {
            Read::Moment(t) => Some(t),
            _ => None,
        }
    }
}

fn leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

/// The days of month `m` (1–12) of year `y`.
pub fn days_in(y: i64, m: i64) -> i64 {
    match m {
        2 if leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to the civil date (any year; month 1–12, day 1–31).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date of a day counted from 1970-01-01.
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// A moment's parts: year, month, day and the milliseconds into the day.
fn parts(t: i64) -> (i64, i64, i64, i64) {
    let (y, m, d) = civil_from_days(t.div_euclid(DAY));
    (y, m, d, t.rem_euclid(DAY))
}

/// A whole number of 1–2 or exactly `n` ASCII digits.
fn number(s: &str, lo: usize, hi: usize) -> Option<i64> {
    ((lo..=hi).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse::<i64>().ok())
        .flatten()
}

/// `YYYY-AA-GG` or `GG.AA.YYYY` (day and month one or two digits): a day of the calendar, years 1–9999.
fn date(s: &str) -> Option<(i64, i64, i64)> {
    let b = s.as_bytes();
    let (y, m, d) = if s.len() == 10 && b[4] == b'-' && b[7] == b'-' {
        (
            number(&s[0..4], 4, 4)?,
            number(&s[5..7], 2, 2)?,
            number(&s[8..10], 2, 2)?,
        )
    } else {
        let mut it = s.split('.');
        let (d, m, y) = (it.next()?, it.next()?, it.next()?);
        if it.next().is_some() {
            return None;
        }
        (number(y, 4, 4)?, number(m, 1, 2)?, number(d, 1, 2)?)
    };
    ((1..=9999).contains(&y) && (1..=12).contains(&m) && (1..=days_in(y, m)).contains(&d))
        .then_some((y, m, d))
}

/// A zone's offset in milliseconds: `Z`, `+SS:DD`, `-SS:DD`, `+SSDD`, `-SSDD`, `+SS`, `-SS`.
fn zone(s: &str) -> Option<i64> {
    if s == "Z" {
        return Some(0);
    }
    let sign = match s.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let rest = &s[1..];
    let (h, m) = match rest.len() {
        2 => (number(rest, 2, 2)?, 0),
        4 => (number(&rest[0..2], 2, 2)?, number(&rest[2..4], 2, 2)?),
        5 if rest.as_bytes()[2] == b':' => (number(&rest[0..2], 2, 2)?, number(&rest[3..5], 2, 2)?),
        _ => return None,
    };
    (h <= 23 && m <= 59).then_some(sign * (h * HOUR + m * MINUTE))
}

/// `SS:DD`, `SS:DD:ss` or `SS:DD:ss.f…` (1–9 digits; below a millisecond dropped) and a zone, in milliseconds into the day and the zone's offset.
fn clock(s: &str) -> Option<(i64, i64)> {
    // The zone starts at a `Z`, or at a sign after the minutes.
    let cut = s
        .char_indices()
        .skip(5)
        .find(|&(_, c)| c == 'Z' || c == '+' || c == '-')
        .map_or(s.len(), |(i, _)| i);
    let (time, offset) = s.split_at(cut);
    let offset = if offset.is_empty() { 0 } else { zone(offset)? };
    let (hm, rest) = time.split_at(time.len().min(5));
    let b = hm.as_bytes();
    if hm.len() != 5 || b[2] != b':' {
        return None;
    }
    let (h, mi) = (number(&hm[0..2], 2, 2)?, number(&hm[3..5], 2, 2)?);
    let (sec, ms) = if rest.is_empty() {
        (0, 0)
    } else {
        let rest = rest.strip_prefix(':')?;
        let (whole, fraction) = match rest.split_once('.') {
            Some((w, f)) => (w, Some(f)),
            None => (rest, None),
        };
        let sec = number(whole, 2, 2)?;
        let ms = match fraction {
            None => 0,
            Some(f) => {
                if !(1..=9).contains(&f.len()) || !f.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                let three: String = f.chars().chain("000".chars()).take(3).collect();
                three.parse::<i64>().ok()?
            }
        };
        (sec, ms)
    };
    (h <= 23 && mi <= 59 && sec <= 59)
        .then_some((h * HOUR + mi * MINUTE + sec * SECOND + ms, offset))
}

/// An attribute's text as a moment (§3): blanks trimmed; a date, or a date
/// and a time (after `T` or one space) with an optional zone.
pub fn read(text: &str) -> Read {
    let s = text.trim_matches(|c: char| c == ' ' || c == '\t');
    if s.is_empty() {
        return Read::Empty;
    }
    // Every form is ASCII; the slicing below needs it (a Turkish letter must not split a character).
    if !s.is_ascii() {
        return Read::Unreadable;
    }
    let (day, time) = match s.find(['T', ' ']) {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    let Some((y, m, d)) = date(day) else {
        return Read::Unreadable;
    };
    let (into, offset) = match time {
        None => (0, 0),
        Some(t) => match clock(t) {
            Some(c) => c,
            None => return Read::Unreadable,
        },
    };
    Read::Moment((days_from_civil(y, m, d) * DAY + into - offset) as f64)
}

/// A moment written into an attribute (§3): `YYYY-AA-GG` at midnight (or
/// always when `date_only`), else `YYYY-AA-GGTSS:DD:ss` (and `.mmm` when it has milliseconds).
pub fn write(t: f64, date_only: bool) -> String {
    let (y, m, d, into) = parts(t as i64);
    let day = format!("{y:04}-{m:02}-{d:02}");
    if date_only || into == 0 {
        return day;
    }
    let (h, mi, s, ms) = (
        into / HOUR,
        into % HOUR / MINUTE,
        into % MINUTE / SECOND,
        into % SECOND,
    );
    if ms == 0 {
        format!("{day}T{h:02}:{mi:02}:{s:02}")
    } else {
        format!("{day}T{h:02}:{mi:02}:{s:02}.{ms:03}")
    }
}

/// A step's unit (§5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl Unit {
    pub const ALL: [Unit; 7] = [
        Unit::Second,
        Unit::Minute,
        Unit::Hour,
        Unit::Day,
        Unit::Week,
        Unit::Month,
        Unit::Year,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Unit::Second => "second",
            Unit::Minute => "minute",
            Unit::Hour => "hour",
            Unit::Day => "day",
            Unit::Week => "week",
            Unit::Month => "month",
            Unit::Year => "year",
        }
    }

    pub fn from_name(s: &str) -> Option<Unit> {
        Unit::ALL.into_iter().find(|u| u.name() == s)
    }

    /// The unit's Turkish word (“1 ay”, “2 yıl”).
    pub fn word(self) -> &'static str {
        match self {
            Unit::Second => "saniye",
            Unit::Minute => "dakika",
            Unit::Hour => "saat",
            Unit::Day => "gün",
            Unit::Week => "hafta",
            Unit::Month => "ay",
            Unit::Year => "yıl",
        }
    }

    /// Its milliseconds, for a unit of fixed length.
    fn fixed(self) -> Option<i64> {
        match self {
            Unit::Second => Some(SECOND),
            Unit::Minute => Some(MINUTE),
            Unit::Hour => Some(HOUR),
            Unit::Day => Some(DAY),
            Unit::Week => Some(WEEK),
            Unit::Month | Unit::Year => None,
        }
    }
}

/// A moment shown (§3): `GG.AA.YYYY`, with ` SS:DD` for a step under a day and ` SS:DD:ss` for seconds.
pub fn show(t: f64, unit: Unit) -> String {
    let (y, m, d, into) = parts(t as i64);
    let day = format!("{d:02}.{m:02}.{y:04}");
    let (h, mi, s) = (into / HOUR, into % HOUR / MINUTE, into % MINUTE / SECOND);
    match unit {
        Unit::Second => format!("{day} {h:02}:{mi:02}:{s:02}"),
        Unit::Minute | Unit::Hour => format!("{day} {h:02}:{mi:02}"),
        _ => day,
    }
}

/// A step: `n` (1–999) units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub n: i64,
    pub unit: Unit,
}

impl Step {
    pub fn new(n: i64, unit: Unit) -> Option<Step> {
        (1..=999).contains(&n).then_some(Step { n, unit })
    }
}

/// A moment rounded down to its unit (§5's anchor).
pub fn floor_to(t: f64, unit: Unit) -> f64 {
    let t = t as i64;
    let (y, m, _, into) = parts(t);
    let day = t - into;
    let r = match unit {
        Unit::Year => days_from_civil(y, 1, 1) * DAY,
        Unit::Month => days_from_civil(y, m, 1) * DAY,
        Unit::Week => {
            // Monday 00:00: 1970-01-01 was a Thursday.
            let days = day.div_euclid(DAY);
            (days - (days + 3).rem_euclid(7)) * DAY
        }
        Unit::Day => day,
        Unit::Hour => t - into.rem_euclid(HOUR),
        Unit::Minute => t - into.rem_euclid(MINUTE),
        Unit::Second => t - into.rem_euclid(SECOND),
    };
    r as f64
}

/// The `k`-th position from `anchor` (§5): `k` steps added at once, months
/// and years by the calendar (a day past the month's end is its last day).
pub fn position(anchor: f64, step: Step, k: i64) -> f64 {
    let t = anchor as i64;
    match step.unit.fixed() {
        Some(ms) => (t + k * step.n * ms) as f64,
        None => {
            let (y, m, d, into) = parts(t);
            let months = if step.unit == Unit::Year { 12 } else { 1 } * step.n * k;
            let total = y * 12 + (m - 1) + months;
            let (y2, m2) = (total.div_euclid(12), total.rem_euclid(12) + 1);
            let d2 = d.min(days_in(y2, m2));
            (days_from_civil(y2, m2, d2) * DAY + into) as f64
        }
    }
}

/// The slider's positions over `extent` (§5): the anchor and the last index
/// K (the least k whose position is not before the extent's end). None past
/// [`MAX_POSITIONS`].
pub fn positions(extent: (f64, f64), step: Step) -> Option<(f64, i64)> {
    let anchor = floor_to(extent.0, step.unit);
    let end = extent.1;
    let mut k = match step.unit.fixed() {
        Some(ms) => {
            let span = (end as i64 - anchor as i64).max(0);
            let per = step.n * ms;
            (span + per - 1) / per
        }
        None => {
            let (ya, ma, ..) = parts(anchor as i64);
            let (yb, mb, ..) = parts(end as i64);
            let per = if step.unit == Unit::Year { 12 } else { 1 } * step.n;
            ((yb * 12 + mb) - (ya * 12 + ma)).max(0) / per
        }
    };
    // The estimate is a step off at most: settle on the least k that reaches the end.
    while k > 0 && position(anchor, step, k - 1) >= end {
        k -= 1;
    }
    while position(anchor, step, k) < end {
        k += 1;
        if k > MAX_POSITIONS {
            return None;
        }
    }
    (k <= MAX_POSITIONS).then_some((anchor, k))
}

/// The step a slider opens with (§5): the first of year, month, day, hour,
/// minute and second that cuts the extent into at least 5 steps; else a second.
pub fn auto_step(extent: (f64, f64)) -> Step {
    for unit in [
        Unit::Year,
        Unit::Month,
        Unit::Day,
        Unit::Hour,
        Unit::Minute,
        Unit::Second,
    ] {
        let step = Step { n: 1, unit };
        if positions(extent, step).is_some_and(|(_, k)| k >= 5) {
            return step;
        }
    }
    Step {
        n: 1,
        unit: Unit::Second,
    }
}

/// How an object's time shows (§4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// [s, e).
    Range,
    /// The moment s.
    Instant,
    /// From s on, whatever its end (Birikimli).
    Cumulative,
}

impl Mode {
    pub fn code(self) -> u8 {
        match self {
            Mode::Range => 0,
            Mode::Instant => 1,
            Mode::Cumulative => 2,
        }
    }

    pub fn from_code(c: u8) -> Option<Mode> {
        match c {
            0 => Some(Mode::Range),
            1 => Some(Mode::Instant),
            2 => Some(Mode::Cumulative),
            _ => None,
        }
    }
}

/// An object's time: its start, its end (+∞ open, = start for a moment) and how it shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Time {
    pub s: f64,
    pub e: f64,
    pub mode: Mode,
}

/// A layer's rule (§2): it has an end field, it is cumulative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rule {
    pub ranged: bool,
    pub cumulative: bool,
}

/// An object's time from its start and end texts (§4); `None` timeless. The
/// end is read only for a ranged layer.
pub fn object_time(rule: Rule, start: Read, end: Read) -> Option<Time> {
    let mode = if rule.cumulative {
        Mode::Cumulative
    } else if rule.ranged {
        Mode::Range
    } else {
        Mode::Instant
    };
    if rule.ranged {
        let (s, e) = (start.moment(), end.moment());
        if s.is_none() && e.is_none() {
            return None;
        }
        Some(Time {
            s: s.unwrap_or(f64::NEG_INFINITY),
            e: e.unwrap_or(f64::INFINITY),
            mode,
        })
    } else {
        let s = start.moment()?;
        Some(Time { s, e: s, mode })
    }
}

/// The slider's window (§4): a moment, or [a, b).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Window {
    Instant(f64),
    Range(f64, f64),
}

impl Window {
    /// The window at position `k` (§5): its moment, or up to the next position.
    pub fn at(anchor: f64, step: Step, k: i64, ranged: bool) -> Window {
        let a = position(anchor, step, k);
        if ranged {
            Window::Range(a, position(anchor, step, k + 1))
        } else {
            Window::Instant(a)
        }
    }
}

/// Whether an object with time `t` shows in window `w` (§4's table).
pub fn shows(t: &Time, w: &Window) -> bool {
    match (t.mode, *w) {
        (Mode::Cumulative, Window::Instant(a)) => t.s <= a,
        (Mode::Cumulative, Window::Range(_, b)) => t.s < b,
        (Mode::Range, Window::Instant(a)) => t.s <= a && a < t.e,
        (Mode::Range, Window::Range(a, b)) => t.s < b && t.e > a,
        (Mode::Instant, Window::Instant(a)) => t.s == a,
        (Mode::Instant, Window::Range(a, b)) => a <= t.s && t.s < b,
    }
}

/// The least and the greatest of the finite starts and ends (§5's extent).
pub fn extent<'a>(times: impl IntoIterator<Item = &'a Time>) -> Option<(f64, f64)> {
    let mut out: Option<(f64, f64)> = None;
    for t in times {
        for v in [t.s, t.e] {
            if v.is_finite() {
                out = Some(match out {
                    None => (v, v),
                    Some((lo, hi)) => (js_min(lo, v), js_max(hi, v)),
                });
            }
        }
    }
    out
}

/// What a layer's values give (Zaman ayarları' preview, §10): objects with a
/// time, timeless ones, unreadable values and the extent.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Summary {
    pub timed: usize,
    pub timeless: usize,
    pub unreadable: usize,
    pub extent: Option<(f64, f64)>,
}

/// The times of a layer's objects from their start and end texts, and what they give.
pub fn layer_times<'a>(
    rule: Rule,
    values: impl IntoIterator<Item = (Option<&'a str>, Option<&'a str>)>,
) -> (Vec<Option<Time>>, Summary) {
    let mut out = Vec::new();
    let mut sum = Summary::default();
    for (start, end) in values {
        let s = start.map_or(Read::Empty, read);
        let e = if rule.ranged {
            end.map_or(Read::Empty, read)
        } else {
            Read::Empty
        };
        sum.unreadable += usize::from(s == Read::Unreadable) + usize::from(e == Read::Unreadable);
        let t = object_time(rule, s, e);
        match t {
            Some(_) => sum.timed += 1,
            None => sum.timeless += 1,
        }
        out.push(t);
    }
    sum.extent = extent(out.iter().flatten());
    (out, sum)
}

fn unit_named(name: &str) -> Result<Unit, String> {
    Unit::from_name(name).ok_or_else(|| format!("Zaman birimi “{name}” bilinmiyor."))
}

fn step_of(n: f64, unit: &str) -> Result<Step, String> {
    let unit = unit_named(unit)?;
    Step::new(n as i64, unit).ok_or_else(|| "Adım 1 ile 999 birim arasında olmalı.".to_owned())
}

/// The web's calls (`apps/web/src/wasm/time.ts`); the objects' times of a
/// layer cross in one typed call instead (`timeLayer`, geometry-wasm).
pub static OPS: &[crate::api::Op] = &[
    // [0 empty | 1 moment | 2 unreadable, the moment or NaN].
    crate::op!("timeRead", |text: String| match read(&text) {
        Read::Empty => vec![0.0, f64::NAN],
        Read::Moment(t) => vec![1.0, t],
        Read::Unreadable => vec![2.0, f64::NAN],
    }),
    crate::op!("timeWrite", |t: f64, date_only: bool| write(t, date_only)),
    crate::op!("timeShow", |t: f64, unit: String| unit_named(&unit)
        .map(|u| show(t, u))),
    crate::op!("timeFloor", |t: f64, unit: String| unit_named(&unit)
        .map(|u| floor_to(t, u))),
    // [n, the unit's index in `Unit::ALL`].
    crate::op!("timeAutoStep", |start: f64, end: f64| {
        let step = auto_step((start, end));
        let unit = Unit::ALL.iter().position(|u| *u == step.unit).unwrap_or(0);
        vec![step.n as f64, unit as f64]
    }),
    // [anchor, K], or null past `MAX_POSITIONS`.
    crate::op!(
        "timePositions",
        |start: f64, end: f64, n: f64, unit: String| {
            step_of(n, &unit)
                .map(|step| positions((start, end), step).map(|(a, k)| vec![a, k as f64]))
        }
    ),
    crate::op!("timePosition", |anchor: f64,
                                n: f64,
                                unit: String,
                                k: f64| {
        step_of(n, &unit).map(|step| position(anchor, step, k as i64))
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(s: &str) -> f64 {
        read(s).moment().unwrap()
    }

    #[test]
    fn dates_and_times_read_as_the_calendar_says() {
        assert_eq!(ms("1970-01-01"), 0.0);
        assert_eq!(ms("02.01.1970"), DAY as f64);
        assert_eq!(ms("1969-12-31T23:59:59.999"), -1.0);
        assert_eq!(
            ms("2024-02-29 12:30"),
            ms("2024-02-29") + (12 * HOUR + 30 * MINUTE) as f64
        );
        assert_eq!(ms("2024-02-29T12:30:00+03:00"), ms("2024-02-29T09:30"));
        assert_eq!(ms("2024-02-29T12:30Z"), ms("2024-02-29T12:30"));
        assert_eq!(ms("0001-01-01"), -62_135_596_800_000.0);
        assert_eq!(read("2023-02-29"), Read::Unreadable);
        assert_eq!(read("  "), Read::Empty);
        assert_eq!(read("2024-01-01T24:00"), Read::Unreadable);
        assert_eq!(read("2024-01-01+03:00"), Read::Unreadable);
    }

    #[test]
    fn written_and_shown_moments_read_back() {
        for s in [
            "2024-02-29",
            "1999-12-31T23:59:59",
            "0001-01-01T00:00:00.007",
        ] {
            assert_eq!(write(ms(s), false), s);
        }
        assert_eq!(write(ms("2024-02-29T08:00"), true), "2024-02-29");
        assert_eq!(
            show(ms("2024-02-29T08:05:09"), Unit::Second),
            "29.02.2024 08:05:09"
        );
    }

    #[test]
    fn months_keep_their_anchor_and_clamp_the_day() {
        let a = ms("2024-01-31");
        let step = Step {
            n: 1,
            unit: Unit::Month,
        };
        assert_eq!(write(position(a, step, 1), false), "2024-02-29");
        assert_eq!(write(position(a, step, 2), false), "2024-03-31");
        assert_eq!(
            write(
                position(
                    a,
                    Step {
                        n: 1,
                        unit: Unit::Year
                    },
                    -1
                ),
                false
            ),
            "2023-01-31"
        );
    }

    #[test]
    fn positions_reach_the_end_and_the_window_shows_by_the_table() {
        let (anchor, k) = positions(
            (ms("2010-03-05"), ms("2015-07-01")),
            Step {
                n: 1,
                unit: Unit::Year,
            },
        )
        .unwrap();
        assert_eq!(write(anchor, false), "2010-01-01");
        assert_eq!(k, 6);
        assert_eq!(
            auto_step((ms("2010-03-05"), ms("2015-07-01"))).unit,
            Unit::Year
        );
        let t = Time {
            s: ms("2012-01-01"),
            e: ms("2014-01-01"),
            mode: Mode::Range,
        };
        assert!(shows(&t, &Window::Instant(ms("2012-01-01"))));
        assert!(!shows(&t, &Window::Instant(ms("2014-01-01"))));
        assert!(shows(
            &t,
            &Window::Range(ms("2013-12-31"), ms("2014-06-01"))
        ));
    }
}
