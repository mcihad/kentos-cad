//! Geçerliliği denetle and Onar (docs/adr/0201 §6): what is wrong with an
//! area's rings or a path as written, each problem once per ring (or path)
//! at its first place; and the area rebuilt from its own rings by the
//! core's overlay (a ring that crosses itself falls into its parts, a hole
//! out of its ring is cut back, overlapping holes are one), repeated
//! vertices dropped.

use super::{Class, path_shape, shape_of, union_all};
use crate::entity::{Shape, area_parts};
use crate::geom::arrangement::{Area, Ring, Rule, Source, TOL, WindingIndex, edge_box};
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, param_on, point_at};
use crate::geom::overlay::overlay;
use crate::geom::region::{orient_ring, ring_area, ring_edges};
use crate::geometry::Bounds;
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::ops::areas::areas_of_entity;
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

/// A problem's kind, in the order a check finds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Two vertices one after another at the same place (a zero-length edge).
    Repeated,
    /// A ring whose area is nothing (its vertices on one line).
    ZeroArea,
    /// A ring that crosses, touches or runs back over itself.
    RingCrossing,
    /// A path that crosses, touches or runs back over itself.
    PathCrossing,
    /// A hole reaching out of its outer ring.
    HoleOutside,
    /// Two holes over each other.
    HolesOverlap,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Repeated => "repeated",
            Kind::ZeroArea => "zeroArea",
            Kind::RingCrossing => "ringCrossing",
            Kind::PathCrossing => "pathCrossing",
            Kind::HoleOutside => "holeOutside",
            Kind::HolesOverlap => "holesOverlap",
        }
    }

    /// The kind a key names.
    pub fn of_key(key: &str) -> Option<Kind> {
        [
            Kind::Repeated,
            Kind::ZeroArea,
            Kind::RingCrossing,
            Kind::PathCrossing,
            Kind::HoleOutside,
            Kind::HolesOverlap,
        ]
        .into_iter()
        .find(|k| k.key() == key)
    }

    /// As the report says it.
    pub fn text(self) -> &'static str {
        match self {
            Kind::Repeated => "Yinelenen köşe",
            Kind::ZeroArea => "Alanı sıfır olan halka",
            Kind::RingCrossing => "Kendini kesen halka",
            Kind::PathCrossing => "Kendini kesen yol",
            Kind::HoleOutside => "Delik dış halkanın dışına taşıyor",
            Kind::HolesOverlap => "Delikler örtüşüyor",
        }
    }
}

/// A problem and the first place it shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Problem {
    pub kind: Kind,
    pub at: Vec2,
}

fn far(a: Vec2, b: Vec2) -> bool {
    js_hypot(a.x - b.x, a.y - b.y) > TOL
}

/// The first of two vertices one after another at one place (a closed ring's last and first too).
fn repeated(pts: &[Vec2], closed: bool) -> Option<Vec2> {
    let n = pts.len();
    let pairs = if closed && n > 1 {
        n
    } else {
        n.saturating_sub(1)
    };
    (0..pairs)
        .find(|&i| !far(pts[i], pts[(i + 1) % n]))
        .map(|i| pts[i])
}

/// Whether a straight-edged ring's vertices all stand on one line (within
/// the tolerance of the line through its first vertex and the one farthest
/// from it): its area is nothing.
fn on_one_line(r: &Ring) -> bool {
    if r.bulges
        .as_ref()
        .is_some_and(|b| b.iter().any(|x| *x != 0.0))
    {
        return false;
    }
    let Some(&p0) = r.pts.first() else {
        return true;
    };
    let mut pf = p0;
    let mut most = 0.0;
    for &p in &r.pts {
        let d = js_hypot(p.x - p0.x, p.y - p0.y);
        if d > most {
            most = d;
            pf = p;
        }
    }
    if most <= TOL {
        return true;
    }
    let (ux, uy) = ((pf.x - p0.x) / most, (pf.y - p0.y) / most);
    r.pts
        .iter()
        .all(|p| ((p.x - p0.x) * uy - (p.y - p0.y) * ux).abs() <= TOL)
}

fn boxes_meet(a: &Bounds, b: &Bounds) -> bool {
    a.min_x <= b.max_x + TOL
        && b.min_x <= a.max_x + TOL
        && a.min_y <= b.max_y + TOL
        && b.min_y <= a.max_y + TOL
}

/// Where two straight edges on one line run over each other for more than
/// the tolerance: the overlap's start along `a`, as its parameter and point.
fn overlap(a: &Edge, b: &Edge) -> Option<(f64, Vec2)> {
    let (Edge::Seg { a: p, b: q }, Edge::Seg { a: r, b: s }) = (*a, *b) else {
        return None;
    };
    let len = js_hypot(q.x - p.x, q.y - p.y);
    if len <= TOL {
        return None;
    }
    let (ux, uy) = ((q.x - p.x) / len, (q.y - p.y) / len);
    let off = |v: Vec2| ((v.x - p.x) * uy - (v.y - p.y) * ux).abs();
    if off(r) > TOL || off(s) > TOL {
        return None;
    }
    let (tr, ts) = (param_on(a, r), param_on(a, s));
    let lo = js_max(0.0, js_min(tr, ts));
    let hi = js_min(1.0, js_max(tr, ts));
    ((hi - lo) * len > TOL).then(|| (lo, point_at(a, lo)))
}

/// Where the edges of one ring or path first cross, touch or run over each
/// other, but for neighbours at the vertex they share (a closed ring's last
/// and first edges are neighbours, and a two-edge ring's at both its
/// vertices): the first pair in the ring's order, the place first along the
/// earlier edge.
fn crossing(edges: &[Edge], closed: bool) -> Option<Vec2> {
    let n = edges.len();
    let boxes: Vec<Bounds> = edges.iter().map(edge_box).collect();
    // Edges by their left end: a pair is tried only while their boxes can meet.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| boxes[a].min_x.total_cmp(&boxes[b].min_x));
    let mut best: Option<((usize, usize), Vec2)> = None;
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if boxes[j].min_x > boxes[i].max_x + TOL {
                break;
            }
            if !boxes_meet(&boxes[i], &boxes[j]) {
                continue;
            }
            let (lo, hi) = (i.min(j), i.max(j));
            if best.is_some_and(|(key, _)| key < (lo, hi)) {
                continue;
            }
            let mut shared = Vec::with_capacity(2);
            if hi == lo + 1 {
                shared.push(point_at(&edges[lo], 1.0));
            }
            if closed && lo == 0 && hi == n - 1 {
                shared.push(point_at(&edges[hi], 1.0));
            }
            let mut first: Option<(f64, Vec2)> = overlap(&edges[lo], &edges[hi]);
            for h in intersect_edges(&edges[lo], &edges[hi]) {
                if shared.iter().any(|&s| !far(s, h.p)) {
                    continue;
                }
                if first.is_none_or(|(t, _)| h.t < t) {
                    first = Some((h.t, h.p));
                }
            }
            if let Some((_, p)) = first {
                best = Some(((lo, hi), p));
            }
        }
    }
    best.map(|(_, p)| p)
}

fn ring_problems(r: &Ring, out: &mut Vec<Problem>) {
    if let Some(at) = repeated(&r.pts, true) {
        out.push(Problem {
            kind: Kind::Repeated,
            at,
        });
    }
    if on_one_line(r) {
        if let Some(&at) = r.pts.first() {
            out.push(Problem {
                kind: Kind::ZeroArea,
                at,
            });
        }
        return;
    }
    let edges: Vec<Edge> = ring_edges(r)
        .into_iter()
        .filter(|e| far(point_at(e, 0.0), point_at(e, 1.0)))
        .collect();
    if let Some(at) = crossing(&edges, true) {
        out.push(Problem {
            kind: Kind::RingCrossing,
            at,
        });
    }
}

/// Whether `p` is on one of the edges, within the tolerance.
fn on_edges(edges: &[Edge], p: Vec2) -> bool {
    edges.iter().any(|e| closest_on_edge(e, p).d <= TOL)
}

/// The first place where one ring's edges properly cross another's: along
/// `a`'s edges in order, the first along the edge; a meeting at either
/// ring's vertex is a touch, not a crossing.
fn rings_cross(a: &[Edge], b: &[Edge], vertices: &[Vec2]) -> Option<Vec2> {
    for ea in a {
        let ba = edge_box(ea);
        let mut first: Option<(f64, Vec2)> = None;
        for eb in b {
            if !boxes_meet(&ba, &edge_box(eb)) {
                continue;
            }
            for h in intersect_edges(ea, eb) {
                if vertices.iter().all(|&v| far(v, h.p)) && first.is_none_or(|(t, _)| h.t < t) {
                    first = Some((h.t, h.p));
                }
            }
        }
        if let Some((_, p)) = first {
            return Some(p);
        }
    }
    None
}

fn area_problems(a: &Area, out: &mut Vec<Problem>) {
    ring_problems(&a.outer, out);
    for h in &a.holes {
        ring_problems(h, out);
    }
    let outer = ring_edges(&a.outer);
    let outer_index = WindingIndex::new(&outer);
    let holes: Vec<Vec<Edge>> = a.holes.iter().map(ring_edges).collect();
    // A hole's first vertex out of the ring (on it is fine), or its edges across the ring's.
    let out_of = a.holes.iter().zip(&holes).find_map(|(h, edges)| {
        h.pts
            .iter()
            .copied()
            .find(|&p| outer_index.winding(p) == 0.0 && !on_edges(&outer, p))
            .or_else(|| {
                let vertices: Vec<Vec2> = h.pts.iter().chain(&a.outer.pts).copied().collect();
                rings_cross(edges, &outer, &vertices)
            })
    });
    if let Some(at) = out_of {
        out.push(Problem {
            kind: Kind::HoleOutside,
            at,
        });
    }
    let indexes: Vec<WindingIndex<'_>> = holes.iter().map(|h| WindingIndex::new(h)).collect();
    let strictly_inside =
        |k: usize, p: Vec2| indexes[k].winding(p) != 0.0 && !on_edges(&holes[k], p);
    'pairs: for i in 0..holes.len() {
        for j in i + 1..holes.len() {
            let vertices: Vec<Vec2> = a.holes[i]
                .pts
                .iter()
                .chain(&a.holes[j].pts)
                .copied()
                .collect();
            let at = rings_cross(&holes[i], &holes[j], &vertices)
                .or_else(|| {
                    a.holes[i]
                        .pts
                        .iter()
                        .copied()
                        .find(|&p| strictly_inside(j, p))
                })
                .or_else(|| {
                    a.holes[j]
                        .pts
                        .iter()
                        .copied()
                        .find(|&p| strictly_inside(i, p))
                });
            if let Some(at) = at {
                out.push(Problem {
                    kind: Kind::HolesOverlap,
                    at,
                });
                break 'pairs;
            }
        }
    }
}

/// A path's vertices as written.
fn path_points(s: &Shape) -> Vec<Vec2> {
    match s {
        Shape::Polyline { pts, .. } => pts.clone(),
        Shape::Line { a, b } => vec![*a, *b],
        _ => {
            let edges = entity_edges(s);
            edges
                .iter()
                .map(|e| point_at(e, 0.0))
                .chain(edges.last().map(|e| point_at(e, 1.0)))
                .collect()
        }
    }
}

fn path_problems(part: &Shape, out: &mut Vec<Problem>) {
    let pts = path_points(part);
    if let Some(at) = repeated(&pts, false) {
        out.push(Problem {
            kind: Kind::Repeated,
            at,
        });
    }
    let edges: Vec<Edge> = entity_edges(part)
        .into_iter()
        .filter(|e| far(point_at(e, 0.0), point_at(e, 1.0)))
        .collect();
    // A path that ends where it starts (a boundary drawn as a polyline) closes there.
    let closed = edges.len() > 1
        && !far(
            point_at(&edges[0], 0.0),
            point_at(&edges[edges.len() - 1], 1.0),
        );
    if let Some(at) = crossing(&edges, closed) {
        out.push(Problem {
            kind: Kind::PathCrossing,
            at,
        });
    }
}

/// What is wrong with an object as written: its areas' rings and holes, or
/// its paths; part by part, in the order written.
pub fn problems(s: &Shape) -> Vec<Problem> {
    let mut out = Vec::new();
    match super::class_of(s) {
        Class::Areas(_) => {
            for part in area_parts(s).iter() {
                for a in areas_of_entity(part) {
                    area_problems(&a, &mut out);
                }
            }
        }
        Class::Paths(_) => {
            for part in area_parts(s).iter() {
                path_problems(part, &mut out);
            }
        }
        _ => {}
    }
    out
}

/// What Onar made of an object: its new shape (none: nothing is left), and
/// its parts, holes, vertices and area (m²; paths: none) before and after.
#[derive(Clone, Debug, PartialEq)]
pub struct Repaired {
    pub shape: Option<Shape>,
    pub parts: (usize, usize),
    pub holes: (usize, usize),
    pub vertices: (usize, usize),
    pub area: Option<(f64, f64)>,
}

/// The rings as one overlay source, each as written.
fn source(rings: &[Ring]) -> Source {
    let mut edges = Vec::new();
    let mut points = Vec::new();
    for r in rings {
        edges.extend(ring_edges(r));
        points.extend_from_slice(&r.pts);
    }
    Source {
        edges,
        points: Some(points),
        cut: None,
    }
}

/// The area as written: its rings' sizes, the holes taken away.
fn written_area(areas: &[Area]) -> f64 {
    areas
        .iter()
        .map(|a| {
            ring_area(&a.outer).abs() - a.holes.iter().map(|h| ring_area(h).abs()).sum::<f64>()
        })
        .sum()
}

/// Onar: an area rebuilt from its own rings, a path without repeated
/// vertices; an object without problems as it is.
pub fn repair(s: &Shape) -> Repaired {
    let class = super::class_of(s);
    let count = super::written_vertices(s);
    if problems(s).is_empty() {
        let (parts, holes, area) = match &class {
            Class::Areas(a) => (
                a.len(),
                a.iter().map(|x| x.holes.len()).sum(),
                Some(super::areas_measure(a)),
            ),
            Class::Paths(_) => (area_parts(s).len(), 0, None),
            Class::Points(p) => (p.len(), 0, None),
            Class::None => (0, 0, None),
        };
        return Repaired {
            shape: (!class.is_none()).then(|| s.clone()),
            parts: (parts, parts),
            holes: (holes, holes),
            vertices: (count, count),
            area: area.map(|a| (a, a)),
        };
    }
    match class {
        Class::Areas(areas) => {
            let mut rebuilt = Vec::new();
            for a in &areas {
                // The ring as written (each loop of a crossing ring is inside
                // it), less its holes, each turned counter-clockwise so that
                // overlapping holes are one.
                let outer = source(std::slice::from_ref(&a.outer));
                if a.holes.is_empty() {
                    rebuilt.extend(overlay(&[outer], Rule::Any));
                } else {
                    let holes: Vec<Ring> = a.holes.iter().map(|h| orient_ring(h, true)).collect();
                    rebuilt.extend(overlay(&[outer, source(&holes)], Rule::FirstNotOthers));
                }
            }
            let joined = union_all(&rebuilt);
            let parts = (areas.len(), joined.len());
            let holes = (
                areas.iter().map(|a| a.holes.len()).sum(),
                joined.iter().map(|a| a.holes.len()).sum(),
            );
            let area = Some((written_area(&areas), super::areas_measure(&joined)));
            let shape = shape_of(&Class::Areas(joined));
            Repaired {
                vertices: (count, shape.as_ref().map_or(0, super::written_vertices)),
                shape,
                parts,
                holes,
                area,
            }
        }
        Class::Paths(paths) => {
            // The paths as the tools take them: without their repeated vertices.
            let shapes: Vec<Shape> = paths.iter().filter_map(|p| path_shape(p)).collect();
            let shape = crate::entity::join_parts(&shapes);
            Repaired {
                parts: (area_parts(s).len(), shapes.len()),
                holes: (0, 0),
                vertices: (count, shape.as_ref().map_or(0, super::written_vertices)),
                area: None,
                shape,
            }
        }
        _ => Repaired {
            shape: None,
            parts: (0, 0),
            holes: (0, 0),
            vertices: (0, 0),
            area: None,
        },
    }
}
