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
    /// The facts of a closure's first and last day.
    pub closures: Option<[usize; 2]>,
}

/// What a question asks of the network besides its places: the vehicle (an index into the rules' profiles) and the
/// day (days since 1970-01-01). None of either: the network as defined.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Access {
    pub profile: Option<usize>,
    pub day: Option<i64>,
}

impl Access {
    pub fn is_none(&self) -> bool {
        self.profile.is_none() && self.day.is_none()
    }
}

/// What varying the graph closed: how many edges the profile closed, how many the day closed (an edge both once, the
/// profile's), and the edges whose values could not be read (their objects, once each, in the graph's order).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Varied {
    pub by_profile: usize,
    pub by_date: usize,
    pub unread: Vec<f64>,
}

/// Why an edge is closed to a question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closing {
    Open,
    Profile,
    Date,
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
                rules.closures = Some([fact(&mut rules.facts, &from), fact(&mut rules.facts, &to)]);
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

    /// Whether `access` closes an edge whose facts are `facts` (`facts[i]` the value of `self.facts[i]`); `unread` set
    /// when a value it reads cannot be read.
    pub fn closing(&self, access: &Access, facts: &[Option<String>], unread: &mut bool) -> Closing {
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
        if let (Some(day), Some([a, b])) = (access.day, self.closures) {
            let bound = |i: usize, unread: &mut bool| match fact(i) {
                None => Some(None),
                Some(t) => match day_of(t) {
                    Some(d) => Some(Some(d)),
                    None => {
                        *unread = true;
                        None
                    }
                },
            };
            if let (Some(from), Some(to)) = (bound(a, unread), bound(b, unread))
                && (from.is_some() || to.is_some())
                && from.is_none_or(|f| f <= day)
                && to.is_none_or(|t| day <= t)
            {
                return Closing::Date;
            }
        }
        Closing::Open
    }
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
}
