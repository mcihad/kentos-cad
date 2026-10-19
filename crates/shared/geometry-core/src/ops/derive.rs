//! A curve as straight chords with a stated deviation (docs/adr/0530, plan/gis.md CBS-16): what a GIS dataset made
//! from the drawing holds of an analytic arc, circle, bulged edge, ellipse, fit-point curve or NURBS. The chords are
//! a derivative: the curve stays the source, and each run says how far its chords come from it.
//!
//! The algorithm is [`ALGORITHM`] `kentos.chords/1`:
//!
//! - **Circular arcs** (a circle, an arc, a polyline's bulged edge): every chord spans the same angle φ, its ends on
//!   the circle (a bulged edge's ends are the edge's own vertices). A chord of angle φ comes `2·r·sin²(φ/4)` from the
//!   arc, its sagitta, and no nearer or further than that at the chord's middle, so the run's deviation is exact.
//!   The count is the least n whose sagitta is within the tolerance, with no chord over a quarter turn.
//! - **Ellipses and fit-point curves**: the computation outlines of `geom::curve_outline` with their chords within
//!   the tolerance: an ellipse piece of parameter step h within a·h²/8 of its chord, a curve's Bézier span within
//!   the tolerance when both inner control points are. The deviation is that bound, not the measured farthest.
//! - **NURBS**: `Nurbs::tessellate`, whose chords are tested at three points a span: the tolerance is the stated
//!   bound, sampled rather than proved.
//!
//! The same input and tolerance give the same points on every platform (`libm`'s sine and cosine).

use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::geom::curve_outline::{ellipse_outline_tol, spline_outline_tol};
use crate::geom::ellipse::EllipseGeom;
use crate::jsmath::{PI, asin, cos, js_hypot, js_max, sin};
use crate::vec2::Vec2;

/// The derivation's algorithm and its version, as a dataset records them.
pub const ALGORITHM: &str = "kentos.chords/1";
/// The finest tolerance (m) a derivation takes.
pub const MIN_TOLERANCE: f64 = 1e-6;
/// The coarsest tolerance (m).
pub const MAX_TOLERANCE: f64 = 1.0;
/// The most chords one arc is cut into: an arc that would need more is cut into this many and its deviation says so.
pub const MAX_CHORDS: usize = 1 << 16;

/// How a run's deviation is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    /// Worked out from the arc's radius and the chords' angle: the farthest the chords come.
    Exact,
    /// The bound the algorithm keeps (an ellipse's or a fit-point curve's), not the measured farthest.
    Bound,
    /// The tolerance the chords were tested against at sample points (a NURBS).
    Sampled,
}

impl Basis {
    pub fn word(self) -> &'static str {
        match self {
            Basis::Exact => "exact",
            Basis::Bound => "bound",
            Basis::Sampled => "sampled",
        }
    }
}

/// The sagitta of a chord of `step` radians on a circle of radius `r`: how far the chord's middle is from the arc.
pub fn sagitta(r: f64, step: f64) -> f64 {
    let s = sin(step.abs() / 4.0);
    2.0 * r * s * s
}

/// The least number of equal chords an arc of radius `r` and `sweep` radians is cut into so that none is further
/// than `tol` from it; none spans over a quarter turn, and at most [`MAX_CHORDS`].
pub fn arc_chord_count(r: f64, sweep: f64, tol: f64) -> usize {
    let theta = sweep.abs();
    if !(r > 0.0 && theta > 0.0 && tol > 0.0 && r.is_finite() && theta.is_finite()) {
        return 1;
    }
    let ratio = tol / (2.0 * r);
    let quarter = PI / 2.0;
    let step = if ratio >= 0.5 {
        quarter
    } else {
        let s = 4.0 * asin(ratio.sqrt());
        if s < quarter { s } else { quarter }
    };
    let mut n = (theta / step).ceil();
    if !(n >= 1.0) {
        n = 1.0;
    }
    let mut n = if n > MAX_CHORDS as f64 {
        MAX_CHORDS
    } else {
        n as usize
    };
    // The closed form can be one short at the boundary: step up until the sagitta holds.
    while n < MAX_CHORDS && sagitta(r, theta / n as f64) > tol {
        n += 1;
    }
    n
}

/// A run of chords: the points (both ends included) and how far the chords come from the curve at most (m).
#[derive(Clone, Debug, PartialEq)]
pub struct Chords {
    pub pts: Vec<Vec2>,
    pub deviation: f64,
}

/// The arc of radius `r` about `c` from angle `a0` through `sweep` radians (signed: positive counter-clockwise) as
/// chords of one angle, every point on the circle.
pub fn arc_chords(c: Vec2, r: f64, a0: f64, sweep: f64, tol: f64) -> Chords {
    let n = arc_chord_count(r, sweep, tol);
    let pts = (0..=n)
        .map(|k| {
            let t = a0 + sweep * (k as f64) / (n as f64);
            Vec2::new(c.x + r * cos(t), c.y + r * sin(t))
        })
        .collect();
    Chords {
        pts,
        deviation: sagitta(r, sweep / n as f64),
    }
}

/// A path as chords: its vertices and the chords' points, and where each lies.
#[derive(Clone, Debug, PartialEq)]
pub struct PathChords {
    /// The points in order. A closed path's ring does not repeat its first point.
    pub pts: Vec<Vec2>,
    /// Each point's place along the path: the edge it starts (the index of the path's vertex it follows) and how
    /// far along that edge, 0 for the path's own vertices, k/n for the k-th of n chords of an arc. A value
    /// (elevation, measure) between two vertices is taken at this fraction.
    pub at: Vec<(usize, f64)>,
    /// The farthest any chord comes from its arc (m); 0 for a path of straight edges.
    pub deviation: f64,
}

/// A path with bulges as chords: each bulged edge cut by [`arc_chord_count`], straight edges kept; the vertices
/// themselves are kept as they are.
pub fn path_chords(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool, tol: f64) -> PathChords {
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    let mut out = Vec::with_capacity(n);
    let mut at = Vec::with_capacity(n);
    let mut deviation = 0.0;
    for i in 0..edges {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        out.push(a);
        at.push((i, 0.0));
        let Some(arc) = bulge_arc(a, b, bulge_at(bulges, i)) else {
            continue;
        };
        let count = arc_chord_count(arc.r, arc.sweep, tol);
        let step = arc.sweep / count as f64;
        deviation = js_max(deviation, sagitta(arc.r, step));
        for k in 1..count {
            let t = arc.a0 + arc.sweep * (k as f64) / (count as f64);
            out.push(Vec2::new(
                arc.c.x + arc.r * cos(t),
                arc.c.y + arc.r * sin(t),
            ));
            at.push((i, k as f64 / count as f64));
        }
    }
    if !closed && n > 0 {
        out.push(pts[n - 1]);
        at.push((n - 1, 0.0));
    }
    PathChords {
        pts: out,
        at,
        deviation,
    }
}

/// An ellipse (or its arc) as chords within `tol`: a whole ellipse's ring does not repeat its first point.
pub fn ellipse_chords(g: &EllipseGeom, tol: f64) -> Vec<Vec2> {
    ellipse_outline_tol(g, tol)
}

/// A fit-point curve as chords within `tol`: a closed curve's ring does not repeat its first point.
pub fn spline_chords(pts: &[Vec2], closed: bool, tol: f64) -> Vec<Vec2> {
    spline_outline_tol(pts, closed, tol)
}

/// The distance from `p` to the segment `a`–`b`.
pub fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    js_hypot(p.x - (a.x + dx * t), p.y - (a.y + dy * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_circle_is_cut_until_its_sagitta_holds() {
        for (r, tol) in [(1.0, 1e-3), (50.0, 1e-4), (0.01, 1e-6), (1000.0, 1e-3)] {
            let run = arc_chords(Vec2::new(10.0, -3.0), r, 0.3, std::f64::consts::TAU, tol);
            assert!(run.deviation <= tol, "{r} {tol}: {}", run.deviation);
            let n = run.pts.len() - 1;
            // One chord fewer would not hold.
            assert!(
                sagitta(r, std::f64::consts::TAU / (n - 1) as f64) > tol || n <= 4,
                "{r} {tol}"
            );
            // Every point is on the circle.
            for p in &run.pts {
                let d = js_hypot(p.x - 10.0, p.y + 3.0);
                assert!((d - r).abs() <= 1e-12 * js_max(r, 1.0), "{d} {r}");
            }
        }
    }

    #[test]
    fn no_chord_spans_over_a_quarter_turn() {
        // A tolerance as large as the radius still cuts a circle into a square at the least.
        assert_eq!(arc_chord_count(1.0, std::f64::consts::TAU, 1.0), 4);
        assert_eq!(arc_chord_count(0.5, std::f64::consts::PI, 1.0), 2);
    }

    #[test]
    fn a_bulged_edge_keeps_its_ends_and_straight_edges_stay() {
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
        ];
        let run = path_chords(&pts, Some(&[1.0, 0.0]), false, 1e-3);
        assert_eq!(run.pts.first(), Some(&pts[0]));
        assert_eq!(run.pts.last(), Some(&pts[2]));
        assert!(run.pts.contains(&pts[1]));
        // A bulge of 1 is a half circle of radius 5: its chords' angle is the same, its deviation within 1 mm.
        assert!(run.deviation > 0.0 && run.deviation <= 1e-3);
        assert_eq!(run.at.len(), run.pts.len());
        assert!(run.at.iter().any(|(i, f)| *i == 0 && *f > 0.0 && *f < 1.0));
        assert!(
            run.at
                .iter()
                .filter(|(i, _)| *i == 1)
                .all(|(_, f)| *f == 0.0)
        );
    }

    #[test]
    fn a_closed_ring_does_not_repeat_its_first_point() {
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 4.0),
        ];
        let run = path_chords(&pts, None, true, 1e-3);
        assert_eq!(run.pts, pts.to_vec());
        assert_eq!(run.deviation, 0.0);
    }
}
