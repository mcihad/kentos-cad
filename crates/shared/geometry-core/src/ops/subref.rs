//! Places on an object that an annotation measures, and how they follow the
//! object's edits (docs/adr/0249, 0263; plan/cad.md `CAD-DATA-13`, `14`, `DEC-03`).
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
//!
//! A place along an edge or a curve (docs/adr/0263 §1) keeps its fraction:
//! on a path it follows its edge as an edge's middle does. A range is two such
//! places; a place inside a block is a place on the piece a block path names
//! in an insert, followed with the definitions of each moment.

use crate::block::Blocks;
use crate::entity::{Entity, Part, Shape};
use crate::geom::arc::{ArcGeom, arc_end, arc_start, norm_angle, point_on_circle, sweep};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::{bulge_arc, bulge_at, segment_mid};
use crate::geom::ellipse::{EllipseGeom, ellipse_length, ellipse_point, param_of_point};
use crate::jsmath::{TAU, atan2, js_hypot, js_max, js_min};
use crate::store::snap::SnapKind;
use crate::vec2::Vec2;
use crate::geom::intersect::closest_on_edge;
use crate::ops::edges::entity_edges;

/// A place on an object.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// A point along a line (all zero), a path's edge from vertex `index`, an arc, a circle or an ellipse, at the
    /// fraction `t` (0 to 1) of its chord or sweep (docs/adr/0263 §1).
    Along {
        part: usize,
        ring: usize,
        index: usize,
        t: f64,
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
    /// The piece a block path names is no longer in the insert (docs/adr/0263 §3).
    PieceGone,
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
            Reason::PieceGone => "piece_gone",
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
        Shape::Wall { .. } => 7,
        _ => 6,
    }
}

/// A wall's line `part` (0 its left face, the last its right face, the components' boundaries between; docs/adr/0340
/// §5): a wall's places are its lines' vertices and edges' middles, ring 0, at its path's vertices.
fn wall_line(shape: &Shape, part: usize) -> Option<(crate::geom::wall::Line, bool)> {
    let Shape::Wall { pts, bulges, wall } = shape else {
        return None;
    };
    let lines = crate::geom::wall::lines(pts, bulges.as_deref(), wall).ok()?;
    lines.into_iter().nth(part).map(|l| (l, wall.closed))
}

/// Where `place` is on `e`; none when `e` has no such place.
pub fn point(e: &Entity, place: Place) -> Option<Vec2> {
    match (&e.shape, place) {
        (Shape::Point { p, .. }, Place::Node { part: 0 }) => Some(*p),
        // A text's, an insert's and a table's insertion point (docs/adr/0272 §1).
        (
            Shape::Text { p, .. } | Shape::Insert { p, .. } | Shape::Table { p, .. },
            Place::Node { part: 0 },
        ) => Some(*p),
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
        (Shape::Wall { .. }, Place::Vertex { part, ring: 0, index }) => {
            wall_line(&e.shape, part)?.0.pts.get(index).copied()
        }
        (Shape::Wall { .. }, Place::Mid { part, ring: 0, index }) => {
            let (l, closed) = wall_line(&e.shape, part)?;
            let n = l.pts.len();
            if index >= n || (!closed && index + 1 >= n) {
                return None;
            }
            Some(segment_mid(l.pts[index], l.pts[(index + 1) % n], l.bulges[index]))
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
        (
            shape,
            Place::Along {
                part,
                ring,
                index,
                t,
            },
        ) => along_point(shape, part, ring, index, t),
        _ => None,
    }
}

/// Whether `place` addresses a part and ring with the same number of vertices (or points) in
/// both shapes: the structure the place depends on did not change.
fn same_structure(before: &Shape, after: &Shape, place: Place) -> bool {
    // Only a path's rings and a multi-point's points have a structure; a line's middle has none.
    let path = matches!(before, Shape::Polyline { .. } | Shape::Polygon { .. });
    // A wall's: as many path vertices, closed alike, as many lines (docs/adr/0340 §5).
    if let (Shape::Wall { pts: p0, wall: w0, .. }, Shape::Wall { pts: p1, wall: w1, .. }) = (before, after) {
        return p0.len() == p1.len()
            && w0.closed == w1.closed
            && crate::geom::wall::boundaries(w0).len() == crate::geom::wall::boundaries(w1).len();
    }
    match place {
        Place::Vertex { part, ring, .. }
        | Place::Mid { part, ring, .. }
        | Place::Along { part, ring, .. }
            if path =>
        {
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
        Shape::Wall { pts, bulges, wall } => {
            for (part, l) in crate::geom::wall::lines(pts, bulges.as_deref(), wall)
                .unwrap_or_default()
                .into_iter()
                .enumerate()
            {
                for (index, p) in l.pts.into_iter().enumerate() {
                    out.push((Place::Vertex { part, ring: 0, index }, p));
                }
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
        Place::Mid { part, ring, index }
        | Place::Along {
            part, ring, index, ..
        } => {
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
            let to = match place {
                Place::Along { t, .. } => Place::Along {
                    part: pa,
                    ring: ra,
                    index: ia,
                    t,
                },
                _ => Place::Mid {
                    part: pa,
                    ring: ra,
                    index: ia,
                },
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
/// first (in the object's order) whose point is `at`, within a nanometre. Yakın, Dik, Teğet and Çeyrek stand on an
/// edge or a curve: the first one `at` is on, as an `along` place (docs/adr/0263 §1). None for another snap
/// (Kesişim is on two objects …) or when no place is there.
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
        // Ekleme noktası: the object's own point (docs/adr/0272 §1).
        SnapKind::Insertion => candidates.push(Place::Node { part: 0 }),
        SnapKind::Nearest | SnapKind::Perpendicular | SnapKind::Tangent | SnapKind::Quadrant => {
            return along_at(e, at);
        }
        _ => return None,
    }
    const NEAR: f64 = 1e-9;
    candidates.into_iter().find(|&p| {
        point(e, p).is_some_and(|q| (q.x - at.x).abs() <= NEAR && (q.y - at.y).abs() <= NEAR)
    })
}

// ---- places along an edge or a curve (docs/adr/0263 §1) ----------------------------------------------------------

/// The circle of a bulged edge: centre, radius, start angle, signed sweep; none when straight.
fn edge_arc(a: Vec2, b: Vec2, bulge: f64) -> Option<(Vec2, f64, f64, f64)> {
    bulge_arc(a, b, bulge).map(|g| (g.c, g.r, g.a0, g.sweep))
}

fn lerp(a: Vec2, b: Vec2, t: f64) -> Vec2 {
    Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// A path's edge from vertex `index`: its ends and bulge; none when the ring has no such edge.
fn edge_of(shape: &Shape, part: usize, ring: usize, index: usize) -> Option<(Vec2, Vec2, f64)> {
    let r = ring_of(shape, part, ring)?;
    let n = r.pts.len();
    if index >= n || (!r.closed && index + 1 >= n) {
        return None;
    }
    Some((
        r.pts[index],
        r.pts[(index + 1) % n],
        bulge_at(r.bulges, index),
    ))
}

/// Where `along { part, ring, index, t }` is on `shape`.
fn along_point(shape: &Shape, part: usize, ring: usize, index: usize, t: f64) -> Option<Vec2> {
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    let whole = part == 0 && ring == 0 && index == 0;
    match shape {
        Shape::Line { a, b } => whole.then(|| lerp(*a, *b, t)),
        Shape::Arc { c, r, a0, a1 } => {
            whole.then(|| point_on_circle(*c, *r, a0 + t * sweep(*a0, *a1)))
        }
        Shape::Circle { c, r } => whole.then(|| point_on_circle(*c, *r, t * TAU)),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => whole.then(|| {
            let g = EllipseGeom {
                c: *c,
                major: *major,
                ratio: *ratio,
                t0: *t0,
                t1: *t1,
            };
            ellipse_point(&g, t0 + t * sweep(*t0, *t1))
        }),
        _ => {
            let (a, b, k) = edge_of(shape, part, ring, index)?;
            Some(match edge_arc(a, b, k) {
                Some((c, r, a0, sw)) => point_on_circle(c, r, a0 + t * sw),
                None => lerp(a, b, t),
            })
        }
    }
}

/// How near a snap's point must be to an edge to stand on it: a nanometre, or 1e-14 of its coordinates when larger.
fn near_enough(at: Vec2) -> f64 {
    js_max(1e-9, 1e-14 * js_max(at.x.abs(), at.y.abs()))
}

fn clamp01(t: f64) -> f64 {
    js_min(1.0, js_max(0.0, t))
}

/// The fraction of a straight edge at which `at` lies on it; none off it.
fn t_on_segment(a: Vec2, b: Vec2, at: Vec2) -> Option<f64> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let ll = dx * dx + dy * dy;
    if ll == 0.0 {
        return None;
    }
    let t = clamp01(((at.x - a.x) * dx + (at.y - a.y) * dy) / ll);
    let q = lerp(a, b, t);
    (js_hypot(q.x - at.x, q.y - at.y) <= near_enough(at)).then_some(t)
}

/// The fraction of a signed sweep from `a0` at which `at` lies on the circle's arc; none off it.
fn t_on_arc(c: Vec2, r: f64, a0: f64, sw: f64, at: Vec2) -> Option<f64> {
    if (js_hypot(at.x - c.x, at.y - c.y) - r).abs() > near_enough(at) {
        return None;
    }
    let th = atan2(at.y - c.y, at.x - c.x);
    let d = if sw > 0.0 {
        norm_angle(th - a0)
    } else {
        norm_angle(a0 - th)
    };
    let span = sw.abs();
    if d > span + 1e-9 {
        return (d >= TAU - 1e-9).then_some(0.0);
    }
    Some(clamp01(d / span))
}

/// The first edge or curve of `e` (in its order) that `at` lies on, as an `along` place.
fn along_at(e: &Entity, at: Vec2) -> Option<Place> {
    let whole = |t: f64| Place::Along {
        part: 0,
        ring: 0,
        index: 0,
        t,
    };
    match &e.shape {
        Shape::Line { a, b } => t_on_segment(*a, *b, at).map(whole),
        Shape::Arc { c, r, a0, a1 } => t_on_arc(*c, *r, *a0, sweep(*a0, *a1), at).map(whole),
        Shape::Circle { c, r } => t_on_arc(*c, *r, 0.0, TAU, at).map(|t| whole(t % 1.0)),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = EllipseGeom {
                c: *c,
                major: *major,
                ratio: *ratio,
                t0: *t0,
                t1: *t1,
            };
            let p = param_of_point(&g, at);
            let q = ellipse_point(&g, p);
            if js_hypot(q.x - at.x, q.y - at.y) > near_enough(at) {
                return None;
            }
            let sw = sweep(*t0, *t1);
            let d = norm_angle(p - t0);
            if d > sw + 1e-9 {
                return (d >= TAU - 1e-9).then(|| whole(0.0));
            }
            Some(whole(clamp01(d / sw)))
        }
        _ => vertex_places(&e.shape).into_iter().find_map(|(p, _)| {
            let Place::Vertex { part, ring, index } = p else {
                return None;
            };
            let (a, b, k) = edge_of(&e.shape, part, ring, index)?;
            let t = match edge_arc(a, b, k) {
                Some((c, r, a0, sw)) => t_on_arc(c, r, a0, sw, at),
                None => t_on_segment(a, b, at),
            }?;
            Some(Place::Along {
                part,
                ring,
                index,
                t,
            })
        }),
    }
}

// ---- the edges and curves a dimension measures (docs/adr/0320 §1) ----------------------------------------------

/// The kind of edge a measure takes: a straight edge (an angle's arms), an arc with ends (an arc length, an arc's
/// angle), or anything round (a radius: a circle, an arc or a path's arc segment).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Straight,
    Arc,
    Round,
}

/// The fraction along a straight edge of the point nearest `at`, within 0 and 1.
fn t_nearest_on_segment(a: Vec2, b: Vec2, at: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let ll = dx * dx + dy * dy;
    if ll == 0.0 {
        return 0.0;
    }
    clamp01(((at.x - a.x) * dx + (at.y - a.y) * dy) / ll)
}

/// The fraction of a signed sweep from `a0` of the point of the arc nearest `at`: its angle's, or the nearer end's
/// when it lies off the arc.
fn t_nearest_on_arc(c: Vec2, a0: f64, sw: f64, at: Vec2) -> f64 {
    let th = atan2(at.y - c.y, at.x - c.x);
    let d = if sw > 0.0 {
        norm_angle(th - a0)
    } else {
        norm_angle(a0 - th)
    };
    let span = sw.abs();
    if span >= TAU - 1e-12 {
        return d / TAU;
    }
    if d <= span {
        clamp01(d / span)
    } else if d - span < TAU - d {
        1.0
    } else {
        0.0
    }
}

/// The `along` place on `e`'s edge of `kind` nearest to `near` (the first of equals, in the object's order), at the
/// point of it nearest `near`: the place an edge picked by a click there is bound by (docs/adr/0320 §1). None when
/// `e` has no edge of that kind.
pub fn edge_place_near(e: &Entity, kind: EdgeKind, near: Vec2) -> Option<Place> {
    let whole = |t: f64| Place::Along {
        part: 0,
        ring: 0,
        index: 0,
        t,
    };
    match (&e.shape, kind) {
        (Shape::Line { a, b }, EdgeKind::Straight) => {
            return (a != b).then(|| whole(t_nearest_on_segment(*a, *b, near)));
        }
        (Shape::Arc { c, a0, a1, .. }, EdgeKind::Arc | EdgeKind::Round) => {
            let sw = sweep(*a0, *a1);
            if kind == EdgeKind::Arc && !(sw > 1e-9 && sw < TAU - 1e-9) {
                return None;
            }
            return Some(whole(t_nearest_on_arc(*c, *a0, sw, near)));
        }
        (Shape::Circle { c, .. }, EdgeKind::Round) => {
            return Some(whole(t_nearest_on_arc(*c, 0.0, TAU, near)));
        }
        (Shape::Polyline { .. } | Shape::Polygon { .. }, _) => {}
        _ => return None,
    }
    let mut best: Option<(Place, f64)> = None;
    for (p, _) in vertex_places(&e.shape) {
        let Place::Vertex { part, ring, index } = p else {
            continue;
        };
        let Some((a, b, k)) = edge_of(&e.shape, part, ring, index) else {
            continue;
        };
        let (edge, t) = match (edge_arc(a, b, k), kind) {
            (None, EdgeKind::Straight) if a != b => (
                crate::geom::intersect::Edge::Seg { a, b },
                t_nearest_on_segment(a, b, near),
            ),
            (Some((c, r, a0, sw)), EdgeKind::Arc | EdgeKind::Round) => (
                crate::geom::intersect::Edge::Arc { c, r, a0, sweep: sw },
                t_nearest_on_arc(c, a0, sw, near),
            ),
            _ => continue,
        };
        let d = closest_on_edge(&edge, near).d;
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((
                Place::Along {
                    part,
                    ring,
                    index,
                    t,
                },
                d,
            ));
        }
    }
    best.map(|(p, _)| p)
}

/// The centre of the curve `place` is on: a circle's or an arc's, or a path's arc segment's (an `along` place or an
/// edge's middle); none on a straight edge or another object (docs/adr/0320 §1).
pub fn curve_centre(e: &Entity, place: Place) -> Option<Vec2> {
    match (&e.shape, place) {
        (
            Shape::Circle { c, .. } | Shape::Arc { c, .. },
            Place::Along { .. } | Place::Start | Place::End | Place::Center,
        ) => Some(*c),
        (
            Shape::Polyline { .. } | Shape::Polygon { .. },
            Place::Along {
                part, ring, index, ..
            }
            | Place::Mid { part, ring, index },
        ) => {
            let (a, b, k) = edge_of(&e.shape, part, ring, index)?;
            edge_arc(a, b, k).map(|(c, ..)| c)
        }
        _ => None,
    }
}

/// The ends of the straight edge `place` is on, in the object's direction: a line's, or a path's straight edge's (an
/// `along` place or an edge's middle); none on a curve (docs/adr/0320 §1).
pub fn edge_line(e: &Entity, place: Place) -> Option<(Vec2, Vec2)> {
    match (&e.shape, place) {
        (Shape::Line { a, b }, Place::Along { .. } | Place::Mid { .. } | Place::Start | Place::End) => {
            Some((*a, *b))
        }
        (
            Shape::Polyline { .. } | Shape::Polygon { .. },
            Place::Along {
                part, ring, index, ..
            }
            | Place::Mid { part, ring, index },
        ) => {
            let (a, b, k) = edge_of(&e.shape, part, ring, index)?;
            edge_arc(a, b, k).is_none().then_some((a, b))
        }
        _ => None,
    }
}

// ---- ranges (docs/adr/0263 §2) -----------------------------------------------------------------------------------

/// The length of a piece of an edge, from fraction `t0` to `t1`.
fn edge_piece(a: Vec2, b: Vec2, k: f64, t0: f64, t1: f64) -> f64 {
    match edge_arc(a, b, k) {
        Some((_, r, _, sw)) => r * sw.abs() * (t1 - t0),
        None => js_hypot(b.x - a.x, b.y - a.y) * (t1 - t0),
    }
}

/// The length of an ellipse's curve between parameters `s0` and `s1` (`s0 < s1`).
fn ellipse_piece(c: Vec2, major: Vec2, ratio: f64, s0: f64, s1: f64) -> f64 {
    ellipse_length(&EllipseGeom {
        c,
        major,
        ratio,
        t0: s0,
        t1: s1,
    })
}

/// The length of the range from `from` to `to`, two `along` places of `e` on one part and ring, in the object's
/// direction; around a closed ring, a circle or a whole ellipse when `to` comes first; none when the places are not
/// such or `to` comes first on an open one.
pub fn range_length(e: &Entity, from: Place, to: Place) -> Option<f64> {
    let (
        Place::Along {
            part,
            ring,
            index: i0,
            t: t0,
        },
        Place::Along {
            part: p1,
            ring: r1,
            index: i1,
            t: t1,
        },
    ) = (from, to)
    else {
        return None;
    };
    point(e, from)?;
    point(e, to)?;
    if (part, ring) != (p1, r1) {
        return None;
    }
    let behind = (i1, t1) < (i0, t0);
    match &e.shape {
        Shape::Line { a, b } => (!behind).then(|| js_hypot(b.x - a.x, b.y - a.y) * (t1 - t0)),
        Shape::Arc { r, a0, a1, .. } => (!behind).then(|| r * sweep(*a0, *a1) * (t1 - t0)),
        Shape::Circle { r, .. } => {
            let d = t1 - t0;
            Some(r * TAU * if d >= 0.0 { d } else { d + 1.0 })
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0: e0,
            t1: e1,
        } => {
            let sw = sweep(*e0, *e1);
            if sw >= TAU {
                let d = t1 - t0;
                let s0 = e0 + t0 * TAU;
                Some(ellipse_piece(
                    *c,
                    *major,
                    *ratio,
                    s0,
                    s0 + (if d >= 0.0 { d } else { d + 1.0 }) * TAU,
                ))
            } else {
                (!behind).then(|| ellipse_piece(*c, *major, *ratio, e0 + t0 * sw, e0 + t1 * sw))
            }
        }
        shape => {
            let r = ring_of(shape, part, ring)?;
            let n = r.pts.len();
            if behind && !r.closed {
                return None;
            }
            let edges = if r.closed { n } else { n - 1 };
            let (mut i, mut t, mut wrapped, mut total) = (i0, t0, behind, 0.0);
            loop {
                let last = i == i1 && !wrapped;
                let end = if last { t1 } else { 1.0 };
                total += edge_piece(r.pts[i], r.pts[(i + 1) % n], bulge_at(r.bulges, i), t, end);
                if last {
                    return Some(total);
                }
                i = (i + 1) % edges;
                t = 0.0;
                if i == 0 {
                    wrapped = false;
                }
            }
        }
    }
}

/// Where a range went: both ends followed; one broken breaks it; ends on different rings break it (`VertexGone`).
pub fn follow_range(
    from: Place,
    to: Place,
    before: &Entity,
    after: Option<&Entity>,
) -> Result<(Place, Place), Reason> {
    let a = match follow(from, before, after) {
        Followed::At(p, _) => p,
        Followed::Broken(r) => return Err(r),
    };
    let b = match follow(to, before, after) {
        Followed::At(p, _) => p,
        Followed::Broken(r) => return Err(r),
    };
    let ring = |p: Place| match p {
        Place::Along { part, ring, .. } => Some((part, ring)),
        _ => None,
    };
    if ring(a) != ring(b) {
        return Err(Reason::VertexGone);
    }
    Ok((a, b))
}

// ---- pieces inside blocks (docs/adr/0263 §3) ---------------------------------------------------------------------

/// The piece a block path names in an insert, as an object of its own.
pub fn piece(blocks: &Blocks, insert: &Entity, path: &[u32]) -> Option<Entity> {
    blocks.piece(&insert.shape, path).map(Entity::new)
}

/// Where a place on a piece inside an insert is.
pub fn point_in(blocks: &Blocks, insert: &Entity, path: &[u32], place: Place) -> Option<Vec2> {
    point(&piece(blocks, insert, path)?, place)
}

/// Where a place on a piece inside an insert went: the insert gone, or no longer an insert, breaks it; the piece
/// gone from the new insert (or its new definition) breaks it (`PieceGone`); else the piece's place follows by
/// [`follow`]. `before` and `after` each carry the definitions of their moment.
pub fn follow_in(
    place: Place,
    path: &[u32],
    before: (&Blocks, &Entity),
    after: Option<(&Blocks, &Entity)>,
) -> Followed {
    follow_path(place, path, before, after).0
}

/// [`follow_in`], and the path the piece has after (docs/adr/0315 §1): each step found again by the object's local
/// id, which a definition keeps through its edits (an object added before it, or one removed, moves its place).
pub fn follow_path(
    place: Place,
    path: &[u32],
    before: (&Blocks, &Entity),
    after: Option<(&Blocks, &Entity)>,
) -> (Followed, Vec<u32>) {
    let Some((blocks_after, after)) = after else {
        return (Followed::Broken(Reason::ObjectGone), path.to_vec());
    };
    if !matches!(after.shape, Shape::Insert { .. }) {
        return (Followed::Broken(Reason::KindChanged), path.to_vec());
    }
    let Some(was) = piece(before.0, before.1, path) else {
        return (Followed::Broken(Reason::NoSuchPlace), path.to_vec());
    };
    let moved = before
        .0
        .path_in(&before.1.shape, blocks_after, &after.shape, path)
        .unwrap_or_else(|| path.to_vec());
    let Some(now) = piece(blocks_after, after, &moved) else {
        return (Followed::Broken(Reason::PieceGone), path.to_vec());
    };
    (follow(place, &was, Some(&now)), moved)
}

/// The sub-object a click at `at` falls on within `tol` (docs/adr/0276): its points (an end, a vertex, a point
/// object's or an insertion point; the nearest), and apart from them the nearest point of its edges and curves.
struct Near {
    point: Option<(Place, f64)>,
    edge: Option<(Vec2, f64)>,
}

fn near_on(e: &Entity, at: Vec2, tol: f64) -> Near {
    let mut point_best: Option<(Place, f64)> = None;
    let candidates = [Place::Start, Place::End, Place::Node { part: 0 }]
        .into_iter()
        .filter_map(|p| point(e, p).map(|q| (p, q)))
        .chain(vertex_places(&e.shape));
    for (p, q) in candidates {
        let d = js_hypot(q.x - at.x, q.y - at.y);
        if d <= tol && point_best.is_none_or(|(_, b)| d < b) {
            point_best = Some((p, d));
        }
    }
    let mut edge: Option<(Vec2, f64)> = None;
    for ed in entity_edges(&e.shape) {
        let c = closest_on_edge(&ed, at);
        if c.d <= tol && edge.is_none_or(|(_, b)| c.d < b) {
            edge = Some((c.p, c.d));
        }
    }
    Near {
        point: point_best,
        edge,
    }
}

/// The sub-object a click at `at` within `tol` falls on (docs/adr/0276): a point of the object (an end, a vertex, a
/// point object's or an insertion point), the nearest, before anything else; else the nearest point of an edge or a
/// curve as an `along` place; none when nothing of the object is within `tol` (an area's inside, a text's body) or
/// the curve has no `along` place (a spline). With the distance.
pub fn place_near(e: &Entity, at: Vec2, tol: f64) -> Option<(Place, f64)> {
    let n = near_on(e, at, tol);
    if n.point.is_some() {
        return n.point;
    }
    let (q, d) = n.edge?;
    along_at(e, q).map(|p| (p, d))
}

/// [`place_near`] inside an insert: its own insertion point and its pieces' points first, then the pieces' edges;
/// the piece's block path with it (empty for the insert's own point). Equal distances keep the flattening's order.
pub fn place_near_in(
    blocks: &Blocks,
    insert: &Entity,
    at: Vec2,
    tol: f64,
) -> Option<(Vec<u32>, Place, f64)> {
    let own = near_on(insert, at, tol).point.filter(|(p, _)| *p == Place::Node { part: 0 });
    let mut point_best: Option<(Vec<u32>, Place, f64)> = own.map(|(p, d)| (Vec::new(), p, d));
    let mut edge_best: Option<(Vec<u32>, Entity, Vec2, f64)> = None;
    for path in blocks.piece_paths(&insert.shape) {
        let Some(e) = piece(blocks, insert, &path) else {
            continue;
        };
        let n = near_on(&e, at, tol);
        if let Some((p, d)) = n.point
            && point_best.as_ref().is_none_or(|b| d < b.2)
        {
            point_best = Some((path.clone(), p, d));
        }
        if let Some((q, d)) = n.edge
            && edge_best.as_ref().is_none_or(|b| d < b.3)
        {
            edge_best = Some((path, e, q, d));
        }
    }
    if point_best.is_some() {
        return point_best;
    }
    let (path, e, q, d) = edge_best?;
    along_at(&e, q).map(|p| (path, p, d))
}

/// The piece and place a snap of `kind` stood on at `at` inside an insert: the first piece (in the flattening's
/// order) that has one (`place_at`).
pub fn place_in(
    blocks: &Blocks,
    insert: &Entity,
    kind: SnapKind,
    at: Vec2,
) -> Option<(Vec<u32>, Place)> {
    blocks
        .piece_paths(&insert.shape)
        .into_iter()
        .find_map(|path| {
            let e = piece(blocks, insert, &path)?;
            place_at(&e, kind, at).map(|p| (path, p))
        })
}
