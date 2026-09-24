//! Linear approximations of curves within a chord tolerance: the PostGIS
//! projection of analytic CAD objects (CLAUDE.md §15). The analytic object
//! stays the source; these points only serve GIS queries, indexes and tiles.
//! `tol` is the largest distance allowed between the curve and a chord
//! (sagitta), in world units.

use std::f64::consts::{PI, TAU};

use crate::Vec2;
use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::jsmath::{acos, cos, js_hypot, js_min, sin};

/// Most chords one curve piece may get. Beyond it the tolerance is not met: at
/// 1 mm a full circle reaches the cap at about 1.3 km radius. The projection is
/// for GIS queries only; the analytic source is unaffected.
pub const MAX_SEGMENTS: usize = 4096;

/// Chords needed for a circular arc of radius `r` sweeping `|sweep|` radians.
pub fn arc_segments(r: f64, sweep: f64, tol: f64) -> usize {
    let sweep = sweep.abs();
    if sweep == 0.0 || r <= 0.0 {
        return 1;
    }
    // A chord of half-angle h leaves a sagitta r(1 − cos h); at least 8 chords per full turn.
    let step = if tol >= r {
        PI / 4.0
    } else {
        js_min(2.0 * acos(1.0 - tol / r), PI / 4.0)
    };
    ((sweep / step).ceil() as usize).clamp(1, MAX_SEGMENTS)
}

/// Points of an arc from angle `a0` over signed `sweep`, both ends included.
pub fn arc_points(c: Vec2, r: f64, a0: f64, sweep: f64, tol: f64) -> Vec<Vec2> {
    let n = arc_segments(r, sweep, tol);
    (0..=n)
        .map(|i| {
            let a = a0 + sweep * (i as f64) / (n as f64);
            Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
        })
        .collect()
}

/// A full circle as a ring (first point not repeated), counter-clockwise from angle 0.
pub fn circle_ring(c: Vec2, r: f64, tol: f64) -> Vec<Vec2> {
    let mut pts = arc_points(c, r, 0.0, TAU, tol);
    pts.pop();
    pts
}

/// A bulged path as points. Vertices keep their exact coordinates; arcs gain
/// inner points. A closed path is a ring whose first point is not repeated.
pub fn bulge_path(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool, tol: f64) -> Vec<Vec2> {
    let n = pts.len();
    if n == 0 {
        return Vec::new();
    }
    let segments = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(n);
    out.push(pts[0]);
    for i in 0..segments {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        if let Some(arc) = bulge_arc(a, b, bulge_at(bulges, i)) {
            let inner = arc_points(arc.c, arc.r, arc.a0, arc.sweep, tol);
            out.extend_from_slice(&inner[1..inner.len() - 1]);
        }
        if !(closed && i + 1 == n) {
            out.push(b);
        }
    }
    out
}

/// An ellipse or elliptical arc (DXF form). Returns the points and whether it
/// is closed (t0 = t1: a full ellipse, returned as a ring without repeating
/// the first point).
pub fn ellipse_points(
    c: Vec2,
    major: Vec2,
    ratio: f64,
    t0: f64,
    t1: f64,
    tol: f64,
) -> (Vec<Vec2>, bool) {
    let full = t0 == t1;
    let mut sweep = t1 - t0;
    if full {
        sweep = TAU;
    } else {
        sweep = sweep.rem_euclid(TAU);
        if sweep == 0.0 {
            sweep = TAU;
        }
    }
    // With even parameter steps dt the sagitta is about ds²/(8ρ): a·dt²/8 at the
    // major vertices (ds = b·dt, ρ = b²/a) and b·dt²/8 at the minor ones, so the
    // worst case is that of a circle with the major radius.
    let n = arc_segments(js_hypot(major.x, major.y), sweep, tol);
    let minor = Vec2::new(-major.y * ratio, major.x * ratio);
    let at = |t: f64| {
        Vec2::new(
            c.x + major.x * cos(t) + minor.x * sin(t),
            c.y + major.y * cos(t) + minor.y * sin(t),
        )
    };
    let last = if full { n - 1 } else { n };
    let pts = (0..=last)
        .map(|i| at(t0 + sweep * (i as f64) / (n as f64)))
        .collect();
    (pts, full)
}

// ── Centripetal Catmull-Rom (the same curve as apps/web/src/model/geom/spline.ts) ──

fn reflect(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(2.0 * b.x - a.x, 2.0 * b.y - a.y)
}

fn knot(a: Vec2, b: Vec2) -> f64 {
    let k = js_hypot(b.x - a.x, b.y - a.y).sqrt();
    if k == 0.0 { 1e-6 } else { k }
}

fn lerp(a: Vec2, b: Vec2, ta: f64, tb: f64, t: f64) -> Vec2 {
    let mut d = tb - ta;
    if d == 0.0 {
        d = 1e-12;
    }
    let u = (tb - t) / d;
    let v = (t - ta) / d;
    Vec2::new(a.x * u + b.x * v, a.y * u + b.y * v)
}

#[allow(clippy::too_many_arguments)]
fn eval_span(
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    t0: f64,
    t1: f64,
    t2: f64,
    t3: f64,
    t: f64,
) -> Vec2 {
    let a1 = lerp(p0, p1, t0, t1, t);
    let a2 = lerp(p1, p2, t1, t2, t);
    let a3 = lerp(p2, p3, t2, t3, t);
    let b1 = lerp(a1, a2, t0, t2, t);
    let b2 = lerp(a2, a3, t1, t3, t);
    lerp(b1, b2, t1, t2, t)
}

fn point_segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    };
    js_hypot(p.x - (a.x + dx * t), p.y - (a.y + dy * t))
}

/// The spline through `pts` as points. Each span is split evenly, doubling
/// the count (from 16) until every chord's midpoint lies within `tol` of the curve.
pub fn catmull_rom(pts: &[Vec2], closed: bool, tol: f64) -> Vec<Vec2> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }
    let at = |i: isize| -> Vec2 {
        if closed {
            return pts[i.rem_euclid(n as isize) as usize];
        }
        if i < 0 {
            return reflect(pts[1], pts[0]);
        }
        if i as usize >= n {
            return reflect(pts[n - 2], pts[n - 1]);
        }
        pts[i as usize]
    };
    let spans = if closed { n } else { n - 1 };
    let mut out = Vec::new();
    for i in 0..spans as isize {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let t1 = knot(p0, p1);
        let t2 = t1 + knot(p1, p2);
        let t3 = t2 + knot(p2, p3);
        let eval = |s: f64| eval_span(p0, p1, p2, p3, 0.0, t1, t2, t3, t1 + (t2 - t1) * s);
        let mut k = 16usize;
        loop {
            let ok = (0..k).all(|j| {
                let (a, b) = (eval(j as f64 / k as f64), eval((j + 1) as f64 / k as f64));
                point_segment_distance(eval((j as f64 + 0.5) / k as f64), a, b) <= tol
            });
            if ok || k >= MAX_SEGMENTS {
                break;
            }
            k *= 2;
        }
        for j in 0..k {
            out.push(if j == 0 {
                p1
            } else {
                eval(j as f64 / k as f64)
            });
        }
    }
    if !closed {
        out.push(pts[n - 1]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsmath::js_max;

    fn max_sagitta_circle(pts: &[Vec2], c: Vec2, r: f64) -> f64 {
        let mut worst: f64 = 0.0;
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let m = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            worst = js_max(worst, r - js_hypot(m.x - c.x, m.y - c.y));
        }
        worst
    }

    #[test]
    fn a_circle_keeps_within_the_chord_tolerance() {
        let c = Vec2::new(486_512.34, 4_420_210.5);
        for (r, tol) in [
            (5.0, 0.001),
            (250.0, 0.001),
            (1000.0, 0.001),
            (0.0005, 0.001),
        ] {
            let ring = circle_ring(c, r, tol);
            assert!(ring.len() >= 8, "r = {r}");
            assert!(max_sagitta_circle(&ring, c, r) <= tol * 1.000001, "r = {r}");
        }
        // Past the cap the chord count stays bounded instead of the tolerance.
        assert_eq!(circle_ring(c, 1e6, 0.001).len(), MAX_SEGMENTS);
    }

    #[test]
    fn a_bulged_path_keeps_its_vertices_exactly() {
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
        ];
        let open = bulge_path(&pts, Some(&[1.0, 0.0]), false, 0.001);
        assert_eq!(open[0], pts[0]);
        assert_eq!(*open.last().unwrap(), pts[2]);
        assert!(open.contains(&pts[1]));
        // The half circle below the first chord bulges to y = −5 at its middle.
        let lowest = open.iter().map(|p| p.y).fold(f64::MAX, js_min);
        assert!((-5.0..-4.999).contains(&lowest), "{lowest}");
        let ring = bulge_path(&pts, None, true, 0.001);
        assert_eq!(ring, pts.to_vec());
        let closed_arc = bulge_path(&pts, Some(&[0.0, 0.0, 0.5]), true, 0.001);
        assert_eq!(&closed_arc[..3], &pts[..]);
        assert!(closed_arc.len() > 3 && closed_arc[closed_arc.len() - 1] != pts[0]);
    }

    #[test]
    fn an_ellipse_is_closed_only_when_full() {
        let c = Vec2::new(0.0, 0.0);
        let (full, closed) = ellipse_points(c, Vec2::new(10.0, 0.0), 0.5, 0.0, 0.0, 0.001);
        assert!(closed && full.len() > 16);
        assert!(full.iter().all(|p| {
            ((p.x / 10.0) * (p.x / 10.0) + (p.y / 5.0) * (p.y / 5.0) - 1.0).abs() < 1e-12
        }));
        let (half, closed) = ellipse_points(c, Vec2::new(10.0, 0.0), 0.5, 0.0, PI, 0.001);
        assert!(!closed);
        assert!((half[0].x - 10.0).abs() < 1e-12 && (half.last().unwrap().x + 10.0).abs() < 1e-9);
    }

    #[test]
    fn the_spline_passes_through_its_points_within_tolerance() {
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 5.0),
            Vec2::new(20.0, -3.0),
            Vec2::new(30.0, 0.0),
        ];
        let open = catmull_rom(&pts, false, 0.001);
        for p in pts {
            assert!(open.contains(&p));
        }
        assert_eq!(open[0], pts[0]);
        assert_eq!(*open.last().unwrap(), pts[3]);
        let closed = catmull_rom(&pts, true, 0.001);
        assert_eq!(closed[0], pts[0]);
        assert!(closed.len() > open.len());
        assert_eq!(catmull_rom(&pts[..2], false, 0.001), pts[..2].to_vec());
    }
}
