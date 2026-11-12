//! How far a shape moved between two of its states (plan/map-cad.md MAP-01, docs/adr/4880): an object as approved and
//! as it is now.
//!
//! - **Vertex by vertex:** when both states are of the same kind and have as many vertices (`entity_vertices`, in its
//!   order), the vertices pair by their number: each vertex's move (Δeast, Δnorth, its length), the largest and its
//!   number (the first of equal ones), the root mean square of the moves.
//! - **Hausdorff distance** of the two outlines (`entity_edges`; a shape without edges is its vertices): the larger of
//!   the two directed distances, the farthest a point of one outline is from the other outline, and the two points
//!   where it is reached. Exact to [`HAUSDORFF_TOLERANCE`]: each edge of one outline is walked along its length; the
//!   distance to the other outline changes at most by the length walked (it is 1-Lipschitz), so a stretch whose ends'
//!   distances are `f0`, `f1` and length `L` holds no point farther than `(f0 + f1 + L) / 2`. Stretches that may still
//!   hold a farther point than the farthest found are halved until none can by more than the tolerance. Arcs are
//!   walked on their circle, never as chords.
//!
//! An outline within a tolerance `t` of another all along, both ways, is one whose Hausdorff distance is at most `t`:
//! the same as asking whether each lies in a band of half-width `t` around the other.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::entity::{Shape, entity_vertices};
use crate::geom::intersect::Edge;
use crate::jsmath::{TAU, atan2, cos, sin};
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

/// The Hausdorff distance is found to within this many metres (a millionth of a millimetre).
pub const HAUSDORFF_TOLERANCE: f64 = 1e-9;
/// At most this many stretches are measured for one outline; past it the farthest found stands (a bound, never reached
/// by a drawn object's outline).
const MOST_STRETCHES: usize = 2_000_000;

/// One vertex's move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertexMove {
    /// Its number in the shape's vertices, from 0.
    pub index: usize,
    pub dx: f64,
    pub dy: f64,
    pub d: f64,
}

/// The Hausdorff distance and where it is reached: `from` on the approved outline, `to` the nearest point of the
/// outline now (or the other way round, whichever way is the larger).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Farthest {
    pub d: f64,
    pub from: Vec2,
    pub to: Vec2,
}

/// How a shape moved.
#[derive(Clone, Debug, PartialEq)]
pub struct Shift {
    /// The two states' vertices pair (same kind, as many vertices).
    pub paired: bool,
    /// How many vertices each state has.
    pub vertices: [usize; 2],
    /// The largest vertex move, when paired.
    pub largest: Option<VertexMove>,
    /// The vertex moves' root mean square, when paired.
    pub rms: Option<f64>,
    /// The outlines' Hausdorff distance; none when either has no point.
    pub hausdorff: Option<Farthest>,
}

fn dist(p: Vec2, q: Vec2) -> f64 {
    let (dx, dy) = (p.x - q.x, p.y - q.y);
    (dx * dx + dy * dy).sqrt()
}

fn length(e: &Edge) -> f64 {
    match *e {
        Edge::Seg { a, b } => dist(a, b),
        Edge::Arc { r, sweep, .. } => r * sweep.abs(),
    }
}

/// The point at `s` along the edge (0 at its start).
fn point_at(e: &Edge, s: f64) -> Vec2 {
    match *e {
        Edge::Seg { a, b } => {
            let l = dist(a, b);
            if l == 0.0 {
                return a;
            }
            let t = s / l;
            Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
        }
        Edge::Arc { c, r, a0, sweep } => {
            let ang = a0 + sweep.signum() * s / r;
            Vec2::new(c.x + r * cos(ang), c.y + r * sin(ang))
        }
    }
}

/// The nearest point of the edge to `p`.
fn nearest_on(e: &Edge, p: Vec2) -> Vec2 {
    match *e {
        Edge::Seg { a, b } => {
            let (vx, vy) = (b.x - a.x, b.y - a.y);
            let l2 = vx * vx + vy * vy;
            if l2 == 0.0 {
                return a;
            }
            let t = (((p.x - a.x) * vx + (p.y - a.y) * vy) / l2).clamp(0.0, 1.0);
            Vec2::new(a.x + vx * t, a.y + vy * t)
        }
        Edge::Arc { c, r, a0, sweep } => {
            let start = Vec2::new(c.x + r * cos(a0), c.y + r * sin(a0));
            let end = Vec2::new(c.x + r * cos(a0 + sweep), c.y + r * sin(a0 + sweep));
            let k = dist(p, c);
            if k > 0.0 {
                let t = atan2(p.y - c.y, p.x - c.x);
                // How far round from the start, in the arc's own direction.
                let u = if sweep >= 0.0 { (t - a0).rem_euclid(TAU) } else { (a0 - t).rem_euclid(TAU) };
                if u <= sweep.abs() {
                    return Vec2::new(c.x + (p.x - c.x) * r / k, c.y + (p.y - c.y) * r / k);
                }
            }
            if dist(p, start) <= dist(p, end) { start } else { end }
        }
    }
}

/// An outline: its edges, else its points.
struct Outline {
    edges: Vec<Edge>,
    points: Vec<Vec2>,
}

impl Outline {
    fn of(s: &Shape) -> Self {
        let edges = entity_edges(s);
        let points = if edges.is_empty() { entity_vertices(s) } else { Vec::new() };
        Outline { edges, points }
    }

    fn is_empty(&self) -> bool {
        self.edges.is_empty() && self.points.is_empty()
    }

    /// The distance from `p` to the outline and its nearest point.
    fn nearest(&self, p: Vec2) -> (f64, Vec2) {
        let mut best = (f64::INFINITY, p);
        for q in self.edges.iter().map(|e| nearest_on(e, p)).chain(self.points.iter().copied()) {
            let d = dist(p, q);
            if d < best.0 {
                best = (d, q);
            }
        }
        best
    }
}

/// A stretch of an edge and how far from the other outline its ends are.
struct Stretch {
    bound: f64,
    edge: usize,
    s0: f64,
    s1: f64,
    f0: f64,
    f1: f64,
}

impl PartialEq for Stretch {
    fn eq(&self, other: &Self) -> bool {
        self.bound.total_cmp(&other.bound) == Ordering::Equal
    }
}
impl Eq for Stretch {}
impl PartialOrd for Stretch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Stretch {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bound.total_cmp(&other.bound)
    }
}

/// The farthest a point of `a` is from `b` (the directed Hausdorff distance), with the two points.
fn directed(a: &Outline, b: &Outline) -> Farthest {
    let mut best = Farthest { d: -1.0, from: Vec2::default(), to: Vec2::default() };
    let take = |p: Vec2, best: &mut Farthest| -> f64 {
        let (d, q) = b.nearest(p);
        if d > best.d {
            *best = Farthest { d, from: p, to: q };
        }
        d
    };
    for p in &a.points {
        take(*p, &mut best);
    }
    let mut heap = BinaryHeap::new();
    for (i, e) in a.edges.iter().enumerate() {
        let l = length(e);
        let f0 = take(point_at(e, 0.0), &mut best);
        let f1 = take(point_at(e, l), &mut best);
        heap.push(Stretch { bound: (f0 + f1 + l) / 2.0, edge: i, s0: 0.0, s1: l, f0, f1 });
    }
    let mut measured = 0usize;
    while let Some(st) = heap.pop() {
        if st.bound <= best.d + HAUSDORFF_TOLERANCE || measured >= MOST_STRETCHES {
            break;
        }
        measured += 1;
        let e = &a.edges[st.edge];
        let mid = (st.s0 + st.s1) / 2.0;
        if mid <= st.s0 || mid >= st.s1 {
            continue;
        }
        let fm = take(point_at(e, mid), &mut best);
        for (s0, s1, f0, f1) in [(st.s0, mid, st.f0, fm), (mid, st.s1, fm, st.f1)] {
            let bound = (f0 + f1 + (s1 - s0)) / 2.0;
            if bound > best.d + HAUSDORFF_TOLERANCE {
                heap.push(Stretch { bound, edge: st.edge, s0, s1, f0, f1 });
            }
        }
    }
    best
}

/// The Hausdorff distance of two shapes' outlines; none when either has no point.
pub fn hausdorff(then: &Shape, now: &Shape) -> Option<Farthest> {
    let (a, b) = (Outline::of(then), Outline::of(now));
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let ab = directed(&a, &b);
    let ba = directed(&b, &a);
    // The way from the approved outline first; the other way only when it is farther.
    Some(if ba.d > ab.d { Farthest { d: ba.d, from: ba.to, to: ba.from } } else { ab })
}

/// Whether two shapes are of one kind (their tag).
fn same_kind(a: &Shape, b: &Shape) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

/// How `then` moved to `now`.
pub fn shift(then: &Shape, now: &Shape) -> Shift {
    let (va, vb) = (entity_vertices(then), entity_vertices(now));
    let paired = same_kind(then, now) && va.len() == vb.len() && !va.is_empty();
    let (mut largest, mut rms) = (None, None);
    if paired {
        let mut sum = 0.0;
        let mut top: Option<VertexMove> = None;
        for (index, (p, q)) in va.iter().zip(&vb).enumerate() {
            let (dx, dy) = (q.x - p.x, q.y - p.y);
            let d = (dx * dx + dy * dy).sqrt();
            sum += d * d;
            if top.is_none_or(|t| d > t.d) {
                top = Some(VertexMove { index, dx, dy, d });
            }
        }
        largest = top;
        rms = Some((sum / va.len() as f64).sqrt());
    }
    Shift { paired, vertices: [va.len(), vb.len()], largest, rms, hausdorff: hausdorff(then, now) }
}

/// How many vertices moved farther than `limit` (docs/adr/5280), when they pair one to one; none otherwise.
pub fn vertices_beyond(then: &Shape, now: &Shape, limit: f64) -> Option<usize> {
    let (va, vb) = (entity_vertices(then), entity_vertices(now));
    if !(same_kind(then, now) && va.len() == vb.len() && !va.is_empty()) {
        return None;
    }
    let d = |p: &Vec2, q: &Vec2| {
        let (dx, dy) = (q.x - p.x, q.y - p.y);
        (dx * dx + dy * dy).sqrt()
    };
    Some(va.iter().zip(&vb).filter(|(p, q)| d(p, q) > limit).count())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(pts: &[(f64, f64)], bulges: Option<Vec<f64>>, closed: bool) -> Shape {
        Shape::Polyline {
            pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
            bulges,
            holes: None,
            parts: None,
            closed,
            widths: None,
        }
    }

    #[test]
    fn a_moved_vertex_is_the_largest_move_and_the_hausdorff_distance() {
        let a = poly(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], None, false);
        let b = poly(&[(0.0, 0.0), (10.3, 0.4), (10.0, 10.0)], None, false);
        let s = shift(&a, &b);
        assert!(s.paired);
        let top = s.largest.expect("paired");
        assert_eq!(top.index, 1);
        assert!((top.d - 0.5).abs() < 1e-12);
        assert!((s.rms.expect("paired") - (0.25f64 / 3.0).sqrt()).abs() < 1e-12);
        // The moved corner is the farthest point of either outline from the other: its distance to the old outline.
        let h = s.hausdorff.expect("outlines");
        assert!(h.d <= 0.5 + 1e-12 && h.d > 0.3, "{h:?}");
    }

    #[test]
    fn an_added_vertex_is_not_paired_and_its_distance_is_the_hausdorff() {
        let a = poly(&[(0.0, 0.0), (10.0, 0.0)], None, false);
        let b = poly(&[(0.0, 0.0), (5.0, 2.0), (10.0, 0.0)], None, false);
        let s = shift(&a, &b);
        assert!(!s.paired && s.largest.is_none());
        let h = s.hausdorff.expect("outlines");
        assert!((h.d - 2.0).abs() < 1e-9, "{h:?}");
        assert!((h.from.x - 5.0).abs() < 1e-6 && h.from.y.abs() < 1e-9, "on the approved line: {h:?}");
        assert!((h.to.x - 5.0).abs() < 1e-9 && (h.to.y - 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_farthest_point_between_vertices_is_found() {
        // A line under a bent one: its farthest point from it is where it is as far from both arms, between vertices.
        let a = poly(&[(0.0, 0.0), (10.0, 0.0)], None, false);
        let b = poly(&[(0.0, 0.0), (4.0, 10.0), (10.0, 0.0)], None, false);
        let d = directed(&Outline::of(&a), &Outline::of(&b));
        let want = 100.0 / (116f64.sqrt() + 136f64.sqrt());
        assert!((d.d - want).abs() < 1e-9, "{d:?} {want}");
        let x = 10.0 * 116f64.sqrt() / (116f64.sqrt() + 136f64.sqrt());
        assert!((d.from.x - x).abs() < 1e-6, "{d:?}");
    }

    #[test]
    fn concentric_arcs_are_their_radii_apart() {
        let arc = |r: f64| Shape::Arc { c: Vec2::new(100.0, 50.0), r, a0: 0.3, a1: 1.9 };
        let h = hausdorff(&arc(20.0), &arc(20.25)).expect("outlines");
        assert!((h.d - 0.25).abs() < 1e-9, "{h:?}");
    }

    #[test]
    fn points_are_their_distance_apart() {
        let p = |x: f64, y: f64| Shape::Point { p: Vec2::new(x, y), z: None, parts: None };
        let s = shift(&p(1.0, 1.0), &p(4.0, 5.0));
        assert!(s.paired);
        assert!((s.largest.expect("paired").d - 5.0).abs() < 1e-12);
        assert!((s.hausdorff.expect("points").d - 5.0).abs() < 1e-12);
    }
}
