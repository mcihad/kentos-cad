//! Yakınlık analizi (docs/adr/0215 §2): two objects as Konuma göre seç reads
//! them (`spatial_query::Geometry`) and how near they are: their nearest
//! points, whose distance is the query's `distance` (0 when they meet), the
//! distance of their centres, and the boundary two areas share within a
//! tolerance (straight edges on one line, arcs of one circle). The store
//! asks these for each candidate pair (`Store::nearest`, `Store::neighbors`);
//! the independent reference is scripts/fixtures/proximity_cases.py
//! (fixtures/proximity/v1).

use crate::geom::arrangement::edge_box;
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, point_at};
use crate::geometry::{Bounds, dist};
use crate::jsmath::{PI, js_hypot, js_max, js_min};
use crate::ops::geoprocess;
use crate::ops::spatial_query::{
    Geometry, TOLERANCE, arc_toward, boxes_meet, edge_gap, ends, inside, intersects, meets_point,
    strictly_inside,
};
use crate::vec2::Vec2;

/// The nearest points of two objects, one on each, and their distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nearest {
    pub d: f64,
    pub a: Vec2,
    pub b: Vec2,
}

/// A point where two objects that meet meet, looked for as `intersects`
/// looks: their edges, then their points, then one inside the other's area.
fn meeting_point(a: &Geometry, b: &Geometry) -> Option<Vec2> {
    for e1 in &a.edges {
        for e2 in &b.edges {
            if let Some(h) = intersect_edges(e1, e2).first() {
                return Some(h.p);
            }
            if edge_gap(e1, e2) <= TOLERANCE {
                return Some(nearest_of_edges(e1, e2).a);
            }
        }
    }
    if let Some(&p) = a.points.iter().find(|&&p| meets_point(b, p)) {
        return Some(p);
    }
    if let Some(&q) = b.points.iter().find(|&&q| meets_point(a, q)) {
        return Some(q);
    }
    a.starts
        .iter()
        .find(|&&p| inside(&b.areas, p))
        .or_else(|| b.starts.iter().find(|&&p| inside(&a.areas, p)))
        .copied()
}

/// The nearest points of two edges that do not cross: the gap `edge_gap`
/// takes and where it is, the first of equal ones in its order.
fn nearest_of_edges(e1: &Edge, e2: &Edge) -> Nearest {
    let mut best = Nearest {
        d: f64::INFINITY,
        a: Vec2::new(0.0, 0.0),
        b: Vec2::new(0.0, 0.0),
    };
    let mut take = |d: f64, a: Vec2, b: Vec2| {
        if d < best.d {
            best = Nearest { d, a, b };
        }
    };
    for p in ends(e1) {
        let c = closest_on_edge(e2, p);
        take(c.d, p, c.p);
    }
    for q in ends(e2) {
        let c = closest_on_edge(e1, q);
        take(c.d, c.p, q);
    }
    for (arc, other, first) in [(e1, e2, true), (e2, e1, false)] {
        if let Edge::Arc { c, .. } = *arc {
            let q = closest_on_edge(other, c).p;
            if let Some(p) = arc_toward(arc, q) {
                let on = closest_on_edge(other, p);
                if first {
                    take(on.d, p, on.p);
                } else {
                    take(on.d, on.p, p);
                }
            }
        }
    }
    best
}

/// The nearest points of `a` and `b`, the first of equal ones in the order
/// `spatial_query::distance` looks (points, then edges); none when either
/// has nothing to measure. Objects that meet are 0 apart at a meeting point.
pub fn nearest_points(a: &Geometry, b: &Geometry) -> Option<Nearest> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    if intersects(a, b) {
        let m = meeting_point(a, b)?;
        return Some(Nearest { d: 0.0, a: m, b: m });
    }
    let mut best = Nearest {
        d: f64::INFINITY,
        a: Vec2::new(0.0, 0.0),
        b: Vec2::new(0.0, 0.0),
    };
    let mut take = |n: Nearest| {
        if n.d < best.d {
            best = n;
        }
    };
    for &p in &a.points {
        for &q in &b.points {
            take(Nearest {
                d: dist(p, q),
                a: p,
                b: q,
            });
        }
        for e in &b.edges {
            let c = closest_on_edge(e, p);
            take(Nearest {
                d: c.d,
                a: p,
                b: c.p,
            });
        }
    }
    for &q in &b.points {
        for e in &a.edges {
            let c = closest_on_edge(e, q);
            take(Nearest {
                d: c.d,
                a: c.p,
                b: q,
            });
        }
    }
    for e1 in &a.edges {
        for e2 in &b.edges {
            take(nearest_of_edges(e1, e2));
        }
    }
    best.d.is_finite().then_some(best)
}

/// The distance of the two objects' centres (`$merkez`); none without one.
pub fn center_distance(a: &Geometry, b: &Geometry) -> Option<f64> {
    Some(dist(a.center?, b.center?))
}

/// What two areas' boundaries share.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shared {
    /// The length they share, metres.
    pub length: f64,
    /// Whether they meet at all (within the tolerance).
    pub meet: bool,
}

/// An arc's angles as an interval going round counter-clockwise: its start
/// and its extent.
fn arc_span(a0: f64, sweep: f64) -> (f64, f64) {
    if sweep < 0.0 {
        (a0 + sweep, -sweep)
    } else {
        (a0, sweep)
    }
}

/// How much of two angle intervals (start, extent) overlap, going round.
fn angle_overlap((s1, w1): (f64, f64), (s2, w2): (f64, f64)) -> f64 {
    let tau = 2.0 * PI;
    let mut best: f64 = 0.0;
    // The second interval shifted by whole turns to sit by the first.
    let base = s2 - ((s2 - s1) / tau).floor() * tau;
    for shift in [-tau, 0.0, tau] {
        let a = js_max(s1, base + shift);
        let b = js_min(s1 + w1, base + shift + w2);
        best += js_max(0.0, b - a);
    }
    js_min(best, js_min(w1, w2))
}

/// The length two edges share within `tol`: straight ones on one line
/// (the shorter one's ends within `tol` of the longer one's line) over
/// the part their projections share; arcs of one circle over the angles
/// they share.
pub fn edge_overlap(e1: &Edge, e2: &Edge, tol: f64) -> f64 {
    match (*e1, *e2) {
        (Edge::Seg { a: a1, b: b1 }, Edge::Seg { a: a2, b: b2 }) => {
            let (l1, l2) = (dist(a1, b1), dist(a2, b2));
            let ((p, q), (r, s), l) = if l1 >= l2 {
                ((a1, b1), (a2, b2), l1)
            } else {
                ((a2, b2), (a1, b1), l2)
            };
            if l <= 0.0 {
                return 0.0;
            }
            let (ux, uy) = ((q.x - p.x) / l, (q.y - p.y) / l);
            let off = |v: Vec2| ((v.x - p.x) * uy - (v.y - p.y) * ux).abs();
            if off(r) > tol || off(s) > tol {
                return 0.0;
            }
            let along = |v: Vec2| (v.x - p.x) * ux + (v.y - p.y) * uy;
            let (t0, t1) = (along(r), along(s));
            let lo = js_max(0.0, js_min(t0, t1));
            let hi = js_min(l, js_max(t0, t1));
            js_max(0.0, hi - lo)
        }
        (
            Edge::Arc {
                c: c1,
                r: r1,
                a0: s1,
                sweep: w1,
            },
            Edge::Arc {
                c: c2,
                r: r2,
                a0: s2,
                sweep: w2,
            },
        ) => {
            if js_hypot(c1.x - c2.x, c1.y - c2.y) > tol || (r1 - r2).abs() > tol {
                return 0.0;
            }
            let overlap = angle_overlap(arc_span(s1, w1), arc_span(s2, w2));
            overlap * (r1 + r2) / 2.0
        }
        _ => 0.0,
    }
}

/// What the boundaries of two areas share within `tol`: the length their
/// edges share (every pair's, holes and parts too) and whether they meet.
pub fn shared_boundary(a: &Geometry, b: &Geometry, tol: f64) -> Shared {
    let mut shared = Shared::default();
    if !boxes_meet(&a.bounds, &b.bounds, tol) {
        return shared;
    }
    // Edges whose boxes are farther apart than the tolerance share nothing
    // and do not meet; a little more lets the crossing test's slack in.
    let pad = tol + PAD;
    for e1 in &a.area_edges {
        let b1 = edge_box(e1);
        if !boxes_meet(&b1, &b.bounds, pad) {
            continue;
        }
        for e2 in &b.area_edges {
            if !boxes_meet(&b1, &edge_box(e2), pad) {
                continue;
            }
            shared.length += edge_overlap(e1, e2, tol);
            if !shared.meet && (!intersect_edges(e1, e2).is_empty() || edge_gap(e1, e2) <= tol) {
                shared.meet = true;
            }
        }
    }
    shared
}

/// How far inside an area its edges' probes are, metres.
const PROBE: f64 = 0.01;

/// What the edge boxes' tests add to let the intersection's own slack in.
const PAD: f64 = 1e-6;

/// A point just inside the area by the middle of each of its edges, where
/// the area is not thinner than the step there; only by the edges whose
/// middles are within the step of `near` (no other probe can fall in it).
fn probes<'a>(g: &'a Geometry, near: &'a Bounds) -> impl Iterator<Item = Vec2> + 'a {
    g.area_edges.iter().filter_map(move |e| {
        let m = point_at(e, 0.5);
        if !within(&padded_box(near, PROBE), m) {
            return None;
        }
        let (nx, ny) = match *e {
            Edge::Seg { a, b } => {
                let l = dist(a, b);
                if l <= 4.0 * PROBE {
                    return None;
                }
                ((a.y - b.y) / l, (b.x - a.x) / l)
            }
            Edge::Arc { c, r, .. } => ((m.x - c.x) / r, (m.y - c.y) / r),
        };
        [1.0, -1.0]
            .into_iter()
            .map(|side| Vec2::new(m.x + side * PROBE * nx, m.y + side * PROBE * ny))
            .find(|&p| inside(&g.areas, p))
    })
}

fn padded_box(b: &Bounds, pad: f64) -> Bounds {
    Bounds {
        min_x: b.min_x - pad,
        min_y: b.min_y - pad,
        max_x: b.max_x + pad,
        max_y: b.max_y + pad,
    }
}

/// Whether `p` is in the box, its edges included.
fn within(b: &Bounds, p: Vec2) -> bool {
    p.x >= b.min_x && p.x <= b.max_x && p.y >= b.min_y && p.y <= b.max_y
}

/// Whether the insides of two areas overlap (docs/adr/0215 §2.2): a corner
/// of one inside the other, their boundaries crossing (not at their ends),
/// or a point just inside one by the middle of an edge inside the other.
/// Boxes narrow every test first: an inside lies within its box's inside, a
/// point strictly inside an area within its box, a crossing within both
/// edges' boxes.
pub fn interiors_overlap(a: &Geometry, b: &Geometry) -> bool {
    if a.areas.is_empty() || b.areas.is_empty() || !boxes_meet(&a.bounds, &b.bounds, 0.0) {
        return false;
    }
    let (p, q) = (&a.bounds, &b.bounds);
    if js_min(p.max_x, q.max_x) <= js_max(p.min_x, q.min_x)
        || js_min(p.max_y, q.max_y) <= js_max(p.min_y, q.min_y)
    {
        return false;
    }
    let corner_inside = |g: &Geometry, other: &Geometry| {
        g.areas
            .iter()
            .flat_map(|area| std::iter::once(&area.outer).chain(area.holes.iter()))
            .flat_map(|r| r.pts.iter())
            .any(|&p| within(&other.bounds, p) && strictly_inside(other, p))
    };
    if corner_inside(a, b) || corner_inside(b, a) {
        return true;
    }
    const EPS: f64 = 1e-9;
    let crossing = |t: f64| t > EPS && t < 1.0 - EPS;
    for e1 in &a.area_edges {
        let b1 = edge_box(e1);
        if !boxes_meet(&b1, &b.bounds, PAD) {
            continue;
        }
        for e2 in &b.area_edges {
            if boxes_meet(&b1, &edge_box(e2), PAD)
                && intersect_edges(e1, e2)
                    .iter()
                    .any(|h| crossing(h.t) && crossing(h.u))
            {
                return true;
            }
        }
    }
    probes(a, &b.bounds).any(|p| within(&b.bounds, p) && strictly_inside(b, p))
        || probes(b, &a.bounds).any(|p| within(&a.bounds, p) && strictly_inside(a, p))
}

/// The area two areas' insides share, m² (the overlay's, docs/adr/0201).
pub fn overlap_area(a: &Geometry, b: &Geometry) -> f64 {
    geoprocess::areas_measure(&geoprocess::intersect_all(&a.areas, &b.areas))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Shape;
    use crate::ops::spatial_query::{distance, geometry_of};

    fn square(x: f64, y: f64, s: f64) -> Geometry {
        geometry_of(&Shape::Polygon {
            pts: vec![
                Vec2::new(x, y),
                Vec2::new(x + s, y),
                Vec2::new(x + s, y + s),
                Vec2::new(x, y + s),
            ],
            bulges: None,
            holes: None,
            parts: None,
        })
        .expect("a geometry")
    }

    #[test]
    fn the_nearest_points_give_the_query_s_distance() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(13.0, 4.0, 2.0);
        let n = nearest_points(&a, &b).expect("both measure");
        assert_eq!(n.d, distance(&a, &b));
        assert_eq!(n.d, 3.0);
        assert_eq!((n.a, n.b), (Vec2::new(10.0, 4.0), Vec2::new(13.0, 4.0)));
        // A point inside the area: 0 apart, where the point is.
        let p = geometry_of(&Shape::Point {
            p: Vec2::new(5.0, 5.0),
            z: None,
            parts: None,
        })
        .expect("a geometry");
        let n = nearest_points(&p, &a).expect("both measure");
        assert_eq!((n.d, n.a), (0.0, Vec2::new(5.0, 5.0)));
        // Centres.
        assert_eq!(center_distance(&a, &square(20.0, 0.0, 10.0)), Some(20.0));
    }

    #[test]
    fn shared_boundaries_are_their_common_length() {
        let a = square(0.0, 0.0, 10.0);
        // Side by side, half of the side in common.
        let b = square(10.0, 5.0, 10.0);
        let s = shared_boundary(&a, &b, TOLERANCE);
        assert!((s.length - 5.0).abs() < 1e-12 && s.meet, "{s:?}");
        // Corner to corner: they meet, nothing in common.
        let c = square(10.0, 10.0, 5.0);
        assert_eq!(
            shared_boundary(&a, &c, TOLERANCE),
            Shared {
                length: 0.0,
                meet: true
            }
        );
        // Apart.
        assert_eq!(
            shared_boundary(&a, &square(11.0, 0.0, 5.0), TOLERANCE),
            Shared::default()
        );
        // Arcs of one circle share the angles they both cover.
        let arc = |a0: f64, sweep: f64| Edge::Arc {
            c: Vec2::new(0.0, 0.0),
            r: 2.0,
            a0,
            sweep,
        };
        let shared = edge_overlap(&arc(0.0, PI), &arc(PI / 2.0, -PI), 1e-9);
        assert!((shared - PI).abs() < 1e-12, "{shared}");
        // Across the turn: [−0.5, 0.5] and [6, 7] ≡ [6 − 2π, 7 − 2π].
        let across = edge_overlap(&arc(-0.5, 1.0), &arc(6.0, 1.0), 1e-9);
        assert!(
            (across - 2.0 * (0.5 - (6.0 - 2.0 * PI))).abs() < 1e-9,
            "{across}"
        );
    }
}
