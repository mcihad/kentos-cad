//! An address data's doors compared with an address register (docs/adr/0736): the institution's or the national
//! address database's table of address codes, each row a code and, as far as the register gives them, its street, door
//! number, neighbourhood, district and position. The code (UAVT's address code) is an independent section's, so a door
//! may have many in the register; a door is the register's by its code, an address the drawing's by its parts:
//!
//! - a door's code no row has (`unregistered`);
//! - a door whose code is a row's but whose street, door number, neighbourhood or district the row writes otherwise
//!   (`differences`): compared by the locator's keys, so a spelling (“Atatürk Blv.”, “ATATÜRK BULVARI”) is no difference
//!   and a street's kind is one only when both say it;
//! - a door farther from its row's position than the tolerance (`distant`, metres);
//! - the register's addresses no door of the drawing has: rows whose code is on no door and whose street and door
//!   number no door has, the neighbourhood and district agreeing where both say them (`undrawn`);
//! - a code on more than one row (`repeats`): the first row is the one a door is compared with.
//!
//! The doors in the drawing's order, the register's lists by their first rows. The independent reference is
//! `scripts/fixtures/address_compare_cases.py`.

use std::collections::HashMap;

use super::fold::Tag;
use super::locator::Locator;
use super::parse::{area_value, door_value, street_value, words_key};
use crate::vec2::Vec2;

/// A row of the register as the host reads it: its texts trimmed, a column the register lacks (or the address data
/// does not compare) none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegisterRow {
    /// Its place in the register, from 1.
    pub row: u32,
    /// Not empty.
    pub code: String,
    pub street: Option<String>,
    pub door: Option<String>,
    pub neighbourhood: Option<String>,
    pub district: Option<String>,
    /// In the project's unit.
    pub at: Option<Vec2>,
}

/// A part of an address the register writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Street,
    Door,
    Neighbourhood,
    District,
}

/// A part a door writes otherwise than its row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartDifference {
    pub part: Part,
    /// The door's text, trimmed.
    pub drawing: String,
    /// The row's text, trimmed.
    pub register: String,
}

/// A door whose code no row has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unregistered {
    pub order: u32,
    pub code: String,
    pub street: String,
    pub door: String,
}

/// A door that writes its address otherwise than its row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    pub order: u32,
    pub code: String,
    pub row: u32,
    pub parts: Vec<PartDifference>,
}

/// A door farther from its row's position than the tolerance.
#[derive(Clone, Debug, PartialEq)]
pub struct Distant {
    pub order: u32,
    pub code: String,
    pub row: u32,
    /// Metres.
    pub distance: f64,
}

/// An address of the register no door has: its first row's texts, its codes and rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Undrawn {
    pub street: String,
    pub door: String,
    pub neighbourhood: String,
    pub codes: Vec<String>,
    pub rows: Vec<u32>,
}

/// A code on more than one row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repeat {
    pub code: String,
    pub rows: Vec<u32>,
}

/// What the comparison found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Comparison {
    /// The locator's doors, those with a code, and those whose code a row has.
    pub doors: usize,
    pub coded: usize,
    pub matched: usize,
    pub unregistered: Vec<Unregistered>,
    pub differences: Vec<Difference>,
    pub distant: Vec<Distant>,
    pub undrawn: Vec<Undrawn>,
    pub repeats: Vec<Repeat>,
}

/// Whether two keys agree: the same, or one of them not said.
fn agree(a: &str, b: &str) -> bool {
    a == b || a.is_empty() || b.is_empty()
}

/// Whether a street the register writes is the door's: the same name, the kinds the same where both say one.
fn same_street(register: &str, name: &str, kind: Option<Tag>) -> bool {
    match street_value(register) {
        Some((n, k)) => n == name && (k.is_none() || kind.is_none() || k == kind),
        None => false,
    }
}

/// `loc`'s doors compared with the register's `rows` (in the register's order), positions `tolerance` metres apart
/// or more said.
pub fn compare(loc: &Locator, rows: &[RegisterRow], tolerance: f64) -> Comparison {
    let mut out = Comparison {
        doors: loc.doors.len(),
        ..Comparison::default()
    };
    // The register's codes: the first row of each, and the codes on more than one.
    let mut first: HashMap<&str, usize> = HashMap::new();
    let mut repeats: Vec<(usize, Repeat)> = Vec::new();
    let mut repeat_at: HashMap<&str, usize> = HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        match first.get(r.code.as_str()) {
            None => {
                first.insert(r.code.as_str(), i);
            }
            Some(&f) => match repeat_at.get(r.code.as_str()) {
                Some(&k) => repeats[k].1.rows.push(r.row),
                None => {
                    repeat_at.insert(r.code.as_str(), repeats.len());
                    repeats.push((
                        f,
                        Repeat {
                            code: r.code.clone(),
                            rows: vec![rows[f].row, r.row],
                        },
                    ));
                }
            },
        }
    }
    repeats.sort_by_key(|(f, _)| *f);
    out.repeats = repeats.into_iter().map(|(_, r)| r).collect();
    // The doors, in the drawing's order.
    let mut doors: Vec<usize> = (0..loc.doors.len()).collect();
    doors.sort_by_key(|&i| loc.doors[i].order);
    for &i in &doors {
        let d = &loc.doors[i];
        if d.code.is_empty() {
            continue;
        }
        out.coded += 1;
        let Some(&f) = first.get(d.code.as_str()) else {
            out.unregistered.push(Unregistered {
                order: d.order,
                code: d.code.clone(),
                street: d.texts.street.clone(),
                door: d.texts.door.clone(),
            });
            continue;
        };
        out.matched += 1;
        let r = &rows[f];
        let g = &loc.groups[d.group];
        let mut parts = Vec::new();
        let mut differ = |part: Part, said: &Option<String>, same: &dyn Fn(&str) -> bool, drawing: &str| {
            if let Some(text) = said
                && !text.is_empty()
                && !same(text)
            {
                parts.push(PartDifference {
                    part,
                    drawing: drawing.to_owned(),
                    register: text.clone(),
                });
            }
        };
        differ(
            Part::Street,
            &r.street,
            &|t| same_street(t, &g.name, d.kind),
            &d.texts.street,
        );
        differ(
            Part::Door,
            &r.door,
            &|t| door_value(t).as_deref() == Some(d.key.as_str()),
            &d.texts.door,
        );
        differ(
            Part::Neighbourhood,
            &r.neighbourhood,
            &|t| area_value(t) == g.neighbourhood,
            &d.texts.neighbourhood,
        );
        differ(
            Part::District,
            &r.district,
            &|t| words_key(t) == g.district,
            &d.texts.district,
        );
        if !parts.is_empty() {
            out.differences.push(Difference {
                order: d.order,
                code: d.code.clone(),
                row: r.row,
                parts,
            });
        }
        if let Some(at) = r.at {
            let (dx, dy) = (d.at.x - at.x, d.at.y - at.y);
            let distance = (dx * dx + dy * dy).sqrt() / loc.per_metre;
            if distance > tolerance {
                out.distant.push(Distant {
                    order: d.order,
                    code: d.code.clone(),
                    row: r.row,
                    distance,
                });
            }
        }
    }
    // The register's addresses no door has: by street name and door number, each door of them checked.
    let mut by_address: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (i, d) in loc.doors.iter().enumerate() {
        by_address
            .entry((loc.groups[d.group].name.as_str(), d.key.as_str()))
            .or_default()
            .push(i);
    }
    type Key = (String, String, String, String);
    let mut undrawn: Vec<(Key, Undrawn)> = Vec::new();
    let mut undrawn_at: HashMap<Key, usize> = HashMap::new();
    for r in rows {
        if loc.codes.contains_key(r.code.as_str()) {
            continue;
        }
        let (Some(street), Some(door)) = (r.street.as_deref(), r.door.as_deref()) else {
            continue;
        };
        let (Some((name, _)), Some(number)) = (street_value(street), door_value(door)) else {
            continue;
        };
        let hood = r.neighbourhood.as_deref().map(area_value).unwrap_or_default();
        let district = r.district.as_deref().map(words_key).unwrap_or_default();
        let drawn = by_address
            .get(&(name.as_str(), number.as_str()))
            .is_some_and(|ds| {
                ds.iter().any(|&i| {
                    let g = &loc.groups[loc.doors[i].group];
                    agree(&g.neighbourhood, &hood) && agree(&g.district, &district)
                })
            });
        if drawn {
            continue;
        }
        let key = (name, number, hood, district);
        match undrawn_at.get(&key) {
            Some(&k) => {
                let u = &mut undrawn[k].1;
                if !u.codes.contains(&r.code) {
                    u.codes.push(r.code.clone());
                }
                u.rows.push(r.row);
            }
            None => {
                undrawn_at.insert(key.clone(), undrawn.len());
                undrawn.push((
                    key,
                    Undrawn {
                        street: street.to_owned(),
                        door: door.to_owned(),
                        neighbourhood: r.neighbourhood.clone().unwrap_or_default(),
                        codes: vec![r.code.clone()],
                        rows: vec![r.row],
                    },
                ));
            }
        }
    }
    out.undrawn = undrawn.into_iter().map(|(_, u)| u).collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::geocode::{DoorInput, Input};

    fn door(order: u32, street: &str, number: &str, hood: &str, code: &str, x: f64) -> DoorInput {
        DoorInput {
            order,
            at: Vec2::new(500_000.0 + x, 4_400_000.0),
            street: street.into(),
            door: number.into(),
            neighbourhood: hood.into(),
            district: "Çankaya".into(),
            code: code.into(),
            ..DoorInput::default()
        }
    }

    fn row(row: u32, code: &str, street: &str, number: &str, hood: &str) -> RegisterRow {
        RegisterRow {
            row,
            code: code.into(),
            street: Some(street.into()),
            door: Some(number.into()),
            neighbourhood: Some(hood.into()),
            district: None,
            at: None,
        }
    }

    #[test]
    fn a_register_says_what_the_doors_lack_and_write_otherwise() {
        let loc = Locator::build(&Input {
            doors: vec![
                door(0, "Atatürk Bulvarı", "12", "Kızılay", "1001", 0.0),
                door(1, "Atatürk Bulvarı", "14", "Kızılay", "1002", 10.0),
                door(2, "Esat Cd.", "3", "Kızılay", "1099", 20.0),
                door(3, "Esat Cd.", "5", "Kızılay", "", 30.0),
            ],
            per_metre: 1.0,
            ..Input::default()
        });
        let rows = vec![
            row(1, "1001", "ATATÜRK BLV.", "12", "Kızılay Mahallesi"),
            row(2, "1002", "Atatürk Bulvarı", "16", "Kızılay"),
            row(3, "1003", "Kuğulu Sk.", "1", "Kızılay"),
            row(4, "1004", "Kuğulu Sokak", "1", "KIZILAY"),
            row(5, "1001", "Atatürk Bulvarı", "12", "Kızılay"),
            row(6, "1005", "Esat Caddesi", "5", "Kızılay"),
        ];
        let c = compare(&loc, &rows, 10.0);
        assert_eq!((c.doors, c.coded, c.matched), (4, 3, 2));
        assert_eq!(c.unregistered.len(), 1);
        assert_eq!(c.unregistered[0].code, "1099");
        // A spelling is no difference; another door number is.
        assert_eq!(c.differences.len(), 1);
        assert_eq!(c.differences[0].parts[0].part, Part::Door);
        // Kuğulu 1 is undrawn with both its codes (two units of one building); Esat 5 has a door.
        assert_eq!(c.undrawn.len(), 1);
        assert_eq!(c.undrawn[0].codes, ["1003", "1004"]);
        assert_eq!(c.repeats, [Repeat { code: "1001".into(), rows: vec![1, 5] }]);
    }
}
