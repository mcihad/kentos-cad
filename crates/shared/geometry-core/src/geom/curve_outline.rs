//! The chords a computation takes a fit-point curve or an ellipse by
//! (docs/adr/0149 §5.3): snaps, picks and selections, crossings, trim and
//! extend boundaries, Böl and Parçala along it, Patlat, area operations.
//! Every chord keeps within [`CURVE_TOL`] (0.1 mm) of the curve, so a point
//! taken on it is that close to the curve itself; a curve's length is not
//! taken from them (`spline_length`, `ellipse_length`). The drawing's
//! outline is coarser and only drawn.
//!
//! The bound is the chord's own, and each piece is split in two until it
//! holds. A fit-point curve's span is a cubic Bézier, which lies within its
//! control points' hull: when both inner control points are within the
//! tolerance of the chord from the first to the last, so is the whole
//! piece (the distance to a segment is convex), else it is halved (de
//! Casteljau). An ellipse's piece of parameter step h comes no further
//! than a·h²/8 from its chord, a the major semi-axis. A query that only
//! needs the curve near a place (`spline_edges_in`, `ellipse_edges_in`)
//! splits only the pieces whose box reaches it, so a snap splits a few.

use crate::geom::ellipse::{
    EllipseGeom, ellipse_point, ellipse_sweep, is_full_ellipse, major_length,
};
use crate::geom::intersect::Edge;
use crate::geom::spline::catmull_rom_beziers;
use crate::geometry::Bounds;
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::vec2::Vec2;

/// How far a chord may come from the curve it stands for in a computation (m).
pub const CURVE_TOL: f64 = 1e-4;

/// How many times a piece is halved at most (2⁻²⁴ of a span).
const DEPTH: u32 = 24;

/// The distance from p to the segment a–b.
fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (px, py) = (p.x - a.x, p.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        js_min(1.0, js_max(0.0, (px * dx + py * dy) / len2))
    } else {
        0.0
    };
    js_hypot(px - dx * t, py - dy * t)
}

/// A cubic Bézier's halves (de Casteljau at ½).
fn halves(c: &[Vec2; 4]) -> ([Vec2; 4], [Vec2; 4]) {
    let m = |a: Vec2, b: Vec2| Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let (p01, p12, p23) = (m(c[0], c[1]), m(c[1], c[2]), m(c[2], c[3]));
    let (p012, p123) = (m(p01, p12), m(p12, p23));
    let mid = m(p012, p123);
    ([c[0], p01, p012, mid], [mid, p123, p23, c[3]])
}

/// A Bézier's chords within the tolerance, in order, as their ends; only
/// the pieces whose hull reaches `area` when one is given.
fn bezier_chords(c: &[Vec2; 4], area: Option<&Bounds>, depth: u32, out: &mut Vec<(Vec2, Vec2)>) {
    if area.is_some_and(|a| !overlaps(&hull_box(c), a)) {
        return;
    }
    let flat = js_max(to_segment(c[1], c[0], c[3]), to_segment(c[2], c[0], c[3])) <= CURVE_TOL;
    if flat || depth >= DEPTH {
        out.push((c[0], c[3]));
        return;
    }
    let (l, r) = halves(c);
    bezier_chords(&l, area, depth + 1, out);
    bezier_chords(&r, area, depth + 1, out);
}

/// A fit-point curve as a computation takes it. An open curve ends on its
/// last point; a closed one's ring does not repeat its first; fewer than
/// three points are drawn straight, closed or not.
pub fn spline_outline(pts: &[Vec2], closed: bool) -> Vec<Vec2> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut chords = Vec::new();
    for span in catmull_rom_beziers(pts, closed) {
        bezier_chords(&span.ctrl, None, 0, &mut chords);
    }
    let mut out: Vec<Vec2> = chords.iter().map(|c| c.0).collect();
    if !closed {
        out.push(pts[pts.len() - 1]);
    }
    out
}

/// The chords of a fit-point curve that can come into `area`.
pub fn spline_edges_in(pts: &[Vec2], closed: bool, area: &Bounds) -> Vec<Edge> {
    if pts.len() < 3 {
        return pts
            .windows(2)
            .map(|w| Edge::Seg { a: w[0], b: w[1] })
            .collect();
    }
    let mut chords = Vec::new();
    for span in catmull_rom_beziers(pts, closed) {
        bezier_chords(&span.ctrl, Some(area), 0, &mut chords);
    }
    chords
        .into_iter()
        .map(|(a, b)| Edge::Seg { a, b })
        .collect()
}

/// An ellipse piece's chords within the tolerance, in order, as their
/// parameters' ends; only the pieces whose chord box, grown by its bound,
/// reaches `area` when one is given.
fn ellipse_chords(
    g: &EllipseGeom,
    a: f64,
    (t0, p0): (f64, Vec2),
    (t1, p1): (f64, Vec2),
    area: Option<&Bounds>,
    depth: u32,
    out: &mut Vec<(Vec2, Vec2)>,
) {
    let h = t1 - t0;
    let bound = a * h * h / 8.0;
    if let Some(area) = area {
        let b = Bounds {
            min_x: js_min(p0.x, p1.x) - bound,
            min_y: js_min(p0.y, p1.y) - bound,
            max_x: js_max(p0.x, p1.x) + bound,
            max_y: js_max(p0.y, p1.y) + bound,
        };
        if !overlaps(&b, area) {
            return;
        }
    }
    if bound <= CURVE_TOL || depth >= DEPTH {
        out.push((p0, p1));
        return;
    }
    let tm = (t0 + t1) / 2.0;
    let pm = ellipse_point(g, tm);
    ellipse_chords(g, a, (t0, p0), (tm, pm), area, depth + 1, out);
    ellipse_chords(g, a, (tm, pm), (t1, p1), area, depth + 1, out);
}

/// The chords of an ellipse (or its arc), from four first pieces.
fn ellipse_pieces(g: &EllipseGeom, area: Option<&Bounds>) -> Vec<(Vec2, Vec2)> {
    let sweep = ellipse_sweep(g);
    let a = major_length(g);
    let mut out = Vec::new();
    let at = |k: f64| {
        let t = g.t0 + sweep * k / 4.0;
        (t, ellipse_point(g, t))
    };
    for k in 0..4 {
        let k = f64::from(k);
        ellipse_chords(g, a, at(k), at(k + 1.0), area, 0, &mut out);
    }
    out
}

/// An ellipse or its arc as a computation takes it. A whole ellipse's ring
/// does not repeat its first point.
pub fn ellipse_outline(g: &EllipseGeom) -> Vec<Vec2> {
    let chords = ellipse_pieces(g, None);
    let mut out: Vec<Vec2> = chords.iter().map(|c| c.0).collect();
    if !is_full_ellipse(g)
        && let Some(last) = chords.last()
    {
        out.push(last.1);
    }
    out
}

/// The chords of an ellipse (or its arc) that can come into `area`.
pub fn ellipse_edges_in(g: &EllipseGeom, area: &Bounds) -> Vec<Edge> {
    ellipse_pieces(g, Some(area))
        .into_iter()
        .map(|(a, b)| Edge::Seg { a, b })
        .collect()
}

/// The box of a Bézier's control points, which holds the curve.
fn hull_box(c: &[Vec2; 4]) -> Bounds {
    let mut b = Bounds {
        min_x: c[0].x,
        min_y: c[0].y,
        max_x: c[0].x,
        max_y: c[0].y,
    };
    for p in &c[1..] {
        b.min_x = js_min(b.min_x, p.x);
        b.min_y = js_min(b.min_y, p.y);
        b.max_x = js_max(b.max_x, p.x);
        b.max_y = js_max(b.max_y, p.y);
    }
    b
}

/// Whether two boxes overlap, touching included.
fn overlaps(a: &Bounds, b: &Bounds) -> bool {
    a.min_x <= b.max_x && a.max_x >= b.min_x && a.min_y <= b.max_y && a.max_y >= b.min_y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsmath::{js_hypot, js_max};

    /// The distance from p to the segment a–b.
    fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
        };
        js_hypot(p.x - (a.x + dx * t), p.y - (a.y + dy * t))
    }

    /// The farthest the curve's points `samples` come from the polyline
    /// `outline`. Both run along the curve, so each sample is measured
    /// against the chords near the last one's nearest.
    fn farthest(samples: &[Vec2], outline: &[Vec2], closed: bool) -> f64 {
        let n = outline.len();
        let count = if closed { n } else { n - 1 };
        let mut at = 0usize;
        let mut worst = 0.0;
        for &p in samples {
            let mut best = (f64::INFINITY, at);
            let window: Vec<usize> = if closed {
                (0..96).map(|k| (at + count + k - 16) % count).collect()
            } else {
                (at.saturating_sub(16)..(at + 80).min(count)).collect()
            };
            for i in window {
                let d = to_segment(p, outline[i], outline[(i + 1) % n]);
                if d < best.0 {
                    best = (d, i);
                }
            }
            at = best.1;
            worst = js_max(worst, best.0);
        }
        worst
    }

    fn bezier(b: &[Vec2; 4], s: f64) -> Vec2 {
        let r = 1.0 - s;
        let (c0, c1, c2, c3) = (r * r * r, 3.0 * r * r * s, 3.0 * r * s * s, s * s * s);
        Vec2::new(
            b[0].x * c0 + b[1].x * c1 + b[2].x * c2 + b[3].x * c3,
            b[0].y * c0 + b[1].y * c1 + b[2].y * c2 + b[3].y * c3,
        )
    }

    /// Fit-point curves near the origin and at TM coordinates, open and
    /// closed: the curve itself (its Bézier spans, 2000 points a span) never
    /// comes further than the tolerance from the computational outline.
    #[test]
    fn a_curve_keeps_within_the_tolerance_of_its_outline() {
        let shapes: [&[(f64, f64)]; 3] = [
            &[
                (0.0, 0.0),
                (10.0, 8.0),
                (25.0, 5.0),
                (32.0, 20.0),
                (20.0, 30.0),
            ],
            &[(0.0, 0.0), (300.0, 40.0), (310.0, 45.0), (600.0, -200.0)],
            &[(0.0, 0.0), (1.0, 3.0), (2.0, -3.0), (3.0, 3.0), (4.0, 0.0)],
        ];
        for origin in [(0.0, 0.0), (487_000.0, 4_420_000.0)] {
            for pts in shapes {
                let pts: Vec<Vec2> = pts
                    .iter()
                    .map(|&(x, y)| Vec2::new(origin.0 + x, origin.1 + y))
                    .collect();
                for closed in [false, true] {
                    let outline = spline_outline(&pts, closed);
                    let samples: Vec<Vec2> = catmull_rom_beziers(&pts, closed)
                        .iter()
                        .flat_map(|span| {
                            (0..=2000).map(move |i| bezier(&span.ctrl, i as f64 / 2000.0))
                        })
                        .collect();
                    let worst = farthest(&samples, &outline, closed);
                    assert!(worst <= CURVE_TOL, "{origin:?} {closed}: {worst}");
                }
            }
        }
    }

    /// Ellipses from round to flat, whole and arcs, small and large.
    #[test]
    fn an_ellipse_keeps_within_the_tolerance_of_its_outline() {
        for (major, ratio, t0, t1) in [
            (Vec2::new(30.0, 17.5), 1.0, 0.0, 0.0),
            (Vec2::new(30.0, 17.5), 0.5, 0.0, 0.0),
            (Vec2::new(30.0, 17.5), 0.05, 0.0, 0.0),
            (Vec2::new(0.0, 500.0), 0.3, 0.4, 2.0),
            (Vec2::new(2.0, 0.0), 0.2, 5.0, 1.0),
        ] {
            let g = EllipseGeom {
                c: Vec2::new(487_010.0, 4_420_020.0),
                major,
                ratio,
                t0,
                t1,
            };
            let full = crate::geom::ellipse::is_full_ellipse(&g);
            let outline = ellipse_outline(&g);
            let sweep = ellipse_sweep(&g);
            let samples: Vec<Vec2> = (0..=20_000)
                .map(|i| ellipse_point(&g, t0 + sweep * i as f64 / 20_000.0))
                .collect();
            let worst = farthest(&samples, &outline, full);
            assert!(worst <= CURVE_TOL, "{major:?} {ratio}: {worst}");
        }
    }
}
