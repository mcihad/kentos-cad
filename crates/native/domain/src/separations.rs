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
//!   walls join and a wall's own body (its faces' band, unjoined) does not reach. Where two samples differ the change
//!   is found by halving to a tenth of a millimetre (docs/adr/8087): a door's length is its width.
//! - **The element** halfway along that walk: an opening through its wall whose outline holds it (a door, a window, a gap; a niche
//!   or a flue is not one), else a wall whose body holds it, else nothing (an open stretch). A wall or an opening of
//!   another storey is not looked at.
//! - Each shared boundary is counted once, from its room whose persistent id comes first.
//! - **A shaft's enclosure** (docs/adr/8087; BYKHY 25/3, 41/9, 62/2): the same elements met walking out of an elevator's
//!   or a service shaft's outline (`Document::shaft_enclosures`); a shaft's sides are short, its corners are kept only
//!   10 cm clear (a quarter of a shorter side): the walls round it hold its corners.
//! - **A shaft through the storeys** (docs/adr/8135; BYKHY 62/4 asks a lift shaft's vent once for the whole shaft): shafts
//!   of one kind on consecutive storeys, the middle of one inside the other, are one shaft, named by its lowest room's
//!   persistent id (the storeys' order; a shaft on no storey is its own). Its area is its outline's.

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

/// How close a change between two elements along an edge is found, metres.
const SHARP: f64 = 1e-4;
/// How far outwards from a shaft's outline its enclosing wall is looked for, metres.
const SHAFT_REACH: f64 = 0.8;
/// How far from a shaft's corner no point is taken, metres; a quarter of a shorter side.
const SHAFT_CORNER: f64 = 0.10;

/// What encloses a shaft (docs/adr/8087): its walls and openings by persistent id, each with its length round it, and
/// the length with neither, metres.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Enclosure {
    pub uid: String,
    pub level: Option<String>,
    /// `elevator` or `service`.
    pub kind: &'static str,
    /// Its outline's area, m².
    pub area: f64,
    /// The shaft through the storeys it belongs to: its lowest room's persistent id.
    pub stack: String,
    pub walls: Vec<(String, f64)>,
    pub openings: Vec<(String, f64)>,
    pub open: f64,
}

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

/// The element at `m` on storey `level`: an opening whose outline holds it, else a wall whose body holds it, else none.
/// Where two hold it (joined walls overlap at their joint) the smaller persistent id: the drawing's order does not
/// matter.
#[allow(clippy::type_complexity)]
fn element(
    walls: &[(String, Option<String>, Vec<Vec<Vec2>>)],
    openings: &[(String, Option<String>, Vec<Vec2>)],
    level: &Option<String>,
    m: Vec2,
) -> At {
    if let Some((o, ..)) =
        openings.iter().filter(|(_, l, place)| l == level && holds(place, m)).min_by(|a, b| a.0.cmp(&b.0))
    {
        return At::Opening(o.clone());
    }
    walls
        .iter()
        .filter(|(_, l, rings)| l == level && rings.iter().filter(|r| holds(r, m)).count() % 2 == 1)
        .min_by(|a, b| a.0.cmp(&b.0))
        .map_or(At::Open, |(w, ..)| At::Wall(w.clone()))
}

/// An edge from `s0` to `s1` (metres along it) cut where `read` changes: read every `STEP` (once at least), and between
/// two readings that differ the change found by halving to a tenth of a millimetre (docs/adr/8087: a door's length is
/// its width, not a count of steps; the facts round to a millimetre, the same however the outline begins); each reading
/// with its length, in order.
fn spans<K: Clone + PartialEq>(s0: f64, s1: f64, read: &mut dyn FnMut(f64) -> K) -> Vec<(K, f64)> {
    let span = s1 - s0;
    let k = (span / STEP).ceil().max(1.0) as usize;
    let piece = span / k as f64;
    let samples: Vec<(f64, K)> = (0..k)
        .map(|j| {
            let s = s0 + piece * (j as f64 + 0.5);
            (s, read(s))
        })
        .collect();
    let mut out = Vec::new();
    let mut from = s0;
    for j in 0..k {
        let to = if j + 1 == k {
            s1
        } else if samples[j].1 == samples[j + 1].1 {
            continue;
        } else {
            let (mut lo, mut hi) = (samples[j].0, samples[j + 1].0);
            while hi - lo > SHARP {
                let mid = (lo + hi) / 2.0;
                if read(mid) == samples[j].1 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            (lo + hi) / 2.0
        };
        out.push((samples[j].1.clone(), to - from));
        from = to;
    }
    out
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
        let (walls, openings) = self.bodies();
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
                        let mut read = |s: f64| -> Option<(String, At)> {
                            let p = Vec2::new(a.x + dx * s, a.y + dy * s);
                            // Outwards until another room of the storey (never its own).
                            let steps = (REACH / ACROSS).round() as usize;
                            let (other, t) = (1..=steps)
                                .find_map(|i| {
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
                                .flatten()?;
                            if other < uid {
                                return None;
                            }
                            let m = Vec2::new(p.x + out.x * t / 2.0, p.y + out.y * t / 2.0);
                            Some((other.clone(), element(&walls, &openings, level, m)))
                        };
                        for (key, length) in spans(CORNER, len - CORNER, &mut read) {
                            let Some((other, at)) = key else { continue };
                            let entry = found
                                .entry((uid.clone(), other))
                                .or_insert_with(|| (level.clone(), BTreeMap::new()));
                            *entry.1.entry(at).or_insert(0.0) += length;
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

    /// The walls' bodies and the openings' outlines (a niche or a flue is none: it does not pass through its wall), each
    /// with its persistent id and storey.
    #[allow(clippy::type_complexity)]
    fn bodies(&self) -> (Vec<(String, Option<String>, Vec<Vec<Vec2>>)>, Vec<(String, Option<String>, Vec<Vec2>)>) {
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
                    let mut place: Vec<Vec2> = o.opening.place.iter().map(|q| Vec2::new(q.x, q.y)).collect();
                    // Its jambs are written start-left, start-right, end-left, end-right: as a ring the end jamb's come
                    // back, else the ring crosses itself and holds only the jambs' corners near the faces.
                    if place.len() == 4 {
                        place.swap(2, 3);
                    }
                    openings.push((uid.to_string(), level, place));
                }
                _ => {}
            }
        }
        (walls, openings)
    }

    /// What encloses each shaft (docs/adr/8087; BYKHY 25/3, 62/2): an elevator's shaft (a void space named Asansör, not
    /// its lobby) or a service shaft or flue (Şaft, Tesisat, Baca, an air shaft; a light well is open to the sky and none):
    /// from its outline's points every 25 cm (never within 10 cm of a corner, a quarter of a shorter side), walking
    /// outwards in steps of 2 cm for at most 80 cm, the first opening or wall body met, or nothing (an open stretch), each
    /// with how long it runs. Each with its area and the shaft through the storeys it belongs to (docs/adr/8135).
    pub fn shaft_enclosures(&self) -> Vec<Enclosure> {
        let (walls, openings) = self.bodies();
        let mut out = Vec::new();
        // Each shaft's outline and middle, for the shafts through the storeys.
        let mut shapes: Vec<(Vec<Area>, Option<Vec2>)> = Vec::new();
        for (uid, e) in self.entities_with_uids() {
            let Entity::Space(s) = e else { continue };
            let x = &s.space;
            if !x.void {
                continue;
            }
            let (name, usage) = (x.name.as_deref(), x.usage.as_deref());
            let kind = match crate::room_names::shaft_of(name, usage) {
                Some(crate::room_names::ShaftKind::Elevator) => "elevator",
                Some(crate::room_names::ShaftKind::LightWell) => continue,
                Some(crate::room_names::ShaftKind::AirShaft) => "service",
                None if crate::room_names::shaft(name, usage) => "service",
                None => continue,
            };
            let level = x.arch.level.clone();
            let mut by: BTreeMap<At, f64> = BTreeMap::new();
            for area in core_areas(&x.parts) {
                let ring = &area.outer.pts;
                let n = ring.len();
                let twice: f64 = (0..n).map(|i| ring[i].x * ring[(i + 1) % n].y - ring[(i + 1) % n].x * ring[i].y).sum();
                for i in 0..n {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    let len = (b.x - a.x).hypot(b.y - a.y);
                    let corner = (len / 4.0).min(SHAFT_CORNER);
                    if len - 2.0 * corner <= ACROSS {
                        continue;
                    }
                    let (dx, dy) = ((b.x - a.x) / len, (b.y - a.y) / len);
                    // Away from the shaft: right of a counter-clockwise outline, left of a clockwise one.
                    let out_n = if twice > 0.0 { Vec2::new(dy, -dx) } else { Vec2::new(-dy, dx) };
                    let mut read = |s: f64| -> At {
                        let p = Vec2::new(a.x + dx * s, a.y + dy * s);
                        let steps = (SHAFT_REACH / ACROSS).round() as usize;
                        // From the first step: on the outline itself a wall's face and its opening's edge meet.
                        (1..=steps)
                            .map(|i| {
                                let t = ACROSS * i as f64;
                                element(&walls, &openings, &level, Vec2::new(p.x + out_n.x * t, p.y + out_n.y * t))
                            })
                            .find(|at| *at != At::Open)
                            .unwrap_or(At::Open)
                    };
                    for (at, length) in spans(corner, len - corner, &mut read) {
                        *by.entry(at).or_insert(0.0) += length;
                    }
                }
            }
            let areas = core_areas(&x.parts);
            let area = kentos_geometry_core::geom::space::area(&areas);
            let middle = kentos_geometry_core::geom::space::label_at(&areas);
            shapes.push((areas, middle));
            let mut enclosure =
                Enclosure { uid: uid.to_string(), level, kind, area, stack: uid.to_string(), ..Enclosure::default() };
            for (at, length) in by {
                match at {
                    At::Wall(w) => enclosure.walls.push((w, length)),
                    At::Opening(o) => enclosure.openings.push((o, length)),
                    At::Open => enclosure.open += length,
                }
            }
            out.push(enclosure);
        }
        self.stack_shafts(&mut out, &shapes);
        out
    }

    /// Names each shaft's stack (docs/adr/8135): shafts of one kind on consecutive storeys, the middle of one inside the
    /// other's outline, are one shaft through the storeys, named by its lowest room's persistent id.
    fn stack_shafts(&self, out: &mut [Enclosure], shapes: &[(Vec<Area>, Option<Vec2>)]) {
        let mut levels: Vec<(String, f64)> = self
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.iter().map(|l| (l.id.clone(), l.elevation)).collect())
            .unwrap_or_default();
        levels.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        let index = |l: &Option<String>| l.as_deref().and_then(|id| levels.iter().position(|(x, _)| x == id));
        let n = out.len();
        let mut root: Vec<usize> = (0..n).collect();
        fn find(root: &mut [usize], mut i: usize) -> usize {
            while root[i] != i {
                root[i] = root[root[i]];
                i = root[i];
            }
            i
        }
        let holds = |i: usize, j: usize| {
            shapes[j].1.is_some_and(|m| shapes[i].0.iter().any(|a| inside_area(a, m)))
        };
        for i in 0..n {
            let Some(at) = index(&out[i].level) else { continue };
            for j in 0..n {
                if out[j].kind != out[i].kind || index(&out[j].level) != Some(at + 1) {
                    continue;
                }
                if holds(i, j) || holds(j, i) {
                    let (a, b) = (find(&mut root, i), find(&mut root, j));
                    if a != b {
                        root[a.max(b)] = a.min(b);
                    }
                }
            }
        }
        let rep: Vec<usize> = (0..n).map(|i| find(&mut root, i)).collect();
        let lowest: Vec<String> = (0..n)
            .map(|i| {
                (0..n)
                    .filter(|&j| rep[j] == rep[i])
                    .min_by(|&a, &b| index(&out[a].level).cmp(&index(&out[b].level)).then_with(|| out[a].uid.cmp(&out[b].uid)))
                    .map_or_else(|| out[i].uid.clone(), |j| out[j].uid.clone())
            })
            .collect();
        for (x, s) in out.iter_mut().zip(lowest) {
            x.stack = s;
        }
    }
}
