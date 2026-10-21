//! Denetle (docs/adr/0229 §13): what a locator holds and what it left out:
//! its doors, streets, lines, neighbourhoods and areas; the objects that are
//! not of their layer's kind, the doors without a street or a door number,
//! the lines and areas without a name; doors of one street with one number
//! (the first 100, in the order of their first doors); and (docs/adr/0579)
//! address codes on more than one door, and streets whose doors write their
//! name in more than one way (the address's normalization: the spellings and
//! how many doors write each, the most written first).

use super::locator::{Counts, Locator};

/// The most repeated doors listed.
pub const REPEATS_LISTED: usize = 100;

/// One street's door that is there more than once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repeat {
    pub street: String,
    pub neighbourhood: String,
    pub door: String,
    /// Its objects' places in the drawing's order.
    pub orders: Vec<u32>,
}

/// One address code on more than one door.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeRepeat {
    pub code: String,
    /// Its doors' places in the drawing's order.
    pub orders: Vec<u32>,
}

/// A street whose doors write its name in more than one way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spelling {
    pub neighbourhood: String,
    pub district: String,
    /// The spellings, as the doors write them, with how many doors write each: the most written first, on a tie the
    /// one written first.
    pub written: Vec<(String, usize)>,
    /// The doors' places in the drawing's order.
    pub orders: Vec<u32>,
}

/// What a check finds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub counts: Counts,
    pub not_doors: usize,
    pub empty_streets: usize,
    pub unread_doors: usize,
    pub not_streets: usize,
    pub unnamed_lines: usize,
    pub not_areas: usize,
    pub unnamed_areas: usize,
    /// How many doors are there more than once, and the first of them.
    pub repeated: usize,
    pub repeats: Vec<Repeat>,
    /// How many address codes are on more than one door, and the first of them (by their first door).
    pub coded: usize,
    pub code_repeats: Vec<CodeRepeat>,
    /// How many streets are written in more than one way, and the first of them (by their first door).
    pub spelled: usize,
    pub spellings: Vec<Spelling>,
}

pub fn check(loc: &Locator) -> Summary {
    let mut repeats: Vec<Repeat> = Vec::new();
    let mut repeated = 0;
    for g in &loc.groups {
        let mut seen: Vec<(&str, Vec<u32>, usize)> = Vec::new();
        for &i in &g.doors {
            let d = &loc.doors[i];
            match seen.iter_mut().find(|(k, _, _)| *k == d.key) {
                Some((_, orders, _)) => orders.push(d.order),
                None => seen.push((&d.key, vec![d.order], i)),
            }
        }
        for (_, orders, first) in seen {
            if orders.len() < 2 {
                continue;
            }
            repeated += 1;
            let d = &loc.doors[first];
            repeats.push(Repeat {
                street: d.texts.street.clone(),
                neighbourhood: d.texts.neighbourhood.clone(),
                door: d.texts.door.clone(),
                orders,
            });
        }
    }
    repeats.sort_by_key(|r| r.orders[0]);
    repeats.truncate(REPEATS_LISTED);
    let mut code_repeats: Vec<CodeRepeat> = loc
        .codes
        .iter()
        .filter(|(_, doors)| doors.len() > 1)
        .map(|(code, doors)| {
            let mut orders: Vec<u32> = doors.iter().map(|&i| loc.doors[i].order).collect();
            orders.sort_unstable();
            CodeRepeat {
                code: code.clone(),
                orders,
            }
        })
        .collect();
    let coded = code_repeats.len();
    code_repeats.sort_by(|a, b| {
        a.orders[0]
            .cmp(&b.orders[0])
            .then_with(|| a.code.cmp(&b.code))
    });
    code_repeats.truncate(REPEATS_LISTED);
    let mut spellings: Vec<Spelling> = Vec::new();
    for g in &loc.groups {
        // Each spelling once, in the order first written, with its count.
        let mut written: Vec<(String, usize, u32)> = Vec::new();
        for &i in &g.doors {
            let d = &loc.doors[i];
            match written.iter_mut().find(|(t, _, _)| *t == d.texts.street) {
                Some((_, n, _)) => *n += 1,
                None => written.push((d.texts.street.clone(), 1, d.order)),
            }
        }
        if written.len() < 2 {
            continue;
        }
        written.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
        let mut orders: Vec<u32> = g.doors.iter().map(|&i| loc.doors[i].order).collect();
        orders.sort_unstable();
        let first = &loc.doors[g.doors[0]].texts;
        spellings.push(Spelling {
            neighbourhood: first.neighbourhood.clone(),
            district: first.district.clone(),
            written: written.into_iter().map(|(t, n, _)| (t, n)).collect(),
            orders,
        });
    }
    let spelled = spellings.len();
    spellings.sort_by_key(|s| s.orders[0]);
    spellings.truncate(REPEATS_LISTED);
    Summary {
        counts: loc.counts(),
        not_doors: loc.not_doors,
        empty_streets: loc.empty_streets,
        unread_doors: loc.unread_doors,
        not_streets: loc.not_streets,
        unnamed_lines: loc.unnamed_lines,
        not_areas: loc.not_areas,
        unnamed_areas: loc.unnamed_areas,
        repeated,
        repeats,
        coded,
        code_repeats,
        spelled,
        spellings,
    }
}
