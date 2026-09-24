//! Non-uniform rational B-splines (DXF SPLINE with control points and
//! knots, hatch spline edges) evaluated with de Boor's algorithm in
//! homogeneous coordinates and sampled until every chord stays within a
//! tolerance of the curve. B-splines are affine-invariant, so callers map
//! the control points first and sample in world units.

use kentos_contracts::Vec2;

use crate::geom::v;

/// The knot span containing `u` (index i with knots[i] ≤ u < knots[i + 1]; the last span for u at the end).
fn span(degree: usize, knots: &[f64], n: usize, u: f64) -> usize {
    if u >= knots[n + 1] {
        // The end of the domain belongs to the last non-empty span.
        let mut i = n;
        while i > degree && knots[i] >= knots[n + 1] {
            i -= 1;
        }
        return i;
    }
    let (mut lo, mut hi) = (degree, n + 1);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if u < knots[mid] { hi = mid } else { lo = mid }
    }
    lo
}

/// The point at parameter `u`.
fn point(degree: usize, knots: &[f64], ctrl: &[Vec2], weights: Option<&[f64]>, u: f64) -> Vec2 {
    let n = ctrl.len() - 1;
    let k = span(degree, knots, n, u);
    let mut d: Vec<[f64; 3]> = (0..=degree)
        .map(|j| {
            let i = k - degree + j;
            let w = weights.map_or(1.0, |w| w[i]);
            [ctrl[i].x * w, ctrl[i].y * w, w]
        })
        .collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = k - degree + j;
            let den = knots[i + degree + 1 - r] - knots[i];
            let a = if den == 0.0 { 0.0 } else { (u - knots[i]) / den };
            let prev = d[j - 1];
            for (c, x) in d[j].iter_mut().enumerate() {
                *x = (1.0 - a) * prev[c] + a * *x;
            }
        }
    }
    let [x, y, w] = d[degree];
    if w == 0.0 { v(x, y) } else { v(x / w, y / w) }
}

fn chord_distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    if l2 == 0.0 {
        return ((p.x - a.x) * (p.x - a.x) + (p.y - a.y) * (p.y - a.y)).sqrt();
    }
    ((p.x - a.x) * dy - (p.y - a.y) * dx).abs() / l2.sqrt()
}

#[allow(clippy::too_many_arguments)]
fn refine(degree: usize, knots: &[f64], ctrl: &[Vec2], w: Option<&[f64]>, tol: f64, u0: f64, p0: Vec2, u1: f64, p1: Vec2, depth: u32, out: &mut Vec<Vec2>) {
    let um = 0.5 * (u0 + u1);
    let pm = point(degree, knots, ctrl, w, um);
    // Quarter points too: a symmetric S-bend can pass through the chord's middle.
    let q1 = point(degree, knots, ctrl, w, 0.5 * (u0 + um));
    let q3 = point(degree, knots, ctrl, w, 0.5 * (um + u1));
    let far = chord_distance(pm, p0, p1).max(chord_distance(q1, p0, p1)).max(chord_distance(q3, p0, p1));
    if depth < 16 && far > tol {
        refine(degree, knots, ctrl, w, tol, u0, p0, um, pm, depth + 1, out);
        refine(degree, knots, ctrl, w, tol, um, pm, u1, p1, depth + 1, out);
    } else {
        out.push(p1);
    }
}

/// The curve as points, every chord within `tol` of it; None when the knot vector does not fit.
pub fn sample(degree: usize, knots: &[f64], ctrl: &[Vec2], weights: Option<&[f64]>, tol: f64) -> Option<Vec<Vec2>> {
    let n = ctrl.len().checked_sub(1)?;
    if degree == 0 || n < degree || knots.len() != ctrl.len() + degree + 1 {
        return None;
    }
    if knots.windows(2).any(|k| !(k[1] >= k[0])) || weights.is_some_and(|w| w.len() != ctrl.len() || w.iter().any(|&x| !(x > 0.0))) {
        return None;
    }
    let (lo, hi) = (knots[degree], knots[n + 1]);
    if !(hi > lo) {
        return None;
    }
    let mut out = vec![point(degree, knots, ctrl, weights, lo)];
    // Every knot span inside the domain is refined on its own (curvature changes at knots).
    let mut breaks: Vec<f64> = knots[degree..=n + 1].to_vec();
    breaks.dedup();
    for pair in breaks.windows(2) {
        let (u0, u1) = (pair[0], pair[1]);
        let p0 = *out.last()?;
        let p1 = point(degree, knots, ctrl, weights, u1);
        refine(degree, knots, ctrl, weights, tol, u0, p0, u1, p1, 0, &mut out);
    }
    out.iter().all(|p| p.x.is_finite() && p.y.is_finite()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quadratic_bezier_and_a_rational_quarter_circle() {
        // Bézier (0,0) (1,2) (2,0): the top point is (1, 1).
        let ctrl = [v(0.0, 0.0), v(1.0, 2.0), v(2.0, 0.0)];
        let knots = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let p = point(2, &knots, &ctrl, None, 0.5);
        assert!((p.x - 1.0).abs() < 1e-15 && (p.y - 1.0).abs() < 1e-15);
        let pts = sample(2, &knots, &ctrl, None, 1e-3).expect("points");
        assert_eq!(pts.first(), Some(&v(0.0, 0.0)));
        assert_eq!(pts.last(), Some(&v(2.0, 0.0)));
        // A quarter circle as a rational quadratic: every sample on the unit circle.
        let w = [1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0];
        let arc = sample(2, &knots, &[v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)], Some(&w), 1e-4).expect("points");
        assert!(arc.len() > 4);
        for q in &arc {
            assert!(((q.x * q.x + q.y * q.y).sqrt() - 1.0).abs() < 1e-12, "{q:?}");
        }
        // Chords stay within the tolerance: midpoints between samples are near the circle.
        for pair in arc.windows(2) {
            let m = v((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0);
            assert!(1.0 - (m.x * m.x + m.y * m.y).sqrt() <= 1e-4);
        }
    }

    #[test]
    fn a_cubic_with_interior_knots_and_bad_vectors() {
        let ctrl = [v(0.0, 0.0), v(1.0, 3.0), v(3.0, 3.0), v(4.0, 0.0), v(6.0, -2.0)];
        let knots = [0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0];
        let pts = sample(3, &knots, &ctrl, None, 1e-3).expect("points");
        assert_eq!(pts.first(), Some(&v(0.0, 0.0)));
        assert_eq!(pts.last(), Some(&v(6.0, -2.0)));
        assert!(sample(3, &knots[..8], &ctrl, None, 1e-3).is_none());
        assert!(sample(3, &[0.0, 0.0, 0.0, 0.0, 1.0, 0.5, 1.0, 1.0, 1.0], &ctrl, None, 1e-3).is_none());
    }
}
