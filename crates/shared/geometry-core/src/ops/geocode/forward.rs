//! From an address to its places (docs/adr/0229 §7): a query's parts
//! looked for in a locator. A code finds its doors; a street name finds the
//! streets whose names are similar (its first words dropped one by one when
//! the whole finds none, the dropped words then its neighbourhood); each
//! street is scored `100 − (100 − B) − P_tür − P_mahalle − P_ilçe −
//! P_posta − P_kapı` and placed at its door, between two doors, or at the
//! street; a neighbourhood name finds similar neighbourhoods at `M − 40`.
//! Candidates of 40 and more, at most `limit`, are sorted by score, kind,
//! street, neighbourhood, door and the drawing's order; the best one's
//! status is Eşleşti, Belirsiz (another of its score elsewhere) or
//! Eşleşmedi.

use std::cmp::Ordering;

use super::fold::Tag;
use super::format::label;
use super::locator::{Door, Group, Locator, Texts};
use super::parse::{Parsed, door_number};
use super::similar::{SIMILAR, similar};
use crate::vec2::Vec2;

/// The least score a candidate is listed with.
pub const LISTED: u32 = 40;

/// What a candidate is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// Adres kodu.
    Code,
    /// Kapı: the door itself, or a door of its number.
    Door,
    /// Kapı (aradeğer): between two doors.
    Interpolated,
    /// Yol: the street.
    Street,
    /// Mahalle.
    Neighbourhood,
}

impl Kind {
    /// Its name as the interface says it.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Code => "Kod",
            Kind::Door => "Kapı",
            Kind::Interpolated => "Kapı (aradeğer)",
            Kind::Street => "Yol",
            Kind::Neighbourhood => "Mahalle",
        }
    }
}

/// The best candidate's standing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    Matched,
    Tied,
    Unmatched,
}

impl Status {
    pub fn word(self) -> &'static str {
        match self {
            Status::Matched => "Eşleşti",
            Status::Tied => "Belirsiz",
            Status::Unmatched => "Eşleşmedi",
        }
    }
}

/// A place an address may be.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub kind: Kind,
    pub score: u32,
    pub at: Vec2,
    pub texts: Texts,
    pub label: String,
    /// Its place in the drawing's order (a door's, a street's first, a neighbourhood's).
    pub order: u32,
}

/// What a search found.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub candidates: Vec<Candidate>,
    pub status: Status,
}

/// The status of sorted candidates (§7.7).
pub fn status_of(candidates: &[Candidate], min_score: u32) -> Status {
    let Some(best) = candidates.first() else {
        return Status::Unmatched;
    };
    if best.score < min_score {
        return Status::Unmatched;
    }
    let tied = candidates[1..]
        .iter()
        .any(|c| c.score == best.score && (c.at.x != best.at.x || c.at.y != best.at.y));
    if tied { Status::Tied } else { Status::Matched }
}

/// The query's parts the search compares, its names as keys.
struct Asked<'q> {
    street: Option<String>,
    kind: Option<Tag>,
    door: Option<&'q str>,
    hood: Option<String>,
    district: Option<String>,
    postcode: Option<&'q str>,
}

/// A district of the locator's in the query's words (§4.7): the longest, the first of equal ones.
fn district_in(loc: &Locator, rest: &[String]) -> Option<String> {
    let mut best: Option<(usize, usize, &Vec<String>)> = None;
    for words in &loc.districts {
        let n = words.len();
        if n == 0 || n > rest.len() {
            continue;
        }
        let Some(at) = (0..=rest.len() - n).find(|&i| rest[i..i + n] == words[..]) else {
            continue;
        };
        if best.is_none_or(|(bn, bat, _)| n > bn || (n == bn && at < bat)) {
            best = Some((n, at, words));
        }
    }
    best.map(|(_, _, w)| w.join(" "))
}

/// How a listed candidate's place and words are made.
#[derive(Clone, Copy)]
enum Make {
    /// A door: the door itself, a door of its number, a code's.
    Door(usize),
    /// Between doors `a` and `c` of street group `group`, at `f` of the way.
    Between {
        group: usize,
        a: usize,
        c: usize,
        f: f64,
    },
    /// A street group's place.
    Street(usize),
    /// A neighbourhood at its place.
    Hood(usize, Vec2),
}

/// A candidate as the sort sees it (§7.6): its words written only when it is listed.
struct Draft<'a> {
    kind: Kind,
    score: u32,
    street_key: &'a str,
    hood_key: &'a str,
    door_key: &'a str,
    order: u32,
    make: Make,
}

/// The sort of §7.6: score, kind, street, neighbourhood, door, the drawing's order.
fn compare_drafts(a: &Draft<'_>, b: &Draft<'_>) -> Ordering {
    b.score
        .cmp(&a.score)
        .then_with(|| a.kind.cmp(&b.kind))
        .then_with(|| a.street_key.cmp(b.street_key))
        .then_with(|| a.hood_key.cmp(b.hood_key))
        .then_with(|| a.door_key.cmp(b.door_key))
        .then_with(|| a.order.cmp(&b.order))
}

/// A street group's words: its first door's (street, neighbourhood, district, province), else its first line's.
fn first_texts(loc: &Locator, group: &Group) -> Texts {
    match group.doors.first() {
        Some(&d) => {
            let t = &loc.doors[d].texts;
            Texts {
                street: t.street.clone(),
                neighbourhood: t.neighbourhood.clone(),
                district: t.district.clone(),
                province: t.province.clone(),
                ..Texts::default()
            }
        }
        None => group
            .lines
            .first()
            .map(|&l| loc.lines[l].texts.clone())
            .unwrap_or_default(),
    }
}

/// A listed draft's candidate: its place and words.
fn made(loc: &Locator, d: &Draft<'_>) -> Candidate {
    let (at, texts) = match d.make {
        Make::Door(i) => (loc.doors[i].at, loc.doors[i].texts.clone()),
        Make::Between { group, a, c, f } => {
            let (pa, pc) = (loc.doors[a].at, loc.doors[c].at);
            let mut texts = first_texts(loc, &loc.groups[group]);
            texts.door = d.door_key.to_owned();
            (
                Vec2::new(pa.x + (pc.x - pa.x) * f, pa.y + (pc.y - pa.y) * f),
                texts,
            )
        }
        Make::Street(g) => (loc.street_place(g), first_texts(loc, &loc.groups[g])),
        Make::Hood(h, at) => {
            let info = &loc.hood_info[h];
            (
                at,
                Texts {
                    neighbourhood: info.text.clone(),
                    district: info.district_text.clone(),
                    province: info.province_text.clone(),
                    ..Texts::default()
                },
            )
        }
    };
    Candidate {
        kind: d.kind,
        score: d.score,
        at,
        label: label(&texts),
        texts,
        order: d.order,
    }
}

/// The drafts listed (40 and more, at most `limit`, in the sort's order) as candidates, and their status.
fn listed(loc: &Locator, mut drafts: Vec<Draft<'_>>, min_score: u32, limit: usize) -> Found {
    drafts.retain(|d| d.score >= LISTED);
    drafts.sort_by(compare_drafts);
    drafts.truncate(limit);
    let candidates: Vec<Candidate> = drafts.iter().map(|d| made(loc, d)).collect();
    let status = status_of(&candidates, min_score);
    Found { candidates, status }
}

/// An address's candidates in a locator (§7). `text` is the one-line
/// address as written (its code is looked for), empty for columns.
pub fn find(loc: &Locator, text: &str, q: &Parsed, min_score: u32, limit: usize) -> Found {
    // 1. A code finds its doors, and nothing else is looked for.
    let code = text.trim();
    if !code.is_empty()
        && let Some(doors) = loc.codes.get(code)
    {
        let drafts = doors
            .iter()
            .map(|&i| {
                let d = &loc.doors[i];
                let g = &loc.groups[d.group];
                Draft {
                    kind: Kind::Code,
                    score: 100,
                    street_key: &g.name,
                    hood_key: &g.neighbourhood,
                    door_key: &d.key,
                    order: d.order,
                    make: Make::Door(i),
                }
            })
            .collect();
        return listed(loc, drafts, min_score, limit);
    }
    let mut asked = Asked {
        street: q.street.clone(),
        kind: q.street_kind,
        door: q.door.as_deref(),
        hood: q.neighbourhood.clone(),
        district: q.district.clone().or_else(|| district_in(loc, &q.rest)),
        postcode: q.postcode.as_deref(),
    };
    // 2. The streets whose names are similar; the first words dropped when none is.
    let mut names: Vec<(usize, u32)> = Vec::new();
    if let Some(street) = asked.street.clone() {
        names = loc.names.similar(&street);
        let words: Vec<&str> = street.split(' ').collect();
        if names.is_empty() && words.len() >= 2 {
            for k in 1..words.len() {
                let tail = words[k..].join(" ");
                let found = loc.names.similar(&tail);
                if !found.is_empty() {
                    if asked.hood.is_none() {
                        asked.hood = Some(words[..k].join(" "));
                    }
                    asked.street = Some(tail);
                    names = found;
                    break;
                }
            }
        }
    }
    let mut drafts: Vec<Draft<'_>> = Vec::new();
    for (name, b) in names {
        for &g in &loc.name_groups[name] {
            drafts.push(street_draft(loc, g, b, &asked));
        }
    }
    // 5. Neighbourhoods: the asked one, or an untagged address's name.
    let hood = if q.untagged {
        q.street.clone()
    } else {
        asked.hood.clone()
    };
    if let Some(hood) = hood {
        for (h, m) in loc.hoods.similar(&hood) {
            let info = &loc.hood_info[h];
            let Some(at) = loc.hood_place(&info.key) else {
                continue;
            };
            drafts.push(Draft {
                kind: Kind::Neighbourhood,
                score: m.saturating_sub(40),
                street_key: "",
                hood_key: &info.key,
                door_key: "",
                order: info.order,
                make: Make::Hood(h, at),
            });
        }
    }
    listed(loc, drafts, min_score, limit)
}

/// A street's draft for the asked parts (§7.3, §7.4).
fn street_draft<'a>(loc: &'a Locator, g: usize, b: u32, asked: &Asked<'a>) -> Draft<'a> {
    let group = &loc.groups[g];
    let mut penalty: u32 = 100 - b;
    if let (Some(a), Some(k)) = (asked.kind, group.kind)
        && a != k
    {
        penalty += 5;
    }
    if let Some(h) = &asked.hood
        && !group.neighbourhood.is_empty()
    {
        penalty += match similar(h, &group.neighbourhood) {
            Some(m) => 100 - m,
            None => 30,
        };
    }
    if let Some(d) = &asked.district
        && !group.district.is_empty()
        && similar(d, &group.district).is_none()
    {
        penalty += 20;
    }
    if let Some(p) = asked.postcode
        && !group.postcodes.is_empty()
        && !group.postcodes.iter().any(|c| c == p)
    {
        penalty += 10;
    }
    let score = |extra: u32| 100u32.saturating_sub(penalty + extra);
    let draft = |kind: Kind, score: u32, door_key: &'a str, order: u32, make: Make| Draft {
        kind,
        score,
        street_key: &group.name,
        hood_key: &group.neighbourhood,
        door_key,
        order,
        make,
    };
    let doors = || group.doors.iter().map(|&i| (i, &loc.doors[i]));
    if let Some(door) = asked.door {
        // The door itself.
        if let Some((i, d)) = doors().find(|(_, d)| d.key == door) {
            return draft(Kind::Door, score(0), &d.key, d.order, Make::Door(i));
        }
        let number = door_number(door);
        // A door of its number: the one without a letter, else the least key.
        if let Some(n) = number {
            let plain = n.to_string();
            let same = doors()
                .filter(|(_, d)| d.number == Some(n))
                .min_by(|(_, a), (_, b)| {
                    let lettered = |d: &Door| d.key != plain;
                    lettered(a)
                        .cmp(&lettered(b))
                        .then_with(|| a.key.cmp(&b.key))
                        .then_with(|| a.order.cmp(&b.order))
                });
            if let Some((i, d)) = same {
                return draft(Kind::Door, score(5), &d.key, d.order, Make::Door(i));
            }
            // Between two doors of its side.
            let side = |d: &Door| d.number.is_some_and(|m| m % 2 == n % 2);
            let low = doors()
                .filter(|(_, d)| side(d))
                .filter_map(|(_, d)| d.number.filter(|&m| m < n))
                .max();
            let high = doors()
                .filter(|(_, d)| side(d))
                .filter_map(|(_, d)| d.number.filter(|&m| m > n))
                .min();
            let first_of = |want: Option<u64>| {
                let want = want?;
                doors()
                    .filter(|(_, d)| d.number == Some(want))
                    .min_by(|(_, a), (_, b)| a.key.cmp(&b.key).then_with(|| a.order.cmp(&b.order)))
            };
            if let (Some((ia, a)), Some((ic, c))) = (first_of(low), first_of(high)) {
                let (na, nc) = (a.number.unwrap_or(0) as f64, c.number.unwrap_or(0) as f64);
                let f = (n as f64 - na) / (nc - na);
                return draft(
                    Kind::Interpolated,
                    score(10),
                    door,
                    a.order.min(c.order),
                    Make::Between {
                        group: g,
                        a: ia,
                        c: ic,
                        f,
                    },
                );
            }
        }
        return draft(Kind::Street, score(25), "", group.order, Make::Street(g));
    }
    draft(Kind::Street, score(15), "", group.order, Make::Street(g))
}

/// The street-level and region-level similarity floor, for callers that score their own.
pub const NAME_FLOOR: u32 = SIMILAR;
