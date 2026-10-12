//! Denetle (docs/adr/0229 §13): what a locator holds and what it left out:
//! its doors, streets, lines, neighbourhoods and areas; the objects that are
//! not of their layer's kind, the doors without a street or a door number,
//! the lines and areas without a name; doors of one street with one number
//! (the first 100, in the order of their first doors).

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
    }
}
