//! Places on an object that an annotation measures, and how they follow the
//! object's edits (docs/adr/0249; plan/cad.md `CAD-DATA-13`, `14`, `DEC-03`).
//!
//! A place is named by the object's persistent id (kept by the caller) and a
//! [`Place`]: a point's node, a line's or arc's start or end, a centre, a
//! path's vertex or the middle of its edge. [`point`] gives where the place
//! is now. [`follow`] says where it went after the object changed:
//!
//! - the object gone: [`Followed::Broken`] with [`Reason::ObjectGone`];
//! - another kind of object (a path stays a path: polyline and polygon are
//!   one family): `KindChanged`;
//! - the same structure (a path's place: its part and ring have as many
//!   vertices and close as before; a node: as many points; any other place,
//!   as a line's middle or an arc's start, has none): the same place,
//!   wherever the object moved it;
//! - another structure (vertices added or removed, parts joined or split): a
//!   vertex is looked for by its exact coordinates in the new geometry (an
//!   untouched vertex keeps its bits); an edge's middle by its two ends. Found
//!   once: the place is rebound there; not found or found more than once: it
//!   breaks (`VertexGone`, `Ambiguous`).
//!
//! Exact comparison keeps the rule deterministic on every platform; an edit
//! that both moves a vertex and changes the structure breaks the place, and
//! the annotation keeps its last coordinates (`CAD-ANNO-02`).

use crate::entity::{Entity, Part, Shape};
use crate::geom::arc::{ArcGeom, arc_end, arc_start};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::{bulge_at, segment_mid};
use crate::geom::ellipse::{EllipseGeom, ellipse_point};
use crate::store::snap::SnapKind;
use crate::vec2::Vec2;

/// A place on an object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A point object's position: part 0 its own, `k` its `k`th extra point (docs/adr/0174).
    Node { part: usize },
    /// A line's, arc's or elliptical arc's start.
    Start,
    /// Its end.
    End,
    /// A circle's, arc's or ellipse's centre.
    Center,
    /// A path's vertex: part 0 the object's own path, `k` its `k`th extra part; ring 0 the
    /// outer ring, `h` its `h`th hole; the vertex's index in the ring.
    Vertex {
        part: usize,
        ring: usize,
        index: usize,
    },
    /// The middle of a line (all zero) or of a path's edge from vertex `index` to the next.
    Mid {
        part: usize,
        ring: usize,
        index: usize,
    },
}

/// Why a place no longer is on its object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The object was deleted (or replaced by new objects, as Patlat does).
    ObjectGone,
    /// The object is now of another kind.
    KindChanged,
    /// The place's part, ring or vertex is not in the object.
    NoSuchPlace,
    /// The structure changed and the vertex (or the edge's ends) is nowhere in it.
    VertexGone,
    /// The structure changed and the vertex is in more than one place.
    Ambiguous,
}

impl Reason {
    /// The stable code.
    pub fn code(self) -> &'static str {
        match self {
            Reason::ObjectGone => "object_gone",
            Reason::KindChanged => "kind_changed",
            Reason::NoSuchPlace => "no_such_place",
            Reason::VertexGone => "vertex_gone",
            Reason::Ambiguous => "ambiguous",
        }
    }
}

/// Where a place went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Followed {
    /// On the object, at this place (the same place, or rebound) and this point.
    At(Place, Vec2),
    /// No longer on it.
    Broken(Reason),
}

/// A path's ring: its points, edge bulges and whether it closes.
struct PathRing<'a> {
    pts: &'a [Vec2],
    bulges: Option<&'a [f64]>,
    closed: bool,
}

/// A path's own points, bulges, holes and parts, and whether it closes.
type PathParts<'a> = (
    &'a [Vec2],
    Option<&'a [f64]>,
    Option<&'a Vec<Ring>>,
    Option<&'a Vec<Part>>,
    bool,
);

fn ring_of<'a>(shape: &'a Shape, part: usize, ring: usize) -> Option<PathRing<'a>> {
    let (pts, bulges, holes, parts, closed): PathParts<'a> = match shape {
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
            closed,
            ..
        } => (
            pts,
            bulges.as_deref(),
            holes.as_ref(),
            parts.as_ref(),
            *closed,
        ),
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => (pts, bulges.as_deref(), holes.as_ref(), parts.as_ref(), true),
        _ => return None,
    };
    let (pts, bulges, holes) = if part == 0 {
        (pts, bulges, holes)
    } else {
        let p = parts?.get(part - 1)?;
        (p.pts.as_slice(), p.bulges.as_deref(), p.holes.as_ref())
    };
    if ring == 0 {
        return Some(PathRing {
            pts,
            bulges,
            closed,
        });
    }
    let h = holes?.get(ring - 1)?;
    Some(PathRing {
        pts: &h.pts,
        bulges: h.bulges.as_deref(),
        closed: true,
    })
}

/// The object's family: a place follows only within one.
fn family(shape: &Shape) -> u8 {
    match shape {
        Shape::Point { .. } => 0,
        Shape::Line { .. } => 1,
        Shape::Polyline { .. } | Shape::Polygon { .. } => 2,
        Shape::Circle { .. } => 3,
        Shape::Arc { .. } => 4,
        Shape::Ellipse { .. } => 5,
        _ => 6,
    }
}

/// Where `place` is on `e`; none when `e` has no such place.
pub fn point(e: &Entity, place: Place) -> Option<Vec2> {
    match (&e.shape, place) {
        (Shape::Point { p, .. }, Place::Node { part: 0 }) => Some(*p),
        (Shape::Point { parts, .. }, Place::Node { part }) => {
            parts.as_ref()?.get(part - 1).map(|q| q.p)
        }
        (Shape::Line { a, .. }, Place::Start) => Some(*a),
        (Shape::Line { b, .. }, Place::End) => Some(*b),
        (
            Shape::Line { a, b },
            Place::Mid {
                part: 0,
                ring: 0,
                index: 0,
            },
        ) => Some(Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)),
        (
            Shape::Circle { c, .. } | Shape::Arc { c, .. } | Shape::Ellipse { c, .. },
            Place::Center,
        ) => Some(*c),
        (Shape::Arc { c, r, a0, a1 }, Place::Start | Place::End) => {
            let g = ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            };
            Some(if place == Place::Start {
                arc_start(&g)
            } else {
                arc_end(&g)
            })
        }
        (
            Shape::Ellipse {
                c,
                major,
                ratio,
                t0,
                t1,
            },
            Place::Start | Place::End,
        ) => {
            let g = EllipseGeom {
                c: *c,
                major: *major,
                ratio: *ratio,
                t0: *t0,
                t1: *t1,
            };
            Some(ellipse_point(
                &g,
                if place == Place::Start { *t0 } else { *t1 },
            ))
        }
        (_, Place::Vertex { part, ring, index }) => {
            ring_of(&e.shape, part, ring)?.pts.get(index).copied()
        }
        (_, Place::Mid { part, ring, index }) => {
            let r = ring_of(&e.shape, part, ring)?;
            let n = r.pts.len();
            if index >= n || (!r.closed && index + 1 >= n) {
                return None;
            }
            let a = r.pts[index];
            let b = r.pts[(index + 1) % n];
            Some(segment_mid(a, b, bulge_at(r.bulges, index)))
        }
        _ => None,
    }
}

/// Whether `place` addresses a part and ring with the same number of vertices (or points) in
/// both shapes: the structure the place depends on did not change.
fn same_structure(before: &Shape, after: &Shape, place: Place) -> bool {
    // Only a path's rings and a multi-point's points have a structure; a line's middle has none.
    let path = matches!(before, Shape::Polyline { .. } | Shape::Polygon { .. });
    match place {
        Place::Vertex { part, ring, .. } | Place::Mid { part, ring, .. } if path => {
            match (ring_of(before, part, ring), ring_of(after, part, ring)) {
                (Some(b), Some(a)) => b.pts.len() == a.pts.len() && b.closed == a.closed,
                _ => false,
            }
        }
        Place::Node { .. } => {
            let count = |s: &Shape| match s {
                Shape::Point { parts, .. } => 1 + parts.as_ref().map_or(0, Vec::len),
                _ => 0,
            };
            count(before) == count(after)
        }
        _ => true,
    }
}

/// Every vertex place of a shape, in order (parts, then rings, then vertices).
fn vertex_places(shape: &Shape) -> Vec<(Place, Vec2)> {
    let mut out = Vec::new();
    let extra = match shape {
        Shape::Polyline { parts, .. } | Shape::Polygon { parts, .. } => {
            parts.as_ref().map_or(0, Vec::len)
        }
        Shape::Point { p, parts, .. } => {
            out.push((Place::Node { part: 0 }, *p));
            for (k, q) in parts.iter().flatten().enumerate() {
                out.push((Place::Node { part: k + 1 }, q.p));
            }
            return out;
        }
        _ => return out,
    };
    for part in 0..=extra {
        let mut ring = 0;
        while let Some(r) = ring_of(shape, part, ring) {
            for (index, p) in r.pts.iter().enumerate() {
                out.push((Place::Vertex { part, ring, index }, *p));
            }
            ring += 1;
        }
    }
    out
}

fn same_bits(a: Vec2, b: Vec2) -> bool {
    a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits()
}

/// Where `place` on `before` went when the object became `after` (none: deleted).
pub fn follow(place: Place, before: &Entity, after: Option<&Entity>) -> Followed {
    let Some(after) = after else {
        return Followed::Broken(Reason::ObjectGone);
    };
    if family(&before.shape) != family(&after.shape) {
        return Followed::Broken(Reason::KindChanged);
    }
    let Some(was) = point(before, place) else {
        return Followed::Broken(Reason::NoSuchPlace);
    };
    if same_structure(&before.shape, &after.shape, place) {
        return match point(after, place) {
            Some(p) => Followed::At(place, p),
            None => Followed::Broken(Reason::NoSuchPlace),
        };
    }
    let found = |want: Vec2| -> Result<Place, Reason> {
        let hits: Vec<Place> = vertex_places(&after.shape)
            .into_iter()
            .filter(|(_, p)| same_bits(*p, want))
            .map(|(pl, _)| pl)
            .collect();
        match hits.as_slice() {
            [one] => Ok(*one),
            [] => Err(Reason::VertexGone),
            _ => Err(Reason::Ambiguous),
        }
    };
    match place {
        Place::Vertex { .. } | Place::Node { .. } => match found(was) {
            Ok(to) => Followed::At(to, was),
            Err(reason) => Followed::Broken(reason),
        },
        Place::Mid { part, ring, index } => {
            // The edge's two ends, found in the new geometry as one edge.
            let Some(r) = ring_of(&before.shape, part, ring) else {
                return Followed::Broken(Reason::NoSuchPlace);
            };
            let n = r.pts.len();
            let (a, b) = (r.pts[index], r.pts[(index + 1) % n]);
            let (
                Ok(Place::Vertex {
                    part: pa,
                    ring: ra,
                    index: ia,
                }),
                Ok(Place::Vertex {
                    part: pb,
                    ring: rb,
                    index: ib,
                }),
            ) = (found(a), found(b))
            else {
                return Followed::Broken(match (found(a), found(b)) {
                    (Err(Reason::Ambiguous), _) | (_, Err(Reason::Ambiguous)) => Reason::Ambiguous,
                    _ => Reason::VertexGone,
                });
            };
            let Some(nr) = ring_of(&after.shape, pa, ra) else {
                return Followed::Broken(Reason::NoSuchPlace);
            };
            let m = nr.pts.len();
            let next = |i: usize| {
                if i + 1 < m {
                    Some(i + 1)
                } else if nr.closed {
                    Some(0)
                } else {
                    None
                }
            };
            if (pa, ra) != (pb, rb) || next(ia) != Some(ib) {
                return Followed::Broken(Reason::VertexGone);
            }
            let to = Place::Mid {
                part: pa,
                ring: ra,
                index: ia,
            };
            match point(after, to) {
                Some(p) => Followed::At(to, p),
                None => Followed::Broken(Reason::NoSuchPlace),
            }
        }
        _ => match point(after, place) {
            Some(p) => Followed::At(place, p),
            None => Followed::Broken(Reason::NoSuchPlace),
        },
    }
}

/// The place on `e` a snap of `kind` stood on at `at` (an associative dimension's point, docs/adr/0249 §3): an
/// end or a vertex for Uç nokta, an edge's middle for Orta nokta, the centre for Merkez, a point for Nokta; the
/// first (in the object's order) whose point is `at`, within a nanometre. None for another snap (Yakın, Kesişim …)
/// or when no place is there.
pub fn place_at(e: &Entity, kind: SnapKind, at: Vec2) -> Option<Place> {
    let mut candidates: Vec<Place> = Vec::new();
    match kind {
        SnapKind::Endpoint => {
            candidates.extend([Place::Start, Place::End]);
            candidates.extend(vertex_places(&e.shape).into_iter().map(|(p, _)| p));
        }
        SnapKind::Midpoint => {
            candidates.push(Place::Mid {
                part: 0,
                ring: 0,
                index: 0,
            });
            for (p, _) in vertex_places(&e.shape) {
                if let Place::Vertex { part, ring, index } = p {
                    candidates.push(Place::Mid { part, ring, index });
                }
            }
        }
        SnapKind::Center => candidates.push(Place::Center),
        SnapKind::Node => {
            candidates.extend(vertex_places(&e.shape).into_iter().map(|(p, _)| p));
        }
        _ => return None,
    }
    const NEAR: f64 = 1e-9;
    candidates.into_iter().find(|&p| {
        point(e, p).is_some_and(|q| (q.x - at.x).abs() <= NEAR && (q.y - at.y).abs() <= NEAR)
    })
}
