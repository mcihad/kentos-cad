//! What joins two storeys' rooms (docs/adr/8086; Binaların Yangından Korunması Hakkında Yönetmelik 4/g, 24/4, 24/5,
//! 26/1): every floor is a fire wall, so rooms of two storeys share a fire compartment only through an opening between
//! them. The standards package joins the compartments across them and reads the atriums.
//!
//! - **A hole:** a hole of a floor slab (role Floor, no covering) of a storey that no other slab of the storey covers (a
//!   lowered wet room's) and no shaft of the storey holds (an elevator's, a light well, an air or service shaft, a flue:
//!   `room_names::shaft`). Its rooms below: the rooms of the storey under it its middle and its sides' points (5 cm inside)
//!   fall in. Its rooms above: the rooms of its storey holding its middle, and those met walking outwards from its sides'
//!   points in steps of 2 cm for at most 1 m (as the separations walk, docs/adr/8081).
//! - **A void space** of a storey that is no shaft (Galeri boşluğu, Atrium): read as a hole.
//! - **A stair** of a storey: the room of its storey holding its foot (its walking line's first point) and the room of
//!   the storey over it holding its head (its last point), each looked for up to 1 m on along the line.
//! - **A stair well:** a hole or a void a stair of the storey under it arrives through (its head or a point 15 cm before
//!   it inside): BYKHY 4/g leaves stair wells out of the atriums.
//! - **A stack:** openings over one another on consecutive storeys (the middle of one inside the other) are one volume,
//!   named by its lowest opening's persistent id: one atrium.
//!
//! Points are sampled every 50 cm along each side, at least once a side; an arc's side is read as its chord.

use kentos_contracts::{Entity, SlabRole};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region::inside_area;
use kentos_geometry_core::vec2::Vec2;

use crate::Document;
use crate::arch_values::core_areas;

/// The step along a side, metres.
const STEP: f64 = 0.5;
/// How far inside a hole its sides' points are taken, metres.
const INSET: f64 = 0.05;
/// The step outwards to find a room beside a hole, metres.
const ACROSS: f64 = 0.02;
/// How far outwards a room beside a hole, or on along a stair's line, is looked for, metres.
const REACH: f64 = 1.0;
/// How far before a stair's head a point inside a hole makes it a stair well, metres.
const BEFORE_HEAD: f64 = 0.15;

/// An opening between two storeys.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VerticalLink {
    /// The slab's (a hole's), the void space's or the stair's persistent id.
    pub uid: String,
    /// `hole`, `void` or `stair`.
    pub kind: &'static str,
    /// The storey under it and the storey it opens into.
    pub below: Option<String>,
    pub above: Option<String>,
    /// The rooms it joins, by persistent id, each storey's in the drawing's order.
    pub rooms_below: Vec<String>,
    pub rooms_above: Vec<String>,
    /// Its area (a hole's, a void's), m²; 0 for a stair.
    pub area: f64,
    /// A stair of the storey under it arrives through it.
    pub stair_well: bool,
    /// The lowest opening of the stack it belongs to; a stair's own id.
    pub stack: String,
}

/// A room of a storey: its id and outline.
struct Room {
    uid: String,
    level: Option<String>,
    areas: Vec<Area>,
}

/// An opening of a storey before its rooms are found: its id, kind, storey, outline and middle.
struct Hole {
    uid: String,
    kind: &'static str,
    level: Option<String>,
    area: Area,
    middle: Vec2,
}

fn inside(areas: &[Area], p: Vec2) -> bool {
    areas.iter().any(|a| inside_area(a, p))
}

/// A ring's points as straight sides, its closing point dropped.
fn corners(r: &Ring) -> Vec<Vec2> {
    let mut pts = r.pts.clone();
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    pts
}

/// Each side's points every [`STEP`] (at least its middle) with the unit normal pointing out of the ring.
fn side_points(r: &Ring) -> Vec<(Vec2, Vec2)> {
    let pts = corners(r);
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    let twice: f64 = (0..n)
        .map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y)
        .sum();
    let ccw = twice > 0.0;
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let l = kentos_num::hypot(b.x - a.x, b.y - a.y);
        if l <= 1e-9 {
            continue;
        }
        let (ux, uy) = ((b.x - a.x) / l, (b.y - a.y) / l);
        // Out of a counter-clockwise ring is to the right of its sides.
        let out_n = if ccw {
            Vec2::new(uy, -ux)
        } else {
            Vec2::new(-uy, ux)
        };
        let k = ((l / STEP).floor() as usize).max(1);
        for j in 0..k {
            let t = (j as f64 + 0.5) * l / k as f64;
            out.push((Vec2::new(a.x + ux * t, a.y + uy * t), out_n));
        }
    }
    out
}

fn push_once(list: &mut Vec<String>, uid: &str) {
    if !list.iter().any(|u| u == uid) {
        list.push(uid.to_owned());
    }
}

impl Document {
    /// The openings between storeys, lowest storey first (docs/adr/8086).
    pub fn vertical_links(&self) -> Vec<VerticalLink> {
        let mut levels: Vec<(String, f64)> = self
            .settings()
            .arch
            .as_ref()
            .map(|a| {
                a.levels
                    .iter()
                    .map(|l| (l.id.clone(), l.elevation))
                    .collect()
            })
            .unwrap_or_default();
        levels.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        let index = |l: &Option<String>| {
            l.as_deref()
                .and_then(|id| levels.iter().position(|(x, _)| x == id))
        };
        let below = |l: &Option<String>| {
            index(l)
                .and_then(|i| i.checked_sub(1))
                .map(|i| levels[i].0.clone())
        };
        let above = |l: &Option<String>| {
            index(l)
                .and_then(|i| levels.get(i + 1))
                .map(|x| x.0.clone())
        };
        let mut rooms: Vec<Room> = Vec::new();
        let mut shafts: Vec<(Option<String>, Vec<Area>)> = Vec::new();
        let mut holes: Vec<Hole> = Vec::new();
        let mut slabs: Vec<(String, Option<String>, Vec<Area>)> = Vec::new();
        let mut stairs: Vec<(String, Option<String>, Vec<Vec2>)> = Vec::new();
        for (uid, e) in self.entities_with_uids() {
            let level = e.own_level().map(str::to_owned);
            match e {
                Entity::Space(s) => {
                    let x = &s.space;
                    let areas = core_areas(&x.parts);
                    if !x.void {
                        rooms.push(Room {
                            uid: uid.to_string(),
                            level,
                            areas,
                        });
                    } else if crate::room_names::shaft(x.name.as_deref(), x.usage.as_deref()) {
                        shafts.push((level, areas));
                    } else {
                        for a in areas {
                            let middle = kentos_geometry_core::geom::space::label_at(
                                std::slice::from_ref(&a),
                            );
                            if let Some(middle) = middle {
                                holes.push(Hole {
                                    uid: uid.to_string(),
                                    kind: "void",
                                    level: level.clone(),
                                    area: a,
                                    middle,
                                });
                            }
                        }
                    }
                }
                Entity::Slab(s) if s.slab.covering.is_none() && s.slab.role == SlabRole::Floor => {
                    let areas = core_areas(&s.slab.parts);
                    for a in &areas {
                        for h in &a.holes {
                            let hole = Area {
                                outer: h.clone(),
                                holes: Vec::new(),
                            };
                            if let Some(middle) = kentos_geometry_core::geom::space::label_at(
                                std::slice::from_ref(&hole),
                            ) {
                                holes.push(Hole {
                                    uid: uid.to_string(),
                                    kind: "hole",
                                    level: level.clone(),
                                    area: hole,
                                    middle,
                                });
                            }
                        }
                    }
                    slabs.push((uid.to_string(), level, areas));
                }
                Entity::Stair(s) => stairs.push((
                    uid.to_string(),
                    level,
                    s.stair.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect(),
                )),
                _ => {}
            }
        }
        // A hole another slab of its storey covers, or a shaft of its storey holds, is no opening.
        holes.retain(|h| {
            h.kind == "void"
                || !(slabs
                    .iter()
                    .any(|(u, l, a)| *u != h.uid && *l == h.level && inside(a, h.middle))
                    || shafts
                        .iter()
                        .any(|(l, a)| *l == h.level && inside(a, h.middle)))
        });
        let room_at = |level: &Option<String>, p: Vec2| -> Option<&Room> {
            rooms
                .iter()
                .find(|r| r.level == *level && inside(&r.areas, p))
        };
        let mut out: Vec<VerticalLink> = Vec::new();
        for h in &holes {
            let low = below(&h.level);
            let mut link = VerticalLink {
                uid: h.uid.clone(),
                kind: h.kind,
                below: low.clone(),
                above: h.level.clone(),
                area: kentos_geometry_core::geom::space::area(std::slice::from_ref(&h.area)),
                stack: h.uid.clone(),
                ..VerticalLink::default()
            };
            let sides = side_points(&h.area.outer);
            if let Some(low) = &low {
                let lower = Some(low.clone());
                for p in std::iter::once(h.middle).chain(
                    sides
                        .iter()
                        .map(|(p, n)| Vec2::new(p.x - n.x * INSET, p.y - n.y * INSET)),
                ) {
                    if let Some(r) = room_at(&lower, p) {
                        push_once(&mut link.rooms_below, &r.uid);
                    }
                }
            }
            if let Some(r) = room_at(&h.level, h.middle) {
                push_once(&mut link.rooms_above, &r.uid);
            }
            for (p, n) in &sides {
                let steps = (REACH / ACROSS).round() as usize;
                for k in 1..=steps {
                    let d = k as f64 * ACROSS;
                    if let Some(r) = room_at(&h.level, Vec2::new(p.x + n.x * d, p.y + n.y * d)) {
                        push_once(&mut link.rooms_above, &r.uid);
                        break;
                    }
                }
            }
            // A stair of the storey under it arriving through it.
            link.stair_well = stairs.iter().any(|(_, l, pts)| {
                *l == low && pts.len() >= 2 && {
                    let (a, b) = (pts[pts.len() - 2], pts[pts.len() - 1]);
                    let len = kentos_num::hypot(b.x - a.x, b.y - a.y).max(1e-9);
                    let before = Vec2::new(
                        b.x - (b.x - a.x) / len * BEFORE_HEAD,
                        b.y - (b.y - a.y) / len * BEFORE_HEAD,
                    );
                    inside_area(&h.area, b) || inside_area(&h.area, before)
                }
            });
            out.push(link);
        }
        // The stacks: openings over one another on consecutive storeys.
        let n = out.len();
        let mut root: Vec<usize> = (0..n).collect();
        fn find(root: &mut [usize], mut i: usize) -> usize {
            while root[i] != i {
                root[i] = root[root[i]];
                i = root[i];
            }
            i
        }
        for i in 0..n {
            for j in 0..n {
                if i == j || out[j].below.is_none() || out[j].below != out[i].above {
                    continue;
                }
                if inside_area(&holes[j].area, holes[i].middle)
                    || inside_area(&holes[i].area, holes[j].middle)
                {
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
                    .min_by(|&a, &b| {
                        index(&out[a].above)
                            .cmp(&index(&out[b].above))
                            .then_with(|| out[a].uid.cmp(&out[b].uid))
                    })
                    .map_or_else(|| out[i].uid.clone(), |j| out[j].uid.clone())
            })
            .collect();
        for (l, s) in out.iter_mut().zip(lowest) {
            l.stack = s;
        }
        // The stairs.
        for (uid, level, pts) in &stairs {
            let (Some(first), Some(last)) = (pts.first().copied(), pts.last().copied()) else {
                continue;
            };
            let up = above(level);
            let along = |from: Vec2, to: Vec2, lvl: &Option<String>| -> Option<String> {
                let len = kentos_num::hypot(to.x - from.x, to.y - from.y).max(1e-9);
                let (ux, uy) = ((to.x - from.x) / len, (to.y - from.y) / len);
                let steps = (REACH / 0.3).floor() as usize;
                (0..=steps).find_map(|k| {
                    let d = k as f64 * 0.3;
                    room_at(lvl, Vec2::new(to.x + ux * d, to.y + uy * d)).map(|r| r.uid.clone())
                })
            };
            let second = pts.get(1).copied().unwrap_or(last);
            let before_last = pts
                .get(pts.len().saturating_sub(2))
                .copied()
                .unwrap_or(first);
            let mut link = VerticalLink {
                uid: uid.clone(),
                kind: "stair",
                below: level.clone(),
                above: up.clone(),
                stack: uid.clone(),
                ..VerticalLink::default()
            };
            if let Some(r) = along(second, first, level) {
                link.rooms_below.push(r);
            }
            if up.is_some()
                && let Some(r) = along(before_last, last, &up)
            {
                link.rooms_above.push(r);
            }
            out.push(link);
        }
        out.sort_by(|a, b| {
            index(&a.below)
                .cmp(&index(&b.below))
                .then_with(|| a.uid.cmp(&b.uid))
        });
        out
    }
}
