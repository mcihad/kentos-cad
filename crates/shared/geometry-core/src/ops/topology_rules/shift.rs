//! How far a repair takes an object from its source geometry (docs/adr/1260 §4): each vertex the object had (a ring's
//! and a path's corners, a point's places) is kept when the new geometry has a vertex at exactly that place, bit for
//! bit; otherwise it moved, by its distance to the nearest point of the new geometry (its edges, arcs as arcs; its
//! points). New vertices a repair adds do not count: what is measured is the deviation from what was surveyed. Nothing
//! is rounded and no tolerance is taken: a vertex moved by less than the storage precision is still moved.

use super::written;
use crate::entity::Shape;
use crate::geom::intersect::closest_on_edge;
use crate::jsmath::{js_hypot, js_max};
use crate::vec2::Vec2;

/// An object's source vertices kept and moved by a repair, and the farthest and total distance moved.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shift {
    pub kept: usize,
    pub moved: usize,
    pub max: f64,
    pub total: f64,
}

impl Shift {
    /// The two together: a whole repair's over its objects.
    pub fn and(self, o: Shift) -> Shift {
        Shift {
            kept: self.kept + o.kept,
            moved: self.moved + o.moved,
            max: js_max(self.max, o.max),
            total: self.total + o.total,
        }
    }
}

/// A point object's places: its own and its parts' (docs/adr/0174).
fn points(s: &Shape) -> Vec<Vec2> {
    match s {
        Shape::Point { p, parts, .. } => std::iter::once(*p)
            .chain(parts.iter().flatten().map(|q| q.p))
            .collect(),
        _ => Vec::new(),
    }
}

/// The vertices a shape has, in its order: its rings' and paths' corners, its points.
fn vertices(s: &Shape) -> Vec<Vec2> {
    if let Shape::Point { .. } = s {
        return points(s);
    }
    written(s).into_iter().flat_map(|w| w.pts).collect()
}

/// The distance from `p` to the nearest point of `s`: of its edges or of its points; none when it has neither.
fn distance(s: &Shape, p: Vec2) -> Option<f64> {
    let mut best: Option<f64> = None;
    let mut take = |d: f64| best = Some(best.map_or(d, |b| if d < b { d } else { b }));
    if let Shape::Point { .. } = s {
        for q in points(s) {
            take(js_hypot(p.x - q.x, p.y - q.y));
        }
        return best;
    }
    for w in written(s) {
        if w.pts.len() == 1 {
            take(js_hypot(p.x - w.pts[0].x, p.y - w.pts[0].y));
        }
        for k in 0..w.edge_count() {
            take(closest_on_edge(&w.edge(k), p).d);
        }
    }
    best
}

/// How far `after` takes the vertices of `before` (a shape the repair keeps; a deleted one is not measured), and how
/// many of the moved ones moved by less than `under` (the storage precision; 0 counts none): they are still moved.
pub fn shift(before: &Shape, after: &Shape, under: f64) -> (Shift, usize) {
    let now = vertices(after);
    let mut out = Shift::default();
    let mut below = 0;
    for v in vertices(before) {
        if now
            .iter()
            .any(|q| q.x.to_bits() == v.x.to_bits() && q.y.to_bits() == v.y.to_bits())
        {
            out.kept += 1;
            continue;
        }
        let d = distance(after, v).unwrap_or(f64::INFINITY);
        out.moved += 1;
        out.max = js_max(out.max, d);
        out.total += d;
        if d < under {
            below += 1;
        }
    }
    (out, below)
}
