//! An online candidate scored by the locator's rules (docs/adr/0229 §10):
//! the provider's own score is shown, not used. Its street is read as a
//! street column is and compared with the query's (the query's first words
//! dropped one by one when the whole is not similar, the dropped words its
//! neighbourhood then); the neighbourhood is compared with the candidate's
//! every area name, the best taken; kind and door as §7 says; no district or
//! postcode penalty. A candidate whose street is not similar is a place name
//! (an untagged address similar to its name, `B − 15`), an area (a
//! neighbourhood similar to an area name, `M − 40`), or 0.

use kentos_geometry_core::ops::geocode::Status;
use kentos_geometry_core::ops::geocode::parse::{
    Parsed, area_value, door_number, door_value, street_value, words_key,
};
use kentos_geometry_core::ops::geocode::similar::similar;

use super::read::OnlineCandidate;

/// What an online candidate is, as the window names it.
pub fn kind_word(kind: Kind) -> &'static str {
    match kind {
        Kind::Door => "Kapı",
        Kind::Street => "Yol",
        Kind::Place => "Yer adı",
        Kind::Area => "Bölge",
        Kind::Other => "Diğer",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Door,
    Street,
    Place,
    Area,
    Other,
}

/// A candidate's score and kind.
pub fn score(q: &Parsed, c: &OnlineCandidate) -> (u32, Kind) {
    let street = street_value(&c.street);
    let mut hood_asked = q.neighbourhood.clone();
    let mut b = None;
    if let (Some(asked), Some((name, _))) = (&q.street, &street) {
        b = similar(asked, name);
        let words: Vec<&str> = asked.split(' ').collect();
        if b.is_none() && words.len() >= 2 {
            for k in 1..words.len() {
                let tail = words[k..].join(" ");
                if let Some(s) = similar(&tail, name) {
                    if hood_asked.is_none() {
                        hood_asked = Some(words[..k].join(" "));
                    }
                    b = Some(s);
                    break;
                }
            }
        }
    }
    let areas: Vec<String> = c
        .areas
        .iter()
        .map(|a| area_value(a))
        .filter(|a| !a.is_empty())
        .collect();
    let best_area = |name: &str| areas.iter().filter_map(|a| similar(name, a)).max();
    if let (Some(b), Some((_, kind))) = (b, &street) {
        let mut penalty = 100 - b;
        if let (Some(a), Some(k)) = (q.street_kind, kind)
            && a != *k
        {
            penalty += 5;
        }
        if let Some(h) = &hood_asked
            && !areas.is_empty()
        {
            penalty += best_area(h).map_or(30, |m| 100 - m);
        }
        let own = door_value(&c.door);
        let (extra, kind) = match (q.door.as_deref(), own.as_deref()) {
            (Some(asked), Some(own)) if asked == own => (0, Kind::Door),
            (Some(asked), Some(own))
                if door_number(asked).is_some() && door_number(asked) == door_number(own) =>
            {
                (5, Kind::Door)
            }
            (Some(_), _) => (25, Kind::Street),
            (None, _) => (15, Kind::Street),
        };
        return (100u32.saturating_sub(penalty + extra), kind);
    }
    if q.untagged
        && let (Some(asked), false) = (&q.street, c.name.is_empty())
        && let Some(b) = similar(asked, &words_key(&c.name))
    {
        return (b.saturating_sub(15), Kind::Place);
    }
    let hood = if q.untagged {
        q.street.clone()
    } else {
        hood_asked
    };
    if let Some(h) = hood
        && let Some(m) = best_area(&h)
    {
        return (m.saturating_sub(40), Kind::Area);
    }
    (0, Kind::Other)
}

/// A batch's pick among a provider's candidates (§10): the highest score,
/// the provider's first of equal ones, and its status.
pub fn pick(
    q: &Parsed,
    candidates: &[OnlineCandidate],
    min_score: u32,
) -> Option<(usize, u32, Kind, Status)> {
    let scored: Vec<(u32, Kind)> = candidates.iter().map(|c| score(q, c)).collect();
    let (best, &(s, kind)) = scored
        .iter()
        .enumerate()
        .max_by(|(i, a), (j, b)| a.0.cmp(&b.0).then_with(|| j.cmp(i)))?;
    let status = if s < min_score {
        Status::Unmatched
    } else if scored.iter().enumerate().any(|(i, (o, _))| {
        i != best
            && *o == s
            && (candidates[i].lon != candidates[best].lon
                || candidates[i].lat != candidates[best].lat)
    }) {
        Status::Tied
    } else {
        Status::Matched
    };
    Some((best, s, kind, status))
}
