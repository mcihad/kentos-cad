//! A NURBS curve built from what a user gives (docs/adr/0259 §3): by its
//! control points (the knots clamped and uniform, or a closed curve's
//! control points wrapped over uniform knots) or through points it passes
//! (Piegl and Tiller's global interpolation, The NURBS Book §9.2.1: chord-
//! length parameters, averaged knots, the collocation system solved; a
//! closed curve periodic). `cad.nurbs.create` and the curve tools build it
//! here.
//!
//! Every step is plain IEEE arithmetic in a fixed order (no `hypot`, no
//! fused operations): scripts/fixtures/nurbs_cases.py repeats it in Python
//! to the bit and checks the result against an exact solution and
//! OpenCASCADE.

use crate::geom::nurbs::OwnedNurbs;
use crate::vec2::Vec2;

/// Why the points make no curve.
#[derive(Clone, Debug, PartialEq)]
pub enum BuildError {
    /// Points `i` and `i + 1` (the first, after the last of a closed curve) at one place.
    Coincident(usize),
    /// The numbers make no curve; said in Turkish.
    Invalid(String),
}

/// The most fit points a closed curve is built through (its system is solved whole).
pub const MAX_CLOSED_FIT: usize = 1000;
/// The highest degree.
pub const MAX_DEGREE: usize = 11;

/// The span `t` lies in (The NURBS Book A2.1): `last` the last control point's index.
pub fn find_span(last: usize, p: usize, t: f64, knots: &[f64]) -> usize {
    if t >= knots[last + 1] {
        return last;
    }
    let (mut low, mut high) = (p, last + 1);
    let mut mid = (low + high) / 2;
    while t < knots[mid] || t >= knots[mid + 1] {
        if t < knots[mid] {
            high = mid;
        } else {
            low = mid;
        }
        mid = (low + high) / 2;
    }
    mid
}

/// The `p + 1` basis functions not zero at `t` in `span` (The NURBS Book A2.2).
pub fn basis(span: usize, t: f64, p: usize, knots: &[f64]) -> Vec<f64> {
    let mut n = vec![0.0; p + 1];
    let mut left = vec![0.0; p + 1];
    let mut right = vec![0.0; p + 1];
    n[0] = 1.0;
    for j in 1..=p {
        left[j] = t - knots[span + 1 - j];
        right[j] = knots[span + j] - t;
        let mut saved = 0.0;
        for r in 0..j {
            let temp = n[r] / (right[r + 1] + left[j - r]);
            n[r] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        n[j] = saved;
    }
    n
}

/// A curve by its control points: an open one's knots as given or clamped
/// and uniform; a closed one's first `degree` control points (and weights)
/// wrapped after the last, over the uniform knots 0, 1, … (periodic).
pub fn by_control(
    degree: usize,
    ctrl: &[Vec2],
    weights: Option<&[f64]>,
    knots: Option<&[f64]>,
    closed: bool,
) -> Result<OwnedNurbs, BuildError> {
    let p = degree;
    if !(1..=MAX_DEGREE).contains(&p) {
        return Err(BuildError::Invalid(format!(
            "Derece {p}; 1 ile {MAX_DEGREE} arasında olmalı."
        )));
    }
    let n = ctrl.len();
    if let Some(w) = weights
        && w.len() != n
    {
        return Err(BuildError::Invalid(format!(
            "Ağırlık sayısı ({}) denetim noktası sayısından ({n}) farklı.",
            w.len()
        )));
    }
    if closed {
        if knots.is_some() {
            return Err(BuildError::Invalid(
                "Kapalı eğrinin düğümlerini komut kurar; düğüm vermeyin.".into(),
            ));
        }
        if n < 3 || n < p {
            return Err(BuildError::Invalid(format!(
                "{p}. dereceden kapalı eğrinin en az {} denetim noktası olur; {n} verildi.",
                p.max(3)
            )));
        }
        let wrap = |i: usize| ctrl[i % n];
        let out = OwnedNurbs {
            degree: p,
            ctrl: (0..n + p).map(wrap).collect(),
            knots: (0..=n + 2 * p).map(|i| i as f64).collect(),
            weights: weights.map(|w| (0..n + p).map(|i| w[i % n]).collect()),
        };
        return Ok(out);
    }
    if n < p + 1 {
        return Err(BuildError::Invalid(format!(
            "{p}. dereceden eğrinin en az {} denetim noktası olur; {n} verildi.",
            p + 1
        )));
    }
    let knots = match knots {
        Some(k) => k.to_vec(),
        None => clamped(p, n),
    };
    Ok(OwnedNurbs {
        degree: p,
        ctrl: ctrl.to_vec(),
        knots,
        weights: weights.map(<[f64]>::to_vec),
    })
}

/// 0 (p + 1 times), 1, …, n − p − 1, n − p (p + 1 times).
fn clamped(p: usize, n: usize) -> Vec<f64> {
    let spans = n - p;
    let mut k = vec![0.0; p + 1];
    k.extend((1..spans).map(|i| i as f64));
    k.extend(std::iter::repeat_n(spans as f64, p + 1));
    k
}

/// The distance between two points: `sqrt(dx·dx + dy·dy)`, rounded at each step.
fn chord(a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    (dx * dx + dy * dy).sqrt()
}

/// The fit points a closed curve passes: a last point at the first's place is the first.
pub fn closed_fit(pts: &[Vec2]) -> &[Vec2] {
    match pts {
        [first, .., last] if pts.len() > 1 && first == last => &pts[..pts.len() - 1],
        _ => pts,
    }
}

/// The degree a curve through `n` points gets: the one asked for, at most `n − 1`.
pub fn fit_degree(asked: usize, n: usize) -> usize {
    asked.min(n.saturating_sub(1)).max(1)
}

/// A curve passing through `pts` in order, of `degree` (lowered to one less
/// than the points when they are fewer).
pub fn through(pts: &[Vec2], degree: usize, closed: bool) -> Result<OwnedNurbs, BuildError> {
    let pts = if closed { closed_fit(pts) } else { pts };
    let n = pts.len();
    let edges = match n {
        0 | 1 => 0,
        _ if closed => n,
        _ => n - 1,
    };
    if let Some(i) = (0..edges).find(|&i| pts[i] == pts[(i + 1) % n]) {
        return Err(BuildError::Coincident(i));
    }
    if !(1..=MAX_DEGREE).contains(&degree) {
        return Err(BuildError::Invalid(format!(
            "Derece {degree}; 1 ile {MAX_DEGREE} arasında olmalı."
        )));
    }
    let least = if closed { 3 } else { 2 };
    if n < least {
        return Err(BuildError::Invalid(format!(
            "{} eğri en az {least} geçiş noktasından geçer; {n} verildi.",
            if closed { "Kapalı" } else { "Açık" }
        )));
    }
    if closed && n > MAX_CLOSED_FIT {
        return Err(BuildError::Invalid(format!(
            "Kapalı eğri en çok {MAX_CLOSED_FIT} geçiş noktasından kurulur; {n} verildi."
        )));
    }
    if n > MAX_POINTS {
        return Err(BuildError::Invalid(format!(
            "Geçiş noktası en çok {MAX_POINTS}; {n} verildi."
        )));
    }
    let p = fit_degree(degree, n);
    Ok(if closed {
        through_closed(pts, p)
    } else {
        through_open(pts, p)
    })
}

/// The most points a curve has.
pub const MAX_POINTS: usize = 100_000;

fn through_open(q: &[Vec2], p: usize) -> OwnedNurbs {
    let n = q.len();
    // Chord-length parameters (9.5): the running length over the whole.
    let d: Vec<f64> = (1..n).map(|k| chord(q[k - 1], q[k])).collect();
    let mut total = 0.0;
    for x in &d {
        total += *x;
    }
    let mut t = vec![0.0; n];
    for k in 1..n - 1 {
        t[k] = t[k - 1] + d[k - 1] / total;
    }
    t[n - 1] = 1.0;
    // Averaged knots (9.8).
    let mut knots = vec![0.0; p + 1];
    for j in 1..n - p {
        let mut s = 0.0;
        for x in &t[j..j + p] {
            s += *x;
        }
        knots.push(s / p as f64);
    }
    knots.extend(std::iter::repeat_n(1.0, p + 1));
    // The collocation system, banded: row k's columns k − p … k + p.
    let w = 2 * p + 1;
    let mut a = vec![0.0; n * w];
    let at = |r: usize, c: usize| r * w + (c + p - r);
    for k in 0..n {
        let span = find_span(n - 1, p, t[k], &knots);
        let nb = basis(span, t[k], p, &knots);
        for (r, v) in nb.iter().enumerate() {
            a[at(k, span - p + r)] = *v;
        }
    }
    let mut bx: Vec<f64> = q.iter().map(|v| v.x).collect();
    let mut by: Vec<f64> = q.iter().map(|v| v.y).collect();
    // Elimination without pivoting: a B-spline collocation matrix is totally positive.
    for k in 0..n {
        let piv = a[at(k, k)];
        for i in k + 1..(k + p + 1).min(n) {
            let f = a[at(i, k)] / piv;
            for j in k..(k + p + 1).min(n) {
                let v = a[at(k, j)];
                a[at(i, j)] -= f * v;
            }
            bx[i] -= f * bx[k];
            by[i] -= f * by[k];
        }
    }
    let mut x = vec![0.0; n];
    let mut y = vec![0.0; n];
    for i in (0..n).rev() {
        let (mut sx, mut sy) = (bx[i], by[i]);
        for j in i + 1..(i + p + 1).min(n) {
            sx -= a[at(i, j)] * x[j];
            sy -= a[at(i, j)] * y[j];
        }
        x[i] = sx / a[at(i, i)];
        y[i] = sy / a[at(i, i)];
    }
    OwnedNurbs {
        degree: p,
        ctrl: (0..n).map(|i| Vec2::new(x[i], y[i])).collect(),
        knots,
        weights: None,
    }
}

/// A curve through `pts` in order with its start and end tangent directions where given (docs/adr/0303 §3;
/// Piegl and Tiller 9.2.2): an open curve only. Each given end adds a control point and an equation, the end's
/// derivative its unit direction times the chord length of the whole; the knots are averaged over the
/// parameters with each constrained end's repeated, and the degree lowered to one less than the control points.
/// No tangent: [`through`].
pub fn through_tangents(
    pts: &[Vec2],
    degree: usize,
    start: Option<Vec2>,
    end: Option<Vec2>,
) -> Result<OwnedNurbs, BuildError> {
    if start.is_none() && end.is_none() {
        return through(pts, degree, false);
    }
    // The checks of an open curve through points first.
    through(pts, degree, false)?;
    let unit = |v: Vec2| {
        let l = chord(Vec2::new(0.0, 0.0), v);
        (l > 0.0 && l.is_finite()).then(|| Vec2::new(v.x / l, v.y / l))
    };
    let (s, e) = match (start.map(unit), end.map(unit)) {
        (Some(None), _) | (_, Some(None)) => {
            return Err(BuildError::Invalid(
                "Teğetin doğrultusu yok (sıfır uzunlukta); bir doğrultu verin ya da teğeti kaldırın.".into(),
            ));
        }
        (s, e) => (s.flatten(), e.flatten()),
    };
    Ok(through_open_tangents(pts, degree, s, e))
}

fn through_open_tangents(
    q: &[Vec2],
    degree: usize,
    s: Option<Vec2>,
    e: Option<Vec2>,
) -> OwnedNurbs {
    let n = q.len();
    let d: Vec<f64> = (1..n).map(|k| chord(q[k - 1], q[k])).collect();
    let mut total = 0.0;
    for x in &d {
        total += *x;
    }
    let mut t = vec![0.0; n];
    for k in 1..n - 1 {
        t[k] = t[k - 1] + d[k - 1] / total;
    }
    t[n - 1] = 1.0;
    // The parameters with each constrained end's repeated: one per control point.
    let mut ext = Vec::with_capacity(n + 2);
    ext.push(t[0]);
    if s.is_some() {
        ext.push(t[0]);
    }
    ext.extend_from_slice(&t[1..n - 1]);
    if e.is_some() {
        ext.push(t[n - 1]);
    }
    ext.push(t[n - 1]);
    let m = ext.len();
    let p = fit_degree(degree, m);
    let mut knots = vec![0.0; p + 1];
    for j in 1..m - p {
        let mut sum = 0.0;
        for x in &ext[j..j + p] {
            sum += *x;
        }
        knots.push(sum / p as f64);
    }
    knots.extend(std::iter::repeat_n(1.0, p + 1));
    // The rows: the first point, the start's derivative, the inner points, the end's derivative, the last point.
    let mut a = vec![0.0; m * m];
    let mut bx = Vec::with_capacity(m);
    let mut by = Vec::with_capacity(m);
    let point_row = |a: &mut [f64], row: usize, k: usize| {
        let span = find_span(m - 1, p, t[k], &knots);
        let nb = basis(span, t[k], p, &knots);
        for (r, v) in nb.iter().enumerate() {
            a[row * m + span - p + r] = *v;
        }
    };
    let mut row = 0;
    point_row(&mut a, row, 0);
    bx.push(q[0].x);
    by.push(q[0].y);
    row += 1;
    if let Some(dir) = s {
        let f = p as f64 / (knots[p + 1] - knots[1]);
        a[row * m] = -f;
        a[row * m + 1] = f;
        bx.push(dir.x * total);
        by.push(dir.y * total);
        row += 1;
    }
    for k in 1..n - 1 {
        point_row(&mut a, row, k);
        bx.push(q[k].x);
        by.push(q[k].y);
        row += 1;
    }
    if let Some(dir) = e {
        let f = p as f64 / (knots[m - 1 + p] - knots[m - 1]);
        a[row * m + m - 2] = -f;
        a[row * m + m - 1] = f;
        bx.push(dir.x * total);
        by.push(dir.y * total);
        row += 1;
    }
    point_row(&mut a, row, n - 1);
    bx.push(q[n - 1].x);
    by.push(q[n - 1].y);
    // Gaussian elimination with partial pivoting (the first largest).
    for k in 0..m {
        let mut best = k;
        for i in k + 1..m {
            if a[i * m + k].abs() > a[best * m + k].abs() {
                best = i;
            }
        }
        if best != k {
            for j in 0..m {
                a.swap(k * m + j, best * m + j);
            }
            bx.swap(k, best);
            by.swap(k, best);
        }
        let piv = a[k * m + k];
        for i in k + 1..m {
            let f = a[i * m + k] / piv;
            for j in k..m {
                let v = a[k * m + j];
                a[i * m + j] -= f * v;
            }
            bx[i] -= f * bx[k];
            by[i] -= f * by[k];
        }
    }
    let mut x = vec![0.0; m];
    let mut y = vec![0.0; m];
    for i in (0..m).rev() {
        let (mut sx, mut sy) = (bx[i], by[i]);
        for j in i + 1..m {
            sx -= a[i * m + j] * x[j];
            sy -= a[i * m + j] * y[j];
        }
        x[i] = sx / a[i * m + i];
        y[i] = sy / a[i * m + i];
    }
    OwnedNurbs {
        degree: p,
        ctrl: (0..m).map(|i| Vec2::new(x[i], y[i])).collect(),
        knots,
        weights: None,
    }
}

fn through_closed(q: &[Vec2], p: usize) -> OwnedNurbs {
    let n = q.len();
    let d: Vec<f64> = (0..n).map(|k| chord(q[k], q[(k + 1) % n])).collect();
    let mut total = 0.0;
    for x in &d {
        total += *x;
    }
    let mut t = vec![0.0; n + 1];
    for k in 1..n {
        t[k] = t[k - 1] + d[k - 1] / total;
    }
    t[n] = 1.0;
    // The knots repeat with period 1: the parameters (odd degree) or their midpoints (even).
    let base: Vec<f64> = (0..n)
        .map(|i| {
            if p % 2 == 1 {
                t[i]
            } else {
                (t[i] + t[i + 1]) / 2.0
            }
        })
        .collect();
    let knot = |m: isize| {
        let q = m.div_euclid(n as isize);
        let r = m.rem_euclid(n as isize) as usize;
        base[r] + q as f64
    };
    let knots: Vec<f64> = (0..=n + 2 * p)
        .map(|j| knot(j as isize - p as isize))
        .collect();
    let mut a = vec![0.0; n * n];
    for k in 0..n {
        let tk = if t[k] >= base[0] { t[k] } else { t[k] + 1.0 };
        let span = find_span(n + p - 1, p, tk, &knots);
        let nb = basis(span, tk, p, &knots);
        for (r, v) in nb.iter().enumerate() {
            a[k * n + (span - p + r) % n] += *v;
        }
    }
    let mut bx: Vec<f64> = q.iter().map(|v| v.x).collect();
    let mut by: Vec<f64> = q.iter().map(|v| v.y).collect();
    // Gaussian elimination with partial pivoting (the first largest).
    for k in 0..n {
        let mut best = k;
        for i in k + 1..n {
            if a[i * n + k].abs() > a[best * n + k].abs() {
                best = i;
            }
        }
        if best != k {
            for j in 0..n {
                a.swap(k * n + j, best * n + j);
            }
            bx.swap(k, best);
            by.swap(k, best);
        }
        let piv = a[k * n + k];
        for i in k + 1..n {
            let f = a[i * n + k] / piv;
            for j in k..n {
                let v = a[k * n + j];
                a[i * n + j] -= f * v;
            }
            bx[i] -= f * bx[k];
            by[i] -= f * by[k];
        }
    }
    let mut x = vec![0.0; n];
    let mut y = vec![0.0; n];
    for i in (0..n).rev() {
        let (mut sx, mut sy) = (bx[i], by[i]);
        for j in i + 1..n {
            sx -= a[i * n + j] * x[j];
            sy -= a[i * n + j] * y[j];
        }
        x[i] = sx / a[i * n + i];
        y[i] = sy / a[i * n + i];
    }
    OwnedNurbs {
        degree: p,
        ctrl: (0..n + p).map(|i| Vec2::new(x[i % n], y[i % n])).collect(),
        knots,
        weights: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// docs/adr/0303 §3: the curve passes every point and leaves and arrives along the tangents given.
    #[test]
    fn a_curve_through_points_takes_its_end_tangents() {
        let pts = [v(0.0, 0.0), v(10.0, 5.0), v(20.0, 0.0), v(30.0, 4.0)];
        for (s, e) in [
            (Some(v(0.0, 1.0)), None),
            (None, Some(v(1.0, -1.0))),
            (Some(v(1.0, 0.0)), Some(v(0.0, -2.0))),
        ] {
            let c = through_tangents(&pts, 3, s, e).expect("a curve");
            assert_eq!(
                c.ctrl.len(),
                4 + usize::from(s.is_some()) + usize::from(e.is_some())
            );
            passes(&c, &pts);
            let view = c.view();
            let (u0, u1) = view.domain();
            let dir = |d: Vec2| {
                let l = (d.x * d.x + d.y * d.y).sqrt();
                (d.x / l, d.y / l)
            };
            if let Some(want) = s {
                let got = dir(view.derivs(u0, 1)[1]);
                let w = dir(want);
                assert!(
                    (got.0 - w.0).abs() < 1e-9 && (got.1 - w.1).abs() < 1e-9,
                    "{got:?}"
                );
            }
            if let Some(want) = e {
                let got = dir(view.derivs(u1, 1)[1]);
                let w = dir(want);
                assert!(
                    (got.0 - w.0).abs() < 1e-9 && (got.1 - w.1).abs() < 1e-9,
                    "{got:?}"
                );
            }
        }
        assert!(matches!(
            through_tangents(&pts, 3, Some(v(0.0, 0.0)), None),
            Err(BuildError::Invalid(_))
        ));
    }

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn passes(c: &OwnedNurbs, pts: &[Vec2]) {
        let view = c.view();
        for q in pts {
            let on = view.point(view.closest(*q));
            let d = ((on.x - q.x) * (on.x - q.x) + (on.y - q.y) * (on.y - q.y)).sqrt();
            assert!(d < 1e-9, "{q:?} {on:?} {d}");
        }
    }

    #[test]
    fn an_open_curve_passes_through_its_points() {
        let pts = [
            v(0.0, 0.0),
            v(3.0, 4.0),
            v(7.0, 2.0),
            v(10.0, 6.0),
            v(14.0, 1.0),
        ];
        let c = through(&pts, 3, false).expect("a curve");
        assert!(c.view().valid());
        assert_eq!(c.ctrl.len(), 5);
        assert_eq!(c.ctrl[0], pts[0]);
        assert_eq!(c.ctrl[4], pts[4]);
        passes(&c, &pts);
        // Two points: a line of degree 1.
        let line = through(&pts[..2], 3, false).unwrap();
        assert_eq!(line.degree, 1);
    }

    #[test]
    fn a_closed_curve_passes_and_closes_smoothly() {
        let pts = [
            v(0.0, 0.0),
            v(10.0, 0.0),
            v(12.0, 7.0),
            v(4.0, 9.0),
            v(-2.0, 5.0),
        ];
        for p in [2, 3, 4] {
            let c = through(&pts, p, true).expect("a curve");
            let view = c.view();
            assert!(view.valid());
            passes(&c, &pts);
            let (lo, hi) = view.domain();
            let (a, b) = (view.derivs(lo, 1), view.derivs(hi, 1));
            assert!(
                (a[0].x - b[0].x).abs() < 1e-9 && (a[1].y - b[1].y).abs() < 1e-7,
                "{p}"
            );
        }
        // The last point at the first's place is the first.
        let mut again = pts.to_vec();
        again.push(pts[0]);
        assert_eq!(through(&again, 3, true), through(&pts, 3, true));
    }

    #[test]
    fn far_from_the_origin_a_curve_measures_as_near_it() {
        // Map coordinates (487 000, 4 420 000): the length's quadrature settles as it does at the origin.
        let near = [
            v(0.0, 0.0),
            v(12.0, 7.5),
            v(25.0, 3.0),
            v(38.0, 11.0),
            v(52.0, 6.25),
        ];
        let far: Vec<Vec2> = near
            .iter()
            .map(|p| v(p.x + 487_000.0, p.y + 4_420_000.0))
            .collect();
        let a = through(&near, 3, false).unwrap().view().length();
        let b = through(&far, 3, false).unwrap().view().length();
        assert!((a - b).abs() < 1e-9, "{a} {b}");
    }

    #[test]
    fn what_makes_no_curve_is_refused() {
        let pts = [v(0.0, 0.0), v(1.0, 0.0), v(1.0, 0.0)];
        assert_eq!(through(&pts, 3, false), Err(BuildError::Coincident(1)));
        assert!(matches!(
            through(&pts[..2], 3, true),
            Err(BuildError::Invalid(_))
        ));
        assert!(matches!(
            by_control(3, &pts[..2], None, None, false),
            Err(BuildError::Invalid(_))
        ));
        let ring = by_control(
            3,
            &[v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0), v(0.0, 4.0)],
            None,
            None,
            true,
        )
        .unwrap();
        assert_eq!(ring.ctrl.len(), 7);
        assert_eq!(ring.knots.len(), 11);
    }
}
