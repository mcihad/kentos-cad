//! What separates two rooms (docs/adr/8081; Binaların Yangından Korunması Hakkında Yönetmelik 4/tt, 24): along the
//! boundary two rooms of a storey share, the walls, the openings and the stretches with neither (a room separator, a
//! gap), each with how long it runs. A fire compartment is closed on every side by elements of at least 60 minutes:
//! the standards package reads the elements' declared classes and joins the rooms nothing such separates.
//!
//! - **The shared boundary** is found from each room's outline to its walls' axes (`SpaceMethod::Centerline`; a room
//!   drawn by its own outline, its own): from a point of one room's outline, walking outwards in steps of 2 cm for at
//!   most 1 m, the first point in another room of the storey makes them neighbours there (two rooms drawn to their
//!   walls' faces do not touch: the walk crosses the wall).
//! - **Sampled** every 25 cm along each edge, at least once an edge; never within 30 cm of an outline's corner, where
//!   walls join and a wall's own body (its faces' band, unjoined) does not reach.
//! - **The element** halfway along that walk: an opening through its wall whose outline holds it (a door, a window, a gap; a niche
//!   or a flue is not one), else a wall whose body holds it, else nothing (an open stretch). A wall or an opening of
//!   another storey is not looked at.
//! - Each shared boundary is counted once, from its room whose persistent id comes first.

use std::collections::BTreeMap;

use kentos_contracts::{Entity, SpaceMethod};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::inside_area;
use kentos_geometry_core::vec2::Vec2;

use crate::Document;
use crate::arch_values::core_areas;
use crate::space_follow::SpaceSearch;

/// The step along a shared boundary, metres.
const STEP: f64 = 0.25;
/// How far from an outline's corner no point is taken, metres.
const CORNER: f64 = 0.30;
/// The step outwards to find the room on the other side, metres.
const ACROSS: f64 = 0.02;
/// How far outwards the room on the other side is looked for, metres: wider walls separate rooms this does not see.
const REACH: f64 = 1.0;

/// What separates two rooms of a storey, metres along their shared boundary.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Separation {
    pub level: Option<String>,
    /// The two rooms by their persistent ids, the first the smaller.
    pub rooms: [String; 2],
    /// The walls along it, by persistent id, each with its length there.
    pub walls: Vec<(String, f64)>,
    /// The openings along it (a door, a window, a gap), by persistent id, each with its length there.
    pub openings: Vec<(String, f64)>,
    /// Its length with neither a wall nor an opening.
    pub open: f64,
}

/// The element at a point of a shared boundary.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum At {
    Wall(String),
    Opening(String),
    Open,
}

/// Whether `ring` holds `p` (even-odd).
fn holds(ring: &[Vec2], p: Vec2) -> bool {
    let n = ring.len();
    let mut odd = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            odd = !odd;
        }
    }
    odd
}

impl Document {
    /// What separates each two rooms of a storey that share a boundary (docs/adr/8081), in their persistent ids'
    /// order; voids and outdoor rooms are no rooms here.
    pub fn separations(&self) -> Vec<Separation> {
        let program = self.program();
        let mut search = SpaceSearch::new(self);
        // The rooms: persistent id, storey, outline to the walls' axes.
        let mut rooms: Vec<(String, Option<String>, Vec<Area>)> = Vec::new();
        for (uid, e) in self.entities_with_uids() {
            let Entity::Space(s) = e else { continue };
            let x = &s.space;
            let outdoor = x
                .usage
                .as_deref()
                .and_then(|u| program.brief(u))
                .is_some_and(|b| b.outdoor);
            if x.void || outdoor {
                continue;
            }
            let outline = if x.seed.is_some() && x.method != SpaceMethod::Centerline {
                let mut axis = x.clone();
                axis.method = SpaceMethod::Centerline;
                let found = search.found_preserving_legacy(&axis);
                if found.broken.is_none() && !found.parts.is_empty() {
                    core_areas(&found.parts)
                } else {
                    core_areas(&x.parts)
                }
            } else {
                core_areas(&x.parts)
            };
            rooms.push((uid.to_string(), x.arch.level.clone(), outline));
        }
        // The walls' bodies and the openings' outlines, each with its storey.
        let mut walls: Vec<(String, Option<String>, Vec<Vec<Vec2>>)> = Vec::new();
        let mut openings: Vec<(String, Option<String>, Vec<Vec2>)> = Vec::new();
        for (uid, e) in self.entities_with_uids() {
            match e {
                Entity::Wall(w) => {
                    let kentos_geometry_core::entity::Shape::Wall { pts, bulges, wall } =
                        kentos_formats::walls::core_wall(&w.wall)
                    else {
                        continue;
                    };
                    let Ok((outer, holes)) = kentos_geometry_core::geom::wall::body(&pts, bulges.as_deref(), &wall)
                    else {
                        continue;
                    };
                    let mut rings = vec![outer];
                    rings.extend(holes.unwrap_or_default());
                    walls.push((uid.to_string(), w.wall.arch.level.clone(), rings));
                }
                // A niche or a flue does not pass through its wall: the wall still separates there.
                Entity::Opening(o)
                    if o.opening.place.len() >= 3
                        && !matches!(o.opening.form, kentos_contracts::OpeningForm::Niche | kentos_contracts::OpeningForm::Flue) =>
                {
                    let level = self.wall_of(&o.opening.host).and_then(|w| w.arch.level);
                    let place = o.opening.place.iter().map(|q| Vec2::new(q.x, q.y)).collect();
                    openings.push((uid.to_string(), level, place));
                }
                _ => {}
            }
        }
        let mut found: BTreeMap<(String, String), (Option<String>, BTreeMap<At, f64>)> = BTreeMap::new();
        for (uid, level, outline) in &rooms {
            for area in outline {
                for ring in std::iter::once(&area.outer.pts).chain(area.holes.iter().map(|h| &h.pts)) {
                    let n = ring.len();
                    for i in 0..n {
                        let (a, b) = (ring[i], ring[(i + 1) % n]);
                        let len = (b.x - a.x).hypot(b.y - a.y);
                        if len <= 2.0 * CORNER {
                            continue;
                        }
                        let (dx, dy) = ((b.x - a.x) / len, (b.y - a.y) / len);
                        // Away from the room: its outer ring counter-clockwise, its holes clockwise, the right side.
                        let out = Vec2::new(dy, -dx);
                        let span = len - 2.0 * CORNER;
                        let k = (span / STEP).ceil().max(1.0) as usize;
                        let piece = span / k as f64;
                        for j in 0..k {
                            let s = CORNER + piece * (j as f64 + 0.5);
                            let p = Vec2::new(a.x + dx * s, a.y + dy * s);
                            // Outwards until another room of the storey (never its own).
                            let steps = (REACH / ACROSS).round() as usize;
                            let Some((other, t)) = (1..=steps).find_map(|i| {
                                let t = ACROSS * i as f64;
                                let q = Vec2::new(p.x + out.x * t, p.y + out.y * t);
                                if outline.iter().any(|x| inside_area(x, q)) {
                                    return Some(None);
                                }
                                rooms
                                    .iter()
                                    .find(|(o, l, areas)| o != uid && l == level && areas.iter().any(|x| inside_area(x, q)))
                                    .map(|(o, ..)| Some((o, t)))
                            })
                            .flatten() else {
                                continue;
                            };
                            if other < uid {
                                continue;
                            }
                            let m = Vec2::new(p.x + out.x * t / 2.0, p.y + out.y * t / 2.0);
                            let at = if let Some((o, ..)) =
                                openings.iter().find(|(_, l, place)| l == level && holds(place, m))
                            {
                                At::Opening(o.clone())
                            } else if let Some((w, ..)) =
                                walls.iter().find(|(_, l, rings)| l == level && rings.iter().filter(|r| holds(r, m)).count() % 2 == 1)
                            {
                                At::Wall(w.clone())
                            } else {
                                At::Open
                            };
                            let entry = found
                                .entry((uid.clone(), other.clone()))
                                .or_insert_with(|| (level.clone(), BTreeMap::new()));
                            *entry.1.entry(at).or_insert(0.0) += piece;
                        }
                    }
                }
            }
        }
        found
            .into_iter()
            .map(|((a, b), (level, by))| {
                let mut s = Separation {
                    level,
                    rooms: [a, b],
                    ..Separation::default()
                };
                for (at, length) in by {
                    match at {
                        At::Wall(w) => s.walls.push((w, length)),
                        At::Opening(o) => s.openings.push((o, length)),
                        At::Open => s.open += length,
                    }
                }
                s
            })
            .collect()
    }
}
