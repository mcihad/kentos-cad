//! The unknowns of a constrained drawing (docs/adr/0334 §4): an object's own numbers, laid out one after another, and
//! the parts a constraint names built from them. A point object is its points' east and north; a line its two ends; a
//! circle its centre and radius; an arc its centre, radius and two angles; a path every vertex of every ring of every
//! part (a bulged edge keeps its bulge, so it is not a segment). Other kinds take no constraint.

use crate::entity::Shape;
use crate::geom::bulge::is_arc_bulge;
use crate::vec2::Vec2;

use super::dual::{D, P};
use super::{Element, PointAt};

/// What an unknown measures: it decides its weight in the least-change step and its local origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VarKind {
    East,
    North,
    Length,
    /// An arc's angle; weighted by its radius so a turn is measured along the arc.
    Angle,
}

/// Where an object's unknowns start, and its rings (part, ring) with where each starts and how many vertices.
#[derive(Clone, Debug, Default)]
pub(super) struct Slot {
    pub start: usize,
    pub rings: Vec<RingAt>,
}

#[derive(Clone, Debug)]
pub(super) struct RingAt {
    pub part: u32,
    pub ring: u32,
    pub start: usize,
    pub count: usize,
    pub closed: bool,
    pub bulges: Vec<f64>,
}

/// The layout of every object's unknowns.
pub(super) struct Model {
    pub x: Vec<f64>,
    pub kinds: Vec<VarKind>,
    /// The object each unknown belongs to.
    pub owner: Vec<usize>,
    /// Per object: its slot, none when its kind takes no constraint.
    pub slots: Vec<Option<Slot>>,
}

/// A ring of a path: its part, its ring, its vertices, its bulges and whether it closes.
type RingOf = (u32, u32, Vec<Vec2>, Vec<f64>, bool);

fn rings_of(shape: &Shape) -> Option<Vec<RingOf>> {
    let ring = |pts: &[Vec2], bulges: &Option<Vec<f64>>, closed: bool| {
        (
            pts.to_vec(),
            bulges.clone().unwrap_or_else(|| vec![0.0; pts.len()]),
            closed,
        )
    };
    let mut out = Vec::new();
    let mut add = |part: u32,
                   pts: &[Vec2],
                   bulges: &Option<Vec<f64>>,
                   holes: Option<&Vec<crate::geom::arrangement::Ring>>,
                   closed: bool| {
        let (p, b, c) = ring(pts, bulges, closed);
        out.push((part, 0, p, b, c));
        for (h, hole) in holes.into_iter().flatten().enumerate() {
            let (p, b, _) = ring(&hole.pts, &hole.bulges, true);
            out.push((part, h as u32 + 1, p, b, true));
        }
    };
    match shape {
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            add(0, pts, bulges, holes.as_ref(), true);
            for (k, p) in parts.iter().flatten().enumerate() {
                add(k as u32 + 1, &p.pts, &p.bulges, p.holes.as_ref(), true);
            }
        }
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
            closed,
            ..
        } => {
            add(0, pts, bulges, holes.as_ref(), *closed);
            for (k, p) in parts.iter().flatten().enumerate() {
                add(k as u32 + 1, &p.pts, &p.bulges, p.holes.as_ref(), false);
            }
        }
        _ => return None,
    }
    Some(out)
}

impl Model {
    /// The unknowns of the objects `used` names (others keep no slot).
    pub fn new(shapes: &[Shape], used: &[bool]) -> Model {
        let mut m = Model {
            x: Vec::new(),
            kinds: Vec::new(),
            owner: Vec::new(),
            slots: vec![None; shapes.len()],
        };
        for (i, shape) in shapes.iter().enumerate() {
            if !used[i] {
                continue;
            }
            let start = m.x.len();
            let push = |m: &mut Model, v: f64, k: VarKind| {
                m.x.push(v);
                m.kinds.push(k);
                m.owner.push(i);
            };
            let mut slot = Slot {
                start,
                rings: Vec::new(),
            };
            match shape {
                Shape::Point { p, parts, .. } => {
                    push(&mut m, p.x, VarKind::East);
                    push(&mut m, p.y, VarKind::North);
                    for q in parts.iter().flatten() {
                        push(&mut m, q.p.x, VarKind::East);
                        push(&mut m, q.p.y, VarKind::North);
                    }
                }
                Shape::Line { a, b } => {
                    for v in [a, b] {
                        push(&mut m, v.x, VarKind::East);
                        push(&mut m, v.y, VarKind::North);
                    }
                }
                Shape::Circle { c, r } => {
                    push(&mut m, c.x, VarKind::East);
                    push(&mut m, c.y, VarKind::North);
                    push(&mut m, *r, VarKind::Length);
                }
                Shape::Arc { c, r, a0, a1 } => {
                    push(&mut m, c.x, VarKind::East);
                    push(&mut m, c.y, VarKind::North);
                    push(&mut m, *r, VarKind::Length);
                    push(&mut m, *a0, VarKind::Angle);
                    push(&mut m, *a1, VarKind::Angle);
                }
                other => {
                    let Some(rings) = rings_of(other) else {
                        continue;
                    };
                    for (part, ring, pts, bulges, closed) in rings {
                        let at = m.x.len();
                        for v in &pts {
                            push(&mut m, v.x, VarKind::East);
                            push(&mut m, v.y, VarKind::North);
                        }
                        slot.rings.push(RingAt {
                            part,
                            ring,
                            start: at,
                            count: pts.len(),
                            closed,
                            bulges,
                        });
                    }
                }
            }
            m.slots[i] = Some(slot);
        }
        m
    }

    /// The unknowns an element is built from, in order; none when the object has no such part.
    pub fn element_vars(
        &self,
        shapes: &[Shape],
        object: usize,
        element: &Element,
    ) -> Option<Vec<usize>> {
        let slot = self.slots.get(object)?.as_ref()?;
        let s = slot.start;
        let shape = &shapes[object];
        match (element, shape) {
            (Element::Point(at), _) => self.point_vars(shape, slot, *at),
            (
                Element::Segment {
                    part: 0,
                    ring: 0,
                    index: 0,
                },
                Shape::Line { .. },
            ) => Some(vec![s, s + 1, s + 2, s + 3]),
            (
                Element::Segment { part, ring, index },
                Shape::Polygon { .. } | Shape::Polyline { .. },
            ) => {
                let r = slot
                    .rings
                    .iter()
                    .find(|r| r.part == *part && r.ring == *ring)?;
                let i = *index as usize;
                let j = if i + 1 < r.count {
                    i + 1
                } else if r.closed && i + 1 == r.count {
                    0
                } else {
                    return None;
                };
                if i >= r.count || is_arc_bulge(r.bulges.get(i).copied().unwrap_or(0.0)) {
                    return None;
                }
                Some(vec![
                    r.start + 2 * i,
                    r.start + 2 * i + 1,
                    r.start + 2 * j,
                    r.start + 2 * j + 1,
                ])
            }
            (Element::Circle, Shape::Circle { .. }) => Some(vec![s, s + 1, s + 2]),
            (Element::Circle, Shape::Arc { .. }) => Some(vec![s, s + 1, s + 2]),
            _ => None,
        }
    }

    fn point_vars(&self, shape: &Shape, slot: &Slot, at: PointAt) -> Option<Vec<usize>> {
        let s = slot.start;
        match (at, shape) {
            (PointAt::Node(k), Shape::Point { parts, .. }) => {
                let count = 1 + parts.as_ref().map_or(0, Vec::len);
                ((k as usize) < count).then(|| vec![s + 2 * k as usize, s + 2 * k as usize + 1])
            }
            (PointAt::Start, Shape::Line { .. }) => Some(vec![s, s + 1]),
            (PointAt::End, Shape::Line { .. }) => Some(vec![s + 2, s + 3]),
            (PointAt::Center, Shape::Circle { .. } | Shape::Arc { .. }) => Some(vec![s, s + 1]),
            // An arc's ends: from its centre, radius and angle.
            (PointAt::Start, Shape::Arc { .. }) => Some(vec![s, s + 1, s + 2, s + 3]),
            (PointAt::End, Shape::Arc { .. }) => Some(vec![s, s + 1, s + 2, s + 4]),
            (
                PointAt::Vertex { part, ring, index },
                Shape::Polygon { .. } | Shape::Polyline { .. },
            ) => {
                let r = slot
                    .rings
                    .iter()
                    .find(|r| r.part == part && r.ring == ring)?;
                let i = index as usize;
                (i < r.count).then(|| vec![r.start + 2 * i, r.start + 2 * i + 1])
            }
            // A polyline's ends are its first and last vertex of its own path.
            (PointAt::Start, Shape::Polyline { .. }) => {
                let r = slot.rings.first()?;
                Some(vec![r.start, r.start + 1])
            }
            (PointAt::End, Shape::Polyline { .. }) => {
                let r = slot.rings.first()?;
                Some(vec![
                    r.start + 2 * (r.count - 1),
                    r.start + 2 * (r.count - 1) + 1,
                ])
            }
            _ => None,
        }
    }
}

/// An element's value as duals, read from the unknowns `vars` names through `val` (the solver gives each its slot).
pub(super) enum Built {
    Point(P),
    Segment(P, P),
    Circle(P, D),
}

/// Builds an element from its unknowns' duals, in [`Model::element_vars`]' order.
pub(super) fn build(shape: &Shape, element: &Element, v: &[D]) -> Built {
    match element {
        Element::Point(at) => match (at, shape) {
            (PointAt::Start, Shape::Arc { .. }) | (PointAt::End, Shape::Arc { .. }) => {
                let (c, r, a) = (P { x: v[0], y: v[1] }, v[2], v[3]);
                Built::Point(P {
                    x: c.x + r * a.cos(),
                    y: c.y + r * a.sin(),
                })
            }
            _ => Built::Point(P { x: v[0], y: v[1] }),
        },
        Element::Segment { .. } => Built::Segment(P { x: v[0], y: v[1] }, P { x: v[2], y: v[3] }),
        Element::Circle => Built::Circle(P { x: v[0], y: v[1] }, v[2]),
    }
}

/// Writes the unknowns back into their shape.
pub(super) fn write_back(shape: &mut Shape, slot: &Slot, x: &[f64]) {
    let s = slot.start;
    match shape {
        Shape::Point { p, parts, .. } => {
            *p = Vec2::new(x[s], x[s + 1]);
            for (k, q) in parts.iter_mut().flatten().enumerate() {
                q.p = Vec2::new(x[s + 2 + 2 * k], x[s + 3 + 2 * k]);
            }
        }
        Shape::Line { a, b } => {
            *a = Vec2::new(x[s], x[s + 1]);
            *b = Vec2::new(x[s + 2], x[s + 3]);
        }
        Shape::Circle { c, r } => {
            *c = Vec2::new(x[s], x[s + 1]);
            *r = x[s + 2];
        }
        Shape::Arc { c, r, a0, a1 } => {
            *c = Vec2::new(x[s], x[s + 1]);
            *r = x[s + 2];
            *a0 = x[s + 3];
            *a1 = x[s + 4];
        }
        Shape::Polygon {
            pts, holes, parts, ..
        }
        | Shape::Polyline {
            pts, holes, parts, ..
        } => {
            for at in &slot.rings {
                let ring: Option<&mut Vec<Vec2>> = match (at.part, at.ring) {
                    (0, 0) => Some(pts),
                    (0, h) => holes
                        .as_mut()
                        .and_then(|hs| hs.get_mut(h as usize - 1))
                        .map(|r| &mut r.pts),
                    (k, 0) => parts
                        .as_mut()
                        .and_then(|ps| ps.get_mut(k as usize - 1))
                        .map(|p| &mut p.pts),
                    (k, h) => parts
                        .as_mut()
                        .and_then(|ps| ps.get_mut(k as usize - 1))
                        .and_then(|p| p.holes.as_mut())
                        .and_then(|hs| hs.get_mut(h as usize - 1))
                        .map(|r| &mut r.pts),
                };
                for (i, v) in ring.into_iter().flatten().enumerate() {
                    *v = Vec2::new(x[at.start + 2 * i], x[at.start + 2 * i + 1]);
                }
            }
        }
        _ => {}
    }
}
