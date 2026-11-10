//! Mahal listesinin kapı ve pencereleri (docs/adr/8038; M38): each door and window in the room it serves, as Planlı
//! Alanlar İmar Yönetmeliği 57(29) lists a room's "kapı-pencere kasa ve kanatları" by the room's code. The rule is the
//! one Doğrama kodları numbers the openings by (docs/adr/8028), so a room's number on its doors and its row in the list
//! agree:
//!
//! - a door between a room and a corridor, hall or the like (geçiş mahalleri) is the room's; between two rooms the one
//!   it opens into; by one room only, that one's;
//! - a window is the room's on either side of it, its left first.
//!
//! Each is said by its type's joinery code (K01), or by its size (90/210) when its type has none; a room's list is
//! sorted, a code that serves it more than once with its count: “K01 (2), K02”. A void, a bare opening, a niche and a
//! flue take none; the design options the drawing does not show are left out, as the schedules leave them.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{Entity, OpeningFields, OpeningForm, OpeningSide};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::geom::arrangement::Area;

use crate::Document;
use crate::Slot;
use crate::arch_values::core_areas;

/// How far beyond a wall's face an opening's side is looked at for a room, metres (Doğrama kodları' too).
pub const SIDE: f64 = 0.3;

/// An opening's middle and its two sides' points a little beyond its wall's faces: left, right.
pub fn opening_sides(f: &OpeningFields) -> Option<(V, V, V)> {
    let p: Vec<V> = f.place.iter().map(|q| V::new(q.x, q.y)).collect();
    if p.len() != 4 {
        return None;
    }
    let mid = |a: V, b: V| V::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let (left, right) = (mid(p[0], p[2]), mid(p[1], p[3]));
    let c = mid(left, right);
    let (dx, dy) = (left.x - right.x, left.y - right.y);
    let len = dx.hypot(dy);
    if len < 1e-9 {
        return None;
    }
    let n = V::new(dx / len, dy / len);
    Some((
        c,
        V::new(left.x + n.x * SIDE, left.y + n.y * SIDE),
        V::new(right.x - n.x * SIDE, right.y - n.y * SIDE),
    ))
}

/// A room's doors and windows, each list as the room list writes it (“K01 (2), K02”); empty: none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomOpenings {
    pub doors: String,
    pub windows: String,
}

/// A room the openings are matched to: its level, its slot, whether people pass through it, its areas.
type Room = (Option<String>, Slot, bool, Vec<Area>);

/// “K01 (2), K02”: the codes in order, each more than once with its count.
fn said(codes: &BTreeMap<String, usize>) -> String {
    codes
        .iter()
        .map(|(c, n)| {
            if *n > 1 {
                format!("{c} ({n})")
            } else {
                c.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

impl Document {
    /// What an opening is called in the room list: its type's joinery code, else its size (90/210).
    fn joinery_label(&self, f: &OpeningFields) -> String {
        f.style
            .as_deref()
            .and_then(|s| self.opening_types().iter().find(|t| t.id == s))
            .and_then(|t| t.code.clone())
            .filter(|c| !c.trim().is_empty())
            .unwrap_or_else(|| f.size_cm())
    }

    /// The room an opening serves, among `rooms` (docs/adr/8028's rule).
    fn served_room(&self, f: &OpeningFields, rooms: &[Room]) -> Option<Slot> {
        let (_, left, right) = opening_sides(f)?;
        let level = self.wall_of(&f.host).and_then(|w| w.arch.level);
        let at = |p: V| {
            rooms.iter().find(|(l, _, _, areas)| {
                *l == level
                    && areas
                        .iter()
                        .any(|a| kentos_geometry_core::geom::region::inside_area(a, p))
            })
        };
        let pick = match (at(left), at(right)) {
            (Some(a), Some(b)) if f.form == OpeningForm::Door => match (a.2, b.2) {
                (true, false) => b,
                (false, true) => a,
                _ if f.side == OpeningSide::Right => b,
                _ => a,
            },
            (Some(a), _) => a,
            (_, Some(b)) => b,
            (None, None) => return None,
        };
        Some(pick.1)
    }

    /// Every room's doors and windows (docs/adr/8038), by the room's slot; a room no door or window serves is left out.
    pub fn room_openings(&self) -> HashMap<Slot, RoomOpenings> {
        let out = self.option_out();
        let rooms: Vec<Room> = self
            .entities()
            .filter_map(|e| match e {
                Entity::Space(s) if !s.space.void && !out.contains(&Slot(e.base().id)) => Some((
                    e.own_level().map(str::to_owned),
                    Slot(e.base().id),
                    crate::room_names::passage(s.space.name.as_deref(), s.space.usage.as_deref()),
                    core_areas(&s.space.parts),
                )),
                _ => None,
            })
            .collect();
        let mut doors: HashMap<Slot, BTreeMap<String, usize>> = HashMap::new();
        let mut windows: HashMap<Slot, BTreeMap<String, usize>> = HashMap::new();
        for e in self.entities() {
            let Entity::Opening(o) = e else {
                continue;
            };
            let f = &o.opening;
            if !matches!(f.form, OpeningForm::Door | OpeningForm::Window)
                || out.contains(&Slot(e.base().id))
            {
                continue;
            }
            let Some(room) = self.served_room(f, &rooms) else {
                continue;
            };
            let to = if f.form == OpeningForm::Door {
                &mut doors
            } else {
                &mut windows
            };
            *to.entry(room)
                .or_default()
                .entry(self.joinery_label(f))
                .or_default() += 1;
        }
        let mut all: HashMap<Slot, RoomOpenings> = HashMap::new();
        for (room, codes) in doors {
            all.entry(room).or_default().doors = said(&codes);
        }
        for (room, codes) in windows {
            all.entry(room).or_default().windows = said(&codes);
        }
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_room_s_codes_are_said_in_order_with_their_counts() {
        let codes: BTreeMap<String, usize> = [
            ("P02".to_owned(), 1),
            ("P01".to_owned(), 2),
            ("90/210".to_owned(), 1),
        ]
        .into_iter()
        .collect();
        assert_eq!(said(&codes), "90/210, P01 (2), P02");
        assert_eq!(said(&BTreeMap::new()), "");
    }
}
