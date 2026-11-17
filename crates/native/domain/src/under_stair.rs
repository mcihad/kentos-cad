//! Merdiven altı (docs/adr/8080; Planlı Alanlar İmar Yönetmeliği 4/1-ğ-3, 31/6): the places under each stair lower than
//! 1.80 m over the floor it starts from, from the stair's body as the section model draws it
//! (`kentos_geometry_core::geom::stair_body`). 4/1-ğ-3 leaves them out of a unit's net area; 31/6 closes them off.
//!
//! - **Waist** (`Document::stair_waist`): the structure of the slab the stair arrives at (one holding its walking line's
//!   end, its top within 0.5 m of where the stair arrives), else of the one it starts from; none when no slab is there.
//!   Without a waist only what is sure counts: the places under treads and landings lower than 1.80 m.
//! - **Floor:** its storey's (its level's elevation; ±0.00 without a level).

use kentos_contracts::{Entity, StairFields};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::stair_body::{UNDER_STAIR_CLEAR, low_under, stair_body};
use kentos_geometry_core::vec2::Vec2;

use crate::Document;

/// A stair's places lower than 1.80 m under it: its persistent id, its storey, the places (solid ones and spaces
/// together) and the spaces alone (the body over its floor there), and whether its waist is known.
#[derive(Clone, Debug, PartialEq)]
pub struct UnderStair {
    pub uid: String,
    pub level: Option<String>,
    pub low: Vec<Area>,
    pub open: Vec<Area>,
    pub waist: bool,
}

/// `ring` as an area: counter-clockwise, no holes.
fn area_of(mut ring: Vec<Vec2>) -> Area {
    let n = ring.len();
    let twice: f64 = (0..n)
        .map(|i| ring[i].x * ring[(i + 1) % n].y - ring[(i + 1) % n].x * ring[i].y)
        .sum();
    if twice < 0.0 {
        ring.reverse();
    }
    Area {
        outer: Ring {
            pts: ring,
            bulges: None,
        },
        holes: Vec::new(),
    }
}

/// Whether `at` lies in `rings` (even-odd).
fn holds(rings: &[Vec<Vec2>], at: Vec2) -> bool {
    let mut odd = false;
    for r in rings {
        let n = r.len();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            if (a.y > at.y) != (b.y > at.y) && at.x < a.x + (at.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                odd = !odd;
            }
        }
    }
    odd
}

impl Document {
    /// The elevation of the floor of `level` (its storey's; 0 without one).
    fn level_floor(&self, level: Option<&str>) -> f64 {
        level
            .and_then(|id| self.settings().arch.as_ref()?.levels.iter().find(|l| l.id == id))
            .map_or(0.0, |l| l.elevation)
    }

    /// A stair's waist (docs/adr/8042 §3, 8080): the structure of the building's slab (no covering) it arrives at, one
    /// holding its walking line's end whose top is within 0.5 m of where it arrives, else of the one it starts from;
    /// none when no slab is there.
    pub fn stair_waist(&self, s: &StairFields) -> Option<f64> {
        let (Some(start), Some(end)) = (s.pts.first(), s.pts.last()) else {
            return None;
        };
        let base = self.level_floor(s.arch.level.as_deref());
        let arrive = base + s.riser_height * f64::from(s.risers);
        let slabs: Vec<(Vec<Vec<Vec2>>, f64, f64)> = self
            .entities()
            .filter_map(|e| match e {
                Entity::Slab(x) if x.slab.covering.is_none() => Some(&x.slab),
                _ => None,
            })
            .map(|x| {
                let rings = x
                    .parts
                    .iter()
                    .flat_map(|part| std::iter::once(&part.outer).chain(&part.holes))
                    .map(|r| r.pts.iter().map(|q| Vec2::new(q.x, q.y)).collect())
                    .collect();
                (rings, self.level_floor(x.arch.level.as_deref()) + x.offset, x.thickness)
            })
            .collect();
        let at = |p: &kentos_contracts::Vec2| Vec2::new(p.x, p.y);
        slabs
            .iter()
            .find(|(r, top, _)| holds(r, at(end)) && (top - arrive).abs() <= 0.5)
            .or_else(|| slabs.iter().find(|(r, top, _)| holds(r, at(start)) && (top - base).abs() <= 0.5))
            .map(|(_, _, t)| *t)
    }

    /// Each stair's places lower than 1.80 m under it (PAİY 4/1-ğ-3, 31/6), in the drawing's order; a stair with none
    /// is left out.
    pub fn under_stairs(&self) -> Vec<UnderStair> {
        let mut out = Vec::new();
        for (uid, e) in self.entities_with_uids() {
            let Entity::Stair(s) = e else { continue };
            let kentos_geometry_core::entity::Shape::Stair { pts, stair: spec } =
                kentos_formats::arch_elements::core_stair(&s.stair)
            else {
                continue;
            };
            let base = self.level_floor(s.stair.arch.level.as_deref());
            let waist = self.stair_waist(&s.stair);
            let parts = stair_body(&pts, &spec, base, waist);
            let low = low_under(&parts, base, UNDER_STAIR_CLEAR, waist.is_some());
            if low.is_empty() {
                continue;
            }
            let open: Vec<Area> = low
                .iter()
                .filter(|(_, o)| *o)
                .map(|(r, _)| area_of(r.clone()))
                .collect();
            out.push(UnderStair {
                uid: uid.to_string(),
                level: s.stair.arch.level.clone(),
                low: low.into_iter().map(|(r, _)| area_of(r)).collect(),
                open,
                waist: waist.is_some(),
            });
        }
        out
    }
}

impl Document {
    /// The rooms on a stair's storey its spaces lower than 1.80 m lie in (31/6, docs/adr/8080): each room's persistent
    /// id and the area of those spaces in its net outline, m², in the drawing's order; voids left out.
    pub fn under_stair_rooms(&self, u: &UnderStair) -> Vec<(String, f64)> {
        if u.open.is_empty() {
            return Vec::new();
        }
        let open = kentos_geometry_core::geom::region::union_areas(&u.open);
        let mut search = crate::space_follow::SpaceSearch::new(self);
        let mut out = Vec::new();
        for (uid, e) in self.entities_with_uids() {
            let Entity::Space(s) = e else { continue };
            let x = &s.space;
            if x.void || x.arch.level != u.level {
                continue;
            }
            let outline = if x.seed.is_some() && x.method != kentos_contracts::SpaceMethod::Net {
                let mut net = x.clone();
                net.method = kentos_contracts::SpaceMethod::Net;
                let found = search.found_preserving_legacy(&net);
                if found.broken.is_none() && !found.parts.is_empty() {
                    crate::arch_values::core_areas(&found.parts)
                } else {
                    crate::arch_values::core_areas(&x.parts)
                }
            } else {
                crate::arch_values::core_areas(&x.parts)
            };
            let a = taken(&outline, &open);
            if a > 1e-6 {
                out.push((uid.to_string(), a));
            }
        }
        out
    }
}

/// The places under the stairs on `level` lower than 1.80 m, together.
pub(crate) fn low_on(unders: &[UnderStair], level: &Option<String>) -> Vec<Area> {
    let low: Vec<Area> = unders
        .iter()
        .filter(|u| u.level == *level)
        .flat_map(|u| u.low.iter().cloned())
        .collect();
    if low.is_empty() {
        low
    } else {
        kentos_geometry_core::geom::region::union_areas(&low)
    }
}

/// How much of a room's net outline `net` lies over `low` (the places under its storey's stairs lower than 1.80 m), m².
pub(crate) fn taken(net: &[Area], low: &[Area]) -> f64 {
    if net.is_empty() || low.is_empty() {
        return 0.0;
    }
    let both = kentos_geometry_core::geom::region::intersect_area_sets(&[net.to_vec(), low.to_vec()]);
    kentos_geometry_core::geom::space::area(&both)
}
