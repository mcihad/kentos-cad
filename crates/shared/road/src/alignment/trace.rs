//! An alignment traced from a drawn line (plan/road.md Y10.6, docs/adr/0908 §12; ROAD-IO-14), as Civil 3D's Create
//! Alignment from Objects and Create Best Fit Alignment make one:
//!
//! - **Elements:** a polyline's straight segments are tangents, its arc segments (bulges) circular curves, as they are
//!   (`fit::pis_of`); where two tangents meet without an arc a PI without a curve, or with `radius`'s curve between them
//!   when one is asked for (Civil 3D's “Add curves between tangents”).
//! - **Best fit:** a line of many vertices: a vertex is on a curve where the circle through it and its neighbours is
//!   smaller than `max_radius` (Civil 3D's Maximum Radius For Curve Detection), on a tangent otherwise; each run of
//!   tangent vertices gives a line by least squares (through their centroid, along their principal direction), the
//!   lines' meetings are the PIs, the ends' feet on the first and the last line the alignment's ends; each PI's radius is
//!   the one whose arc, tangent to both lines, lies nearest the curve's vertices (least squares), none where the two runs
//!   share a vertex. No clothoids.
//!
//! The alignment is built from the PIs as any other; how far it lies from the line is the caller's to measure.

use super::chain::PiInput;
use super::fit::{FitElement, FitError, FitKind, pis_of};

/// Why a line makes no PIs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TraceError {
    /// Fewer than two distinct vertices.
    TooFew,
    /// The elements' own reason (`fit::pis_of`).
    Fit(FitError),
    /// Best fit: no run of tangent vertices (every vertex on a curve).
    NoTangent,
    /// Best fit: two runs' lines that do not meet (parallel), by the second run's index (from 0).
    Parallel { run: usize },
}

fn pi(id: String, p: [f64; 2], radius: f64) -> PiInput {
    PiInput { id, x: p[0], y: p[1], radius, spiral_in: 0.0, spiral_out: 0.0, ove: false }
}

/// The ids of `n` PIs: `a`, `p1`, …, `b`.
fn ids(n: usize) -> Vec<String> {
    (0..n).map(|i| if i == 0 { "a".into() } else if i + 1 == n { "b".into() } else { format!("p{i}") }).collect()
}

/// A polyline's segments as elements: a straight one a tangent, one with a bulge `b` its circular arc (`b` the tangent
/// of a quarter of its turn, left positive).
pub fn elements_of(points: &[[f64; 2]], bulges: &[f64]) -> Vec<FitElement> {
    points
        .windows(2)
        .enumerate()
        .map(|(i, w)| {
            let (a, b) = (w[0], w[1]);
            let chord = (b[0] - a[0]).hypot(b[1] - a[1]);
            let bulge = bulges.get(i).copied().unwrap_or(0.0);
            if bulge == 0.0 {
                return FitElement { kind: FitKind::Line, start: a, end: b, pi: None, length: chord };
            }
            let turn = 4.0 * bulge.atan();
            let radius = chord / (2.0 * (turn / 2.0).sin()).abs();
            // The centre: from the chord's middle along its left normal, signed by the turn.
            let (mx, my) = (0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1]));
            let (ux, uy) = ((b[0] - a[0]) / chord, (b[1] - a[1]) / chord);
            let h = radius * (turn / 2.0).cos() * turn.signum();
            let center = [mx - uy * h, my + ux * h];
            FitElement {
                kind: FitKind::Curve { radius, clockwise: bulge < 0.0, center: Some(center) },
                start: a,
                end: b,
                pi: None,
                length: radius * turn.abs(),
            }
        })
        .collect()
}

/// The PIs of a polyline taken element by element; `radius` > 0 puts that curve on every inner PI without one.
pub fn from_elements(points: &[[f64; 2]], bulges: &[f64], radius: f64) -> Result<Vec<PiInput>, TraceError> {
    let points = distinct(points);
    if points.len() < 2 {
        return Err(TraceError::TooFew);
    }
    let mut pis = pis_of(&elements_of(&points, bulges)).map_err(TraceError::Fit)?;
    let n = pis.len();
    if radius > 0.0 {
        for p in pis.iter_mut().take(n - 1).skip(1) {
            if p.radius == 0.0 {
                p.radius = radius;
            }
        }
    }
    Ok(pis)
}

/// The points without one equal to the one before.
fn distinct(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(points.len());
    for p in points {
        if out.last() != Some(p) {
            out.push(*p);
        }
    }
    out
}

/// The radius of the circle through three points; infinite where they are in a line.
pub fn circumradius(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if cross == 0.0 {
        return f64::INFINITY;
    }
    let (ab, bc, ca) =
        ((b[0] - a[0]).hypot(b[1] - a[1]), (c[0] - b[0]).hypot(c[1] - b[1]), (a[0] - c[0]).hypot(a[1] - c[1]));
    ab * bc * ca / (2.0 * cross.abs())
}

/// A line by least squares: its centroid and unit direction (the principal axis, pointing from the first to the last
/// point).
pub fn line_of(points: &[[f64; 2]]) -> ([f64; 2], [f64; 2]) {
    let n = points.len() as f64;
    let (mut cx, mut cy) = (0.0, 0.0);
    for p in points {
        cx += p[0];
        cy += p[1];
    }
    let (cx, cy) = (cx / n, cy / n);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for p in points {
        let (dx, dy) = (p[0] - cx, p[1] - cy);
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    let theta = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let mut d = [theta.cos(), theta.sin()];
    let (first, last) = (points[0], points[points.len() - 1]);
    if d[0] * (last[0] - first[0]) + d[1] * (last[1] - first[1]) < 0.0 {
        d = [-d[0], -d[1]];
    }
    ([cx, cy], d)
}

/// Where two lines (a point and a direction each) meet; none where they are parallel.
fn meet(p: [f64; 2], d: [f64; 2], q: [f64; 2], e: [f64; 2]) -> Option<[f64; 2]> {
    let cross = d[0] * e[1] - d[1] * e[0];
    if cross.abs() < 1e-12 {
        return None;
    }
    let t = ((q[0] - p[0]) * e[1] - (q[1] - p[1]) * e[0]) / cross;
    Some([p[0] + t * d[0], p[1] + t * d[1]])
}

/// The foot of `p` on a line.
fn foot(c: [f64; 2], d: [f64; 2], p: [f64; 2]) -> [f64; 2] {
    let t = (p[0] - c[0]) * d[0] + (p[1] - c[1]) * d[1];
    [c[0] + t * d[0], c[1] + t * d[1]]
}

/// The sum of the squared distances of `points` to the arc of radius `r` tangent to both lines at the PI `v`, the
/// lines' directions in `d` (in) and `e` (out).
fn misfit(v: [f64; 2], d: [f64; 2], e: [f64; 2], r: f64, points: &[[f64; 2]]) -> f64 {
    let turn = (d[0] * e[1] - d[1] * e[0]).atan2(d[0] * e[0] + d[1] * e[1]);
    let half = 0.5 * turn.abs();
    // The bisector inward: the sum of the reversed in direction and the out direction.
    let (bx, by) = (e[0] - d[0], e[1] - d[1]);
    let bl = bx.hypot(by);
    let c = [v[0] + bx / bl * r / half.cos(), v[1] + by / bl * r / half.cos()];
    points.iter().map(|p| ((p[0] - c[0]).hypot(p[1] - c[1]) - r).powi(2)).sum()
}

/// The radius in (0, `most`] whose arc lies nearest `points` (golden section, to a millimetre's millionth).
fn best_radius(v: [f64; 2], d: [f64; 2], e: [f64; 2], points: &[[f64; 2]], most: f64) -> f64 {
    let g = 0.5 * (5f64.sqrt() - 1.0);
    let (mut lo, mut hi) = (most * 1e-9, most);
    let mut x1 = hi - g * (hi - lo);
    let mut x2 = lo + g * (hi - lo);
    let (mut f1, mut f2) = (misfit(v, d, e, x1, points), misfit(v, d, e, x2, points));
    for _ in 0..400 {
        if hi - lo <= 1e-9 {
            break;
        }
        if f1 <= f2 {
            hi = x2;
            x2 = x1;
            f2 = f1;
            x1 = hi - g * (hi - lo);
            f1 = misfit(v, d, e, x1, points);
        } else {
            lo = x1;
            x1 = x2;
            f1 = f2;
            x2 = lo + g * (hi - lo);
            f2 = misfit(v, d, e, x2, points);
        }
    }
    0.5 * (lo + hi)
}

/// The PIs of a line of many vertices by best fit (see the module's comment).
pub fn best_fit(points: &[[f64; 2]], max_radius: f64) -> Result<Vec<PiInput>, TraceError> {
    let p = distinct(points);
    let n = p.len();
    if n < 2 {
        return Err(TraceError::TooFew);
    }
    // Each vertex on a tangent (true) or a curve; the ends on a tangent.
    let straight: Vec<bool> =
        (0..n).map(|i| i == 0 || i + 1 == n || circumradius(p[i - 1], p[i], p[i + 1]) >= max_radius).collect();
    // The tangent runs: two or more tangent vertices in a row.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < n {
        if !straight[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i + 1 < n && straight[i + 1] {
            i += 1;
        }
        if i > start {
            runs.push((start, i));
        }
        i += 1;
    }
    if runs.is_empty() {
        return Err(TraceError::NoTangent);
    }
    let lines: Vec<([f64; 2], [f64; 2])> = runs.iter().map(|&(a, b)| line_of(&p[a..=b])).collect();
    let mut places = vec![foot(lines[0].0, lines[0].1, p[0])];
    let mut radii = vec![0.0];
    for k in 1..runs.len() {
        let ((c0, d0), (c1, d1)) = (lines[k - 1], lines[k]);
        let v = meet(c0, d0, c1, d1).ok_or(TraceError::Parallel { run: k })?;
        let (end_prev, start_next) = (runs[k - 1].1, runs[k].0);
        let between: Vec<[f64; 2]> = p[end_prev + 1..start_next].to_vec();
        let radius = if between.is_empty() {
            0.0
        } else {
            // The arc's tangent points stay on the runs' own lines: its tangent length at most the PI's distance to
            // the runs' far ends.
            let turn = (d0[0] * d1[1] - d0[1] * d1[0]).atan2(d0[0] * d1[0] + d0[1] * d1[1]).abs();
            let reach = |q: [f64; 2]| (q[0] - v[0]).hypot(q[1] - v[1]);
            let room = reach(p[runs[k - 1].0]).min(reach(p[runs[k].1]));
            best_radius(v, d0, d1, &between, room / (0.5 * turn).tan())
        };
        places.push(v);
        radii.push(radius);
    }
    let last = lines[lines.len() - 1];
    places.push(foot(last.0, last.1, p[n - 1]));
    radii.push(0.0);
    let names = ids(places.len());
    Ok(names.into_iter().zip(places).zip(radii).map(|((id, at), r)| pi(id, at, r)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_polyline_with_an_arc_gives_its_pi() {
        // A tangent east, a quarter turn left of radius 10, a tangent north.
        let b = (std::f64::consts::FRAC_PI_8).tan();
        let pis = from_elements(&[[0.0, 0.0], [20.0, 0.0], [30.0, 10.0], [30.0, 30.0]], &[0.0, b, 0.0], 0.0).unwrap();
        assert_eq!(pis.len(), 3);
        assert!((pis[1].x - 30.0).abs() < 1e-9 && pis[1].y.abs() < 1e-9 && (pis[1].radius - 10.0).abs() < 1e-9, "{pis:?}");
    }

    #[test]
    fn best_fit_finds_a_curve_between_two_tangents() {
        // Points along a tangent, an arc of radius 50 turning 90° left, a tangent: every 2 m.
        let mut pts = Vec::new();
        for i in 0..=25 {
            pts.push([i as f64 * 2.0, 0.0]);
        }
        for i in 1..40 {
            let a = std::f64::consts::FRAC_PI_2 * i as f64 / 40.0;
            pts.push([50.0 + 50.0 * a.sin(), 50.0 - 50.0 * a.cos()]);
        }
        for i in 0..=25 {
            pts.push([100.0, 50.0 + i as f64 * 2.0]);
        }
        let pis = best_fit(&pts, 500.0).unwrap();
        assert_eq!(pis.len(), 3, "{pis:?}");
        assert!((pis[1].x - 100.0).abs() < 1e-6 && pis[1].y.abs() < 1e-6, "{pis:?}");
        assert!((pis[1].radius - 50.0).abs() < 1e-3, "{pis:?}");
    }
}
