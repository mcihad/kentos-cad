//! Araç profilleri ve tarihli kapanmalar (docs/adr/0567): what a vehicle may
//! travel and how fast, and which edges are closed on a day. The graph is
//! built once; a question naming a profile or a day varies its costs, not its
//! pieces:
//!
//! 1. An edge is closed to a profile where one of its limit fields reads a
//!    number below the profile's value (an empty field is no limit) or one of
//!    its forbidding fields holds one of the profile's values (Turkish case
//!    aside).
//! 2. An edge is closed on a day `d` where its first day `a` and last day `b`
//!    (`YYYY-MM-DD`, both in) hold `a ≤ d ≤ b`; an empty first day is from
//!    always, an empty last day until ever, both empty not closed.
//! 3. A profile's top speed caps every speed cost's speed (a time profile's
//!    too): a piece's minutes are the larger of its own and its length at the
//!    top speed.
//! 4. A profile going both ways travels a one-way edge against its way at the
//!    same cost; an edge closed by the direction field or the closed edges'
//!    expression stays closed.
//! 5. A closure's weekdays and hours (docs/adr/0568): a day closes an edge only
//!    when its weekday is one its weekday field names (empty: every day); with
//!    hours the edge is closed only in them, a window that ends before it
//!    starts (`22:00-06:00`) going on into the next day. The hours are read
//!    only with a time: a speed cost's clock goes at no speed in them (a
//!    vehicle waits or goes round), another cost passes none closed at the
//!    question's moment. The week is the question's day and the five after it,
//!    and the day before it (a reverse search's past).
//!
//! A value that cannot be read (a limit not a number, a day not a date) closes
//! nothing and is said. The independent reference is
//! `scripts/fixtures/network_access_cases.py`.

use super::rules::{caseless, number};
use crate::api::json::Json;

/// A vehicle profile as the rules read it.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileRule {
    /// Its name as the definition writes it, and folded.
    pub name: String,
    pub key: String,
    /// Each limit: the fact (an index into `AccessRules::facts`) and the vehicle's value.
    pub limits: Vec<(usize, f64)>,
    /// Each forbidding field: the fact and its values, folded.
    pub forbidden: Vec<(usize, Vec<String>)>,
    /// km/h.
    pub top_speed: Option<f64>,
    pub both_ways: bool,
}

/// The definition's profiles and closures: the edge fields they read (`facts`, each once, in the order the
/// profiles name them, then the closures' first and last day) and the rules on them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AccessRules {
    pub facts: Vec<String>,
    pub profiles: Vec<ProfileRule>,
    /// The closures' facts.
    pub closures: Option<ClosureRule>,
}

/// A closure's facts: its first and last day, its weekdays and its hours (docs/adr/0567 §2, 0568 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosureRule {
    pub days: [usize; 2],
    pub weekdays: Option<usize>,
    pub hours: Option<usize>,
}

/// What a question asks of the network besides its places: the vehicle (an index into the rules' profiles), the
/// day (days since 1970-01-01) and, when the closures have hours, the time of day it sets off at (minutes; a reverse
/// question's arrival). None of them: the network as defined.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Access {
    pub profile: Option<usize>,
    pub day: Option<i64>,
    pub time: Option<f64>,
}

impl Access {
    pub fn is_none(&self) -> bool {
        self.profile.is_none() && self.day.is_none()
    }
}

/// Minutes in a week: a timed closure's clock repeats after it (docs/adr/0568 §3).
pub const WEEK: f64 = 7.0 * 1440.0;

/// What varying the graph closed: how many edges the profile closed, how many the day closed (an edge both once, the
/// profile's), how many are closed in hours of the week (docs/adr/0568) and whether they went unread for want of a
/// time, and the edges whose values could not be read (their objects, once each, in the graph's order).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Varied {
    pub by_profile: usize,
    pub by_date: usize,
    pub by_hours: usize,
    pub hours_unasked: bool,
    pub unread: Vec<f64>,
}

/// Why an edge is closed to a question: its vehicle, its day, or hours of its week (the windows given apart).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closing {
    Open,
    Profile,
    Date,
    Hours,
}

impl AccessRules {
    /// The rules of a definition's `profiles` and `closures` (the contract's JSON); none of either, no rules.
    pub fn from_json(def: &Json) -> Result<AccessRules, String> {
        let mut rules = AccessRules::default();
        let fact = |facts: &mut Vec<String>, name: &str| -> usize {
            match facts.iter().position(|f| f == name) {
                Some(i) => i,
                None => {
                    facts.push(name.to_owned());
                    facts.len() - 1
                }
            }
        };
        let text = |v: &Json, what: &str| match v {
            Json::Str(s) => Ok(s.clone()),
            _ => Err(format!("ağın araç profilinin {what} metin olmalı")),
        };
        match def.get("profiles") {
            Json::Null => {}
            Json::Arr(list) => {
                for p in list {
                    let name = text(p.get("name"), "adı")?;
                    let mut limits = Vec::new();
                    if let Json::Arr(ls) = p.get("limits") {
                        for l in ls {
                            let field = text(l.get("field"), "sınır alanı")?;
                            let value = match l.get("value") {
                                Json::Num(v) if v.is_finite() && *v > 0.0 => *v,
                                _ => return Err("ağın araç profilinin sınırı sıfırdan büyük sayı olmalı".into()),
                            };
                            limits.push((fact(&mut rules.facts, &field), value));
                        }
                    }
                    let mut forbidden = Vec::new();
                    if let Json::Arr(fs) = p.get("forbidden") {
                        for f in fs {
                            let field = text(f.get("field"), "yasak alanı")?;
                            let values = match f.get("values") {
                                Json::Arr(vs) => vs
                                    .iter()
                                    .map(|v| text(v, "yasak değeri").map(|s| caseless(&s)))
                                    .collect::<Result<Vec<_>, _>>()?,
                                _ => return Err("ağın araç profilinin yasak değerleri liste olmalı".into()),
                            };
                            forbidden.push((fact(&mut rules.facts, &field), values));
                        }
                    }
                    let top_speed = match p.get("topSpeed") {
                        Json::Null => None,
                        Json::Num(v) if v.is_finite() && *v > 0.0 => Some(*v),
                        _ => return Err("ağın araç profilinin en yüksek hızı sıfırdan büyük sayı olmalı".into()),
                    };
                    let both_ways = matches!(p.get("bothWays"), Json::Bool(true));
                    rules.profiles.push(ProfileRule {
                        key: caseless(&name),
                        name,
                        limits,
                        forbidden,
                        top_speed,
                        both_ways,
                    });
                }
            }
            _ => return Err("ağın araç profilleri liste olmalı".into()),
        }
        match def.get("closures") {
            Json::Null => {}
            c => {
                let from = text(c.get("from"), "kapanma alanı")?;
                let to = text(c.get("to"), "kapanma alanı")?;
                let days = [fact(&mut rules.facts, &from), fact(&mut rules.facts, &to)];
                let mut optional = |key: &str| match c.get(key) {
                    Json::Null => Ok(None),
                    v => text(v, "kapanma alanı").map(|f| Some(fact(&mut rules.facts, &f))),
                };
                let weekdays = optional("days")?;
                let hours = optional("hours")?;
                rules.closures = Some(ClosureRule { days, weekdays, hours });
            }
        }
        Ok(rules)
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty() && self.closures.is_none()
    }

    /// The profile named `name` (Turkish case aside).
    pub fn profile(&self, name: &str) -> Option<usize> {
        let key = caseless(name);
        self.profiles.iter().position(|p| p.key == key)
    }

    /// Whether the closures have hours: a question's time then varies the network (docs/adr/0568).
    pub fn timed(&self) -> bool {
        self.closures.is_some_and(|c| c.hours.is_some())
    }

    /// Whether `access` closes an edge whose facts are `facts` (`facts[i]` the value of `self.facts[i]`); `unread` set
    /// when a value it reads cannot be read. Closed in hours: `windows` gets them, minutes of the access's week (its
    /// day from 0, the day before from 6 days on), each `[from, to)` within `[0, WEEK]`.
    pub fn closing(
        &self,
        access: &Access,
        facts: &[Option<String>],
        unread: &mut bool,
        windows: &mut Vec<(f64, f64)>,
    ) -> Closing {
        let fact = |i: usize| {
            facts
                .get(i)
                .and_then(Option::as_deref)
                .map(str::trim)
                .filter(|t| !t.is_empty())
        };
        if let Some(p) = access.profile.and_then(|i| self.profiles.get(i)) {
            for &(i, value) in &p.limits {
                if let Some(t) = fact(i) {
                    match number(t) {
                        Some(edge) if edge < value => return Closing::Profile,
                        Some(_) => {}
                        None => *unread = true,
                    }
                }
            }
            for (i, values) in &p.forbidden {
                if let Some(t) = fact(*i)
                    && values.contains(&caseless(t))
                {
                    return Closing::Profile;
                }
            }
        }
        let (Some(day), Some(rule)) = (access.day, self.closures) else {
            return Closing::Open;
        };
        // Each value read in turn; one not read closes nothing.
        let mut read = |i: Option<usize>, how: &dyn Fn(&str) -> Option<Reading>| match i.and_then(fact) {
            None => Some(None),
            Some(t) => match how(t) {
                Some(r) => Some(Some(r)),
                None => {
                    *unread = true;
                    None
                }
            },
        };
        let as_day = |t: &str| day_of(t).map(Reading::Day);
        let as_weekdays = |t: &str| weekdays_of(t).map(Reading::Weekdays);
        let as_hours = |t: &str| hours_of(t).map(Reading::Hours);
        let [a, b] = rule.days;
        let Some(from) = read(Some(a), &as_day) else { return Closing::Open };
        let Some(to) = read(Some(b), &as_day) else { return Closing::Open };
        let Some(weekdays) = read(rule.weekdays, &as_weekdays) else { return Closing::Open };
        let Some(hours) = read(rule.hours, &as_hours) else { return Closing::Open };
        if from.is_none() && to.is_none() && weekdays.is_none() && hours.is_none() {
            return Closing::Open;
        }
        let (from, to) = (from.map(Reading::day), to.map(Reading::day));
        let mask = weekdays.map_or(ALL_WEEKDAYS, Reading::weekdays);
        let closed_on = |d: i64| {
            from.is_none_or(|f| f <= d) && to.is_none_or(|t| d <= t) && mask & (1 << weekday_of(d)) != 0
        };
        let Some(hours) = hours.map(Reading::hours) else {
            return if closed_on(day) { Closing::Date } else { Closing::Open };
        };
        let first = windows.len();
        for slot in 0..7 {
            let d = if slot < 6 { day + slot } else { day - 1 };
            if !closed_on(d) {
                continue;
            }
            let base = slot as f64 * 1440.0;
            for &(s, e) in &hours {
                let (s, e) = (base + s, base + if e > s { e } else { e + 1440.0 });
                if e > WEEK {
                    windows.push((s, WEEK));
                    windows.push((0.0, e - WEEK));
                } else {
                    windows.push((s, e));
                }
            }
        }
        if windows.len() > first { Closing::Hours } else { Closing::Open }
    }
}

/// A closure's value as read.
enum Reading {
    Day(i64),
    Weekdays(u8),
    Hours(Vec<(f64, f64)>),
}

impl Reading {
    fn day(self) -> i64 {
        match self {
            Reading::Day(d) => d,
            _ => unreachable!("read as a day"),
        }
    }

    fn weekdays(self) -> u8 {
        match self {
            Reading::Weekdays(m) => m,
            _ => unreachable!("read as weekdays"),
        }
    }

    fn hours(self) -> Vec<(f64, f64)> {
        match self {
            Reading::Hours(h) => h,
            _ => unreachable!("read as hours"),
        }
    }
}

/// Every weekday, Monday the lowest bit.
pub const ALL_WEEKDAYS: u8 = 0x7f;

/// A day's weekday (days since 1970-01-01, a Thursday): 0 Monday … 6 Sunday.
pub fn weekday_of(day: i64) -> u32 {
    (day + 3).rem_euclid(7) as u32
}

/// The weekdays a text names (docs/adr/0568 §1): days or ranges (`Cum-Pzt` goes round the week end) apart by commas
/// or semicolons, each a Turkish name, its short form or 1 (Monday) … 7 (Sunday), case aside; Monday the lowest bit.
pub fn weekdays_of(text: &str) -> Option<u8> {
    let mut mask = 0u8;
    for part in text.split([',', ';']) {
        let (a, b) = match part.split_once(['-', '–']) {
            Some((a, b)) => (weekday_named(a)?, weekday_named(b)?),
            None => {
                let d = weekday_named(part)?;
                (d, d)
            }
        };
        let mut d = a;
        loop {
            mask |= 1 << d;
            if d == b {
                break;
            }
            d = (d + 1) % 7;
        }
    }
    Some(mask)
}

fn weekday_named(name: &str) -> Option<u32> {
    let names: [&[&str]; 7] = [
        &["1", "pazartesi", "pzt", "pts"],
        &["2", "salı", "sali", "sal"],
        &["3", "çarşamba", "carsamba", "çar", "car", "çrş"],
        &["4", "perşembe", "persembe", "per", "prş"],
        &["5", "cuma", "cum"],
        &["6", "cumartesi", "cmt", "cts"],
        &["7", "pazar", "paz", "pzr"],
    ];
    let key = caseless(name);
    names.iter().position(|n| n.contains(&key.as_str())).map(|d| d as u32)
}

/// The hours a text names (docs/adr/0568 §1): windows `H:MM-H:MM` apart by commas or semicolons, minutes of the day
/// each `[from, to)`; one that ends at or before it starts goes on into the next day (`24:00` is the day's end). A
/// window that starts where it ends is not read.
pub fn hours_of(text: &str) -> Option<Vec<(f64, f64)>> {
    // `H:MM` or `HH:MM`, 00:00–23:59 and 24:00: minutes of the day.
    let clock = |t: &str| -> Option<u32> {
        let (h, m) = t.trim().split_once(':')?;
        let digits = |s: &str, most: usize| {
            (!s.is_empty() && s.len() <= most && s.bytes().all(|c| c.is_ascii_digit()))
                .then(|| s.parse::<u32>().ok())
                .flatten()
        };
        let (h, m) = (digits(h, 2)?, digits(m, 2).filter(|_| m.len() == 2)?);
        (m < 60 && (h < 24 || (h == 24 && m == 0))).then_some(h * 60 + m)
    };
    let mut out = Vec::new();
    for part in text.split([',', ';']) {
        let (a, b) = part.split_once(['-', '–'])?;
        let (s, e) = (clock(a)?, clock(b)?);
        if s == 1440 || s == e {
            return None;
        }
        out.push((f64::from(s), f64::from(e)));
    }
    Some(out)
}

/// The day a text names: `YYYY-MM-DD`, then nothing or a time (`T` or a space and anything); days since 1970-01-01.
pub fn day_of(text: &str) -> Option<i64> {
    let t = text.trim();
    let b = t.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    if b.len() > 10 && b[10] != b'T' && b[10] != b' ' {
        return None;
    }
    let digits = |s: &[u8]| -> Option<i64> {
        s.iter()
            .try_fold(0i64, |n, c| c.is_ascii_digit().then(|| n * 10 + i64::from(c - b'0')))
    };
    let (y, m, d) = (digits(&b[0..4])?, digits(&b[5..7])?, digits(&b[8..10])?);
    if !(1..=12).contains(&m) || d < 1 || d > month_days(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

fn month_days(y: i64, m: i64) -> i64 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A day (days since 1970-01-01) as `YYYY-MM-DD` (Hinnant's `civil_from_days`).
pub fn date_text(day: i64) -> String {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Hinnant's `days_from_civil`).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_are_read_as_dates() {
        assert_eq!(day_of("1970-01-01"), Some(0));
        assert_eq!(day_of("2026-11-03"), Some(20_760));
        assert_eq!(day_of("2026-11-03T08:30"), Some(20_760));
        assert_eq!(day_of("2024-02-29"), Some(19_782));
        assert_eq!(day_of("2023-02-29"), None);
        assert_eq!(day_of("2026-13-01"), None);
        assert_eq!(day_of("03.11.2026"), None);
        assert_eq!(day_of("2026-11-03x"), None);
        for d in [-1, 0, 19_782, 20_760, 2_932_896] {
            assert_eq!(day_of(&date_text(d)), Some(d));
        }
    }

    #[test]
    fn weekdays_and_hours_are_read() {
        // 2026-11-03 is a Tuesday.
        assert_eq!(weekday_of(20_760), 1);
        assert_eq!(weekday_of(0), 3);
        assert_eq!(weekdays_of("Pzt-Cum"), Some(0b001_1111));
        assert_eq!(weekdays_of("CUMARTESİ, pazar"), Some(0b110_0000));
        assert_eq!(weekdays_of("Cum-Pzt"), Some(0b111_0001));
        assert_eq!(weekdays_of("1-5; 7"), Some(0b101_1111));
        assert_eq!(weekdays_of("Çarşamba"), Some(0b000_0100));
        assert_eq!(weekdays_of("carsamba"), Some(0b000_0100));
        assert_eq!(weekdays_of("Pzt,"), None);
        assert_eq!(weekdays_of("8"), None);
        assert_eq!(hours_of("22:00-06:00"), Some(vec![(1320.0, 360.0)]));
        assert_eq!(hours_of("7:00 - 9:30, 17:00-24:00"), Some(vec![(420.0, 570.0), (1020.0, 1440.0)]));
        assert_eq!(hours_of("00:00-24:00"), Some(vec![(0.0, 1440.0)]));
        assert_eq!(hours_of("08:00-08:00"), None);
        assert_eq!(hours_of("24:00-06:00"), None);
        assert_eq!(hours_of("8-9"), None);
        assert_eq!(hours_of("08:0-09:00"), None);
        assert_eq!(hours_of("08:60-09:00"), None);
    }
}
