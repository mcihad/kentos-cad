//! Denetle (docs/adr/0229 §13): what a locator holds and what it left out:
//! its doors, streets, lines, neighbourhoods and areas; the objects that are
//! not of their layer's kind, the doors without a street or a door number,
//! the lines and areas without a name; doors of one street with one number
//! (the first 100, in the order of their first doors); and (docs/adr/0579)
//! address codes on more than one door, and streets whose doors write their
//! name in more than one way (the address's normalization: the spellings and
//! how many doors write each, the most written first); and (docs/adr/0647 §1)
//! neighbourhoods and districts whose doors write their names in more than one
//! way ([`name_groups`], which `cad.addresses.normalize` sets right).

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

/// Which name of the doors a group is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Named {
    /// A street's: its doors (grouped by street, neighbourhood and district).
    Street,
    /// A neighbourhood's: doors grouped by the neighbourhood's key and the district's.
    Neighbourhood,
    /// A district's: doors grouped by the district's key.
    District,
}

/// Doors that write one name in more than one way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameGroup {
    /// The district its first door writes (a neighbourhood's group); empty otherwise.
    pub within: String,
    /// The spellings and how many doors write each: the most written first, on a tie the one written first.
    pub written: Vec<(String, usize)>,
    /// Its doors' places in the drawing's order and the name each writes.
    pub doors: Vec<(u32, String)>,
}

/// The groups of `named` whose doors write the name in more than one way, by their first doors. A door without the
/// group's key (an empty neighbourhood or district) is in none.
pub fn name_groups(loc: &Locator, named: Named) -> Vec<NameGroup> {
    let text_of = |i: usize| -> &str {
        let t = &loc.doors[i].texts;
        match named {
            Named::Street => &t.street,
            Named::Neighbourhood => &t.neighbourhood,
            Named::District => &t.district,
        }
    };
    let keyed: Vec<Vec<usize>> = match named {
        Named::Street => loc.groups.iter().map(|g| g.doors.clone()).collect(),
        Named::Neighbourhood | Named::District => {
            let mut by: Vec<((String, String), Vec<usize>)> = Vec::new();
            let mut at: std::collections::HashMap<(String, String), usize> =
                std::collections::HashMap::new();
            let mut doors: Vec<usize> = (0..loc.doors.len()).collect();
            doors.sort_by_key(|&i| loc.doors[i].order);
            for i in doors {
                let g = &loc.groups[loc.doors[i].group];
                let key = match named {
                    Named::Neighbourhood if !g.neighbourhood.is_empty() => {
                        (g.neighbourhood.clone(), g.district.clone())
                    }
                    Named::District if !g.district.is_empty() => {
                        (String::new(), g.district.clone())
                    }
                    _ => continue,
                };
                let k = *at.entry(key.clone()).or_insert_with(|| {
                    by.push((key, Vec::new()));
                    by.len() - 1
                });
                by[k].1.push(i);
            }
            by.into_iter().map(|(_, d)| d).collect()
        }
    };
    let mut out = Vec::new();
    for doors in keyed {
        let mut written: Vec<(String, usize, u32)> = Vec::new();
        for &i in &doors {
            let t = text_of(i);
            match written.iter_mut().find(|(w, _, _)| w == t) {
                Some((_, n, _)) => *n += 1,
                None => written.push((t.to_owned(), 1, loc.doors[i].order)),
            }
        }
        if written.len() < 2 {
            continue;
        }
        written.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
        let mut listed: Vec<(u32, String)> = doors
            .iter()
            .map(|&i| (loc.doors[i].order, text_of(i).to_owned()))
            .collect();
        listed.sort_by_key(|d| d.0);
        let within = match named {
            Named::Neighbourhood => doors
                .iter()
                .min_by_key(|&&i| loc.doors[i].order)
                .map(|&i| loc.doors[i].texts.district.clone())
                .unwrap_or_default(),
            _ => String::new(),
        };
        out.push(NameGroup {
            within,
            written: written.into_iter().map(|(t, n, _)| (t, n)).collect(),
            doors: listed,
        });
    }
    out.sort_by_key(|g| g.doors[0].0);
    out
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
    /// Neighbourhoods and districts written in more than one way (docs/adr/0647 §1): all of them (the command lists
    /// the first `limit`).
    pub neighbourhood_spellings: Vec<NameGroup>,
    pub district_spellings: Vec<NameGroup>,
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
        neighbourhood_spellings: name_groups(loc, Named::Neighbourhood),
        district_spellings: name_groups(loc, Named::District),
    }
}
