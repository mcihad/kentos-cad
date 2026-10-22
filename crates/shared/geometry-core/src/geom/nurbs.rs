//! A general NURBS curve (docs/adr/0259): evaluated with de Boor's algorithm
//! in homogeneous coordinates, its derivatives from the basis functions'
//! (Piegl and Tiller, The NURBS Book, A2.3 and A4.2), split exactly by knot
//! insertion (A5.1), measured by adaptive Gauss–Legendre quadrature of its
//! speed and searched for the nearest point by Newton's method from a sampled
//! start. The source numbers are never rounded: a split curve is the same
//! curve, its control points the inserted ones. Drawing, picking and the
//! area of a closed curve use `tessellate`'s points, every chord within the
//! tolerance; measures, snaps and cuts use the curve itself.

use crate::geom::quadrature::integrate;
use crate::jsmath::{js_max, js_min};
use crate::vec2::Vec2;

const fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

/// The drawing's tolerance for a curve as points, metres (as `curve_outline`'s).
pub const TOLERANCE: f64 = 1e-4;
/// How deep a span is halved at most when tessellated.
const DEPTH: u32 = 18;

/// A NURBS curve's numbers.
#[derive(Clone, Debug, PartialEq)]
pub struct Nurbs<'a> {
    pub degree: usize,
    pub ctrl: &'a [Vec2],
    pub knots: &'a [f64],
    pub weights: Option<&'a [f64]>,
}

/// A curve owned: what a split gives.
#[derive(Clone, Debug, PartialEq)]
pub struct OwnedNurbs {
    pub degree: usize,
    pub ctrl: Vec<Vec2>,
    pub knots: Vec<f64>,
    pub weights: Option<Vec<f64>>,
}

impl OwnedNurbs {
    pub fn view(&self) -> Nurbs<'_> {
        Nurbs {
            degree: self.degree,
            ctrl: &self.ctrl,
            knots: &self.knots,
            weights: self.weights.as_deref(),
        }
    }
}

impl<'a> Nurbs<'a> {
    /// Whether the numbers make a curve (the contract's rule, `nurbs_problem`, in short).
    pub fn valid(&self) -> bool {
        let (p, n) = (self.degree, self.ctrl.len());
        p >= 1
            && n > p
            && self.knots.len() == n + p + 1
            && self.knots.windows(2).all(|w| w[1] >= w[0])
            && self.knots.iter().all(|k| k.is_finite())
            && self.knots[n] > self.knots[p]
            && self
                .weights
                .is_none_or(|w| w.len() == n && w.iter().all(|x| *x > 0.0 && x.is_finite()))
    }

    /// The parameter interval the curve runs over.
    pub fn domain(&self) -> (f64, f64) {
        (self.knots[self.degree], self.knots[self.ctrl.len()])
    }

    fn w(&self, i: usize) -> f64 {
        self.weights.map_or(1.0, |w| w[i])
    }

    /// The knot span holding `u` (A2.1): knots[i] ≤ u < knots[i + 1], the last span for the domain's end.
    fn span(&self, u: f64) -> usize {
        let (p, n) = (self.degree, self.ctrl.len() - 1);
        let k = self.knots;
        if u >= k[n + 1] {
            let mut i = n;
            while i > p && k[i] >= k[n + 1] {
                i -= 1;
            }
            return i;
        }
        if u <= k[p] {
            let mut i = p;
            while i < n && k[i + 1] <= k[p] {
                i += 1;
            }
            return i;
        }
        let (mut lo, mut hi) = (p, n + 1);
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if u < k[mid] {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        lo
    }

    /// The basis functions and their derivatives to `nd` at `u` in span `i` (A2.3): `ders[k][j]`.
    fn basis_ders(&self, i: usize, u: f64, nd: usize) -> Vec<Vec<f64>> {
        let p = self.degree;
        let k = self.knots;
        let mut ndu = vec![vec![0.0; p + 1]; p + 1];
        let mut left = vec![0.0; p + 1];
        let mut right = vec![0.0; p + 1];
        ndu[0][0] = 1.0;
        for j in 1..=p {
            left[j] = u - k[i + 1 - j];
            right[j] = k[i + j] - u;
            let mut saved = 0.0;
            for r in 0..j {
                ndu[j][r] = right[r + 1] + left[j - r];
                let temp = if ndu[j][r] == 0.0 {
                    0.0
                } else {
                    ndu[r][j - 1] / ndu[j][r]
                };
                ndu[r][j] = saved + right[r + 1] * temp;
                saved = left[j - r] * temp;
            }
            ndu[j][j] = saved;
        }
        let nd = nd.min(p);
        let mut ders = vec![vec![0.0; p + 1]; nd + 1];
        for j in 0..=p {
            ders[0][j] = ndu[j][p];
        }
        let mut a = [vec![0.0; p + 1], vec![0.0; p + 1]];
        for r in 0..=p {
            let (mut s1, mut s2) = (0usize, 1usize);
            a[0][0] = 1.0;
            for kk in 1..=nd {
                let mut d = 0.0;
                let rk = r as isize - kk as isize;
                let pk = p as isize - kk as isize;
                if r >= kk {
                    let den = ndu[(pk + 1) as usize][rk as usize];
                    a[s2][0] = if den == 0.0 { 0.0 } else { a[s1][0] / den };
                    d = a[s2][0] * ndu[rk as usize][pk as usize];
                }
                let j1 = if rk >= -1 { 1 } else { (-rk) as usize };
                let j2 = if (r as isize - 1) <= pk {
                    kk - 1
                } else {
                    p - r
                };
                for j in j1..=j2 {
                    let den = ndu[(pk + 1) as usize][(rk + j as isize) as usize];
                    a[s2][j] = if den == 0.0 {
                        0.0
                    } else {
                        (a[s1][j] - a[s1][j - 1]) / den
                    };
                    d += a[s2][j] * ndu[(rk + j as isize) as usize][pk as usize];
                }
                if r as isize <= pk {
                    let den = ndu[(pk + 1) as usize][r];
                    a[s2][kk] = if den == 0.0 {
                        0.0
                    } else {
                        -a[s1][kk - 1] / den
                    };
                    d += a[s2][kk] * ndu[r][pk as usize];
                }
                ders[kk][r] = d;
                std::mem::swap(&mut s1, &mut s2);
            }
        }
        let mut f = p as f64;
        for kk in 1..=nd {
            for j in 0..=p {
                ders[kk][j] *= f;
            }
            f *= (p - kk) as f64;
        }
        ders
    }

    /// The point and its derivatives to `nd` (2 at most) at `u` (A4.2 for the rational curve).
    pub fn derivs(&self, u: f64, nd: usize) -> Vec<Vec2> {
        let (lo, hi) = self.domain();
        let u = u.clamp(lo, hi);
        let p = self.degree;
        let i = self.span(u);
        let nd = nd.min(2);
        let n = self.basis_ders(i, u, nd);
        // Relative to the span's first control point: a rational curve moves with its control points,
        // and far from the origin (map coordinates) the derivatives keep their digits.
        let o = self.ctrl[i - p];
        // Aders (weighted point) and wders.
        let mut aders = vec![v(0.0, 0.0); nd + 1];
        let mut wders = vec![0.0; nd + 1];
        for k in 0..=nd {
            if k > p {
                break;
            }
            for j in 0..=p {
                let c = i - p + j;
                let w = self.w(c);
                aders[k].x += n[k][j] * (self.ctrl[c].x - o.x) * w;
                aders[k].y += n[k][j] * (self.ctrl[c].y - o.y) * w;
                wders[k] += n[k][j] * w;
            }
        }
        let binom = |a: usize, b: usize| -> f64 {
            let mut r = 1.0;
            for t in 0..b {
                r = r * (a - t) as f64 / (t + 1) as f64;
            }
            r
        };
        let mut ck = vec![v(0.0, 0.0); nd + 1];
        for k in 0..=nd {
            let mut x = aders[k].x;
            let mut y = aders[k].y;
            for j in 1..=k {
                let b = binom(k, j) * wders[j];
                x -= b * ck[k - j].x;
                y -= b * ck[k - j].y;
            }
            ck[k] = v(x / wders[0], y / wders[0]);
        }
        ck[0] = v(ck[0].x + o.x, ck[0].y + o.y);
        ck
    }

    /// The point at `u` (de Boor in homogeneous coordinates).
    pub fn point(&self, u: f64) -> Vec2 {
        let (lo, hi) = self.domain();
        let u = u.clamp(lo, hi);
        let p = self.degree;
        let k = self.span(u);
        // Relative to the span's first control point, as `derivs`.
        let o = self.ctrl[k - p];
        let mut d: Vec<[f64; 3]> = (0..=p)
            .map(|j| {
                let i = k - p + j;
                let w = self.w(i);
                [(self.ctrl[i].x - o.x) * w, (self.ctrl[i].y - o.y) * w, w]
            })
            .collect();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = k - p + j;
                let den = self.knots[i + p + 1 - r] - self.knots[i];
                let a = if den == 0.0 {
                    0.0
                } else {
                    (u - self.knots[i]) / den
                };
                let prev = d[j - 1];
                for (c, x) in d[j].iter_mut().enumerate() {
                    *x = (1.0 - a) * prev[c] + a * *x;
                }
            }
        }
        let [x, y, w] = d[p];
        v(x / w + o.x, y / w + o.y)
    }

    /// The distinct knots inside the domain, both ends included: the spans the curve is smooth on.
    pub fn breaks(&self) -> Vec<f64> {
        let (p, n) = (self.degree, self.ctrl.len());
        let mut b: Vec<f64> = self.knots[p..=n].to_vec();
        b.dedup();
        b
    }

    /// The curve as points, every chord within `tol` of it (its ends exact).
    pub fn tessellate(&self, tol: f64) -> Vec<Vec2> {
        self.tessellate_bounded(tol, usize::MAX).unwrap_or_default()
    }

    /// The same tolerance rule with a hard point budget. None means invalid input or a budget
    /// exhausted; no partial curve is returned (paper CAD output, ADR 0593).
    pub fn tessellate_bounded(&self, tol: f64, limit: usize) -> Option<Vec<Vec2>> {
        if !self.valid() || !tol.is_finite() || tol <= 0. || limit < 2 {
            return None;
        }
        let breaks = self.breaks();
        let mut out = vec![self.point(*breaks.first()?)];
        for w in breaks.windows(2) {
            let (u0, u1) = (w[0], w[1]);
            let p0 = *out.last()?;
            let p1 = self.point(u1);
            let min = if self.degree >= 2 { 2 } else { 0 };
            if !self.refine(tol, u0, p0, u1, p1, 0, min, limit, &mut out) {
                return None;
            }
        }
        Some(out)
    }

    #[allow(clippy::too_many_arguments)]
    fn refine(
        &self,
        tol: f64,
        u0: f64,
        p0: Vec2,
        u1: f64,
        p1: Vec2,
        depth: u32,
        min: u32,
        limit: usize,
        out: &mut Vec<Vec2>,
    ) -> bool {
        if out.len() >= limit {
            return false;
        }
        let um = 0.5 * (u0 + u1);
        let pm = self.point(um);
        let q1 = self.point(0.5 * (u0 + um));
        let q3 = self.point(0.5 * (um + u1));
        let far = js_max(
            js_max(chord_distance(pm, p0, p1), chord_distance(q1, p0, p1)),
            chord_distance(q3, p0, p1),
        );
        if depth >= DEPTH && far > tol && limit != usize::MAX {
            return false;
        }
        if depth < DEPTH && (depth < min || far > tol) {
            self.refine(tol, u0, p0, um, pm, depth + 1, min, limit, out)
                && self.refine(tol, um, pm, u1, p1, depth + 1, min, limit, out)
        } else {
            out.push(p1);
            true
        }
    }

    /// The length of the curve between `a` and `b` (parameters), span by span.
    pub fn length_between(&self, a: f64, b: f64) -> f64 {
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let speed = |u: f64| {
            let d = self.derivs(u, 1)[1];
            (d.x * d.x + d.y * d.y).sqrt()
        };
        let mut total = 0.0;
        for w in self.breaks().windows(2) {
            let (s, e) = (js_max(w[0], a), js_min(w[1], b));
            if e > s {
                total += integrate(&speed, s, e, 1e-14);
            }
        }
        total
    }

    /// The whole curve's length.
    pub fn length(&self) -> f64 {
        let (lo, hi) = self.domain();
        self.length_between(lo, hi)
    }

    /// The area the curve and the chord from its end back to its start enclose, signed (positive
    /// counter-clockwise): Green's ½∮(x dy − y dx), span by span; for a closed curve its area.
    pub fn signed_area(&self) -> f64 {
        let f = |u: f64| {
            let d = self.derivs(u, 1);
            0.5 * (d[0].x * d[1].y - d[0].y * d[1].x)
        };
        let mut total = 0.0;
        for w in self.breaks().windows(2) {
            total += integrate(&f, w[0], w[1], 1e-14);
        }
        let (lo, hi) = self.domain();
        let (a, b) = (self.point(hi), self.point(lo));
        total + 0.5 * (a.x * b.y - a.y * b.x)
    }

    /// The parameter at arc length `s` from the start (Newton on the length, kept in its bracket).
    pub fn param_at_length(&self, s: f64) -> f64 {
        let (lo, hi) = self.domain();
        let total = self.length();
        if s <= 0.0 {
            return lo;
        }
        if s >= total {
            return hi;
        }
        let (mut a, mut b) = (lo, hi);
        let mut u = lo + (hi - lo) * s / total;
        for _ in 0..60 {
            let f = self.length_between(lo, u) - s;
            if f.abs() <= 1e-12 * js_max(total, 1.0) {
                break;
            }
            if f > 0.0 {
                b = u;
            } else {
                a = u;
            }
            let d = self.derivs(u, 1)[1];
            let speed = (d.x * d.x + d.y * d.y).sqrt();
            let next = if speed > 0.0 { u - f / speed } else { f64::NAN };
            u = if next.is_finite() && next > a && next < b {
                next
            } else {
                0.5 * (a + b)
            };
        }
        u
    }

    /// The parameter of the point nearest `q` (a sampled start refined by Newton's method).
    pub fn closest(&self, q: Vec2) -> f64 {
        let (lo, hi) = self.domain();
        let breaks = self.breaks();
        let per = 8 * self.degree.max(1);
        let (mut best_u, mut best_d) = (lo, f64::INFINITY);
        for w in breaks.windows(2) {
            for s in 0..=per {
                let u = w[0] + (w[1] - w[0]) * s as f64 / per as f64;
                let p = self.point(u);
                let d = sq(p.x - q.x) + sq(p.y - q.y);
                if d < best_d {
                    best_d = d;
                    best_u = u;
                }
            }
        }
        let mut u = best_u;
        for _ in 0..30 {
            let d = self.derivs(u, 2);
            let r = v(d[0].x - q.x, d[0].y - q.y);
            let f = d[1].x * r.x + d[1].y * r.y;
            let df = d[2].x * r.x + d[2].y * r.y + d[1].x * d[1].x + d[1].y * d[1].y;
            if df == 0.0 {
                break;
            }
            let next = (u - f / df).clamp(lo, hi);
            if (next - u).abs() <= 1e-15 * js_max(hi - lo, 1.0) {
                u = next;
                break;
            }
            u = next;
        }
        // Newton may wander to a worse local minimum: keep the better of it and the sample.
        let dist = |u: f64| {
            let p = self.point(u);
            sq(p.x - q.x) + sq(p.y - q.y)
        };
        if dist(u) <= dist(best_u) { u } else { best_u }
    }

    /// Bounds of the curve: of its tessellation, which the convex hull of its control points contains.
    pub fn bounds(&self) -> [f64; 4] {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in self.tessellate(TOLERANCE) {
            b[0] = js_min(b[0], p.x);
            b[1] = js_min(b[1], p.y);
            b[2] = js_max(b[2], p.x);
            b[3] = js_max(b[3], p.y);
        }
        b
    }

    /// The curve with `u` inserted `r` times (A5.1, in homogeneous coordinates).
    pub fn insert(&self, u: f64, r: usize) -> OwnedNurbs {
        let p = self.degree;
        let np = self.ctrl.len() - 1;
        let k = self.span(u);
        let s = self.knots.iter().filter(|x| **x == u).count();
        let r = r.min(p.saturating_sub(s));
        let pw: Vec<[f64; 3]> = (0..=np)
            .map(|i| {
                let w = self.w(i);
                [self.ctrl[i].x * w, self.ctrl[i].y * w, w]
            })
            .collect();
        if r == 0 {
            return self.owned();
        }
        let mp = np + p + 1;
        let mut uq = Vec::with_capacity(mp + r + 1);
        uq.extend_from_slice(&self.knots[..=k]);
        uq.extend(std::iter::repeat_n(u, r));
        uq.extend_from_slice(&self.knots[k + 1..]);
        let mut qw = vec![[0.0; 3]; np + r + 1];
        qw[..=(k - p)].copy_from_slice(&pw[..=(k - p)]);
        qw[k - s + r..=np + r].copy_from_slice(&pw[k - s..=np]);
        let mut rw: Vec<[f64; 3]> = (0..=(p - s)).map(|i| pw[k - p + i]).collect();
        let mut l = 0;
        for j in 1..=r {
            l = k - p + j;
            for i in 0..=(p - j - s) {
                let alpha = (u - self.knots[l + i]) / (self.knots[i + k + 1] - self.knots[l + i]);
                for c in 0..3 {
                    rw[i][c] = alpha * rw[i + 1][c] + (1.0 - alpha) * rw[i][c];
                }
            }
            qw[l] = rw[0];
            qw[k + r - j - s] = rw[p - j - s];
        }
        if l + 1 < k - s {
            qw[l + 1..k - s].copy_from_slice(&rw[1..k - s - l]);
        }
        from_homogeneous(p, uq, &qw, self.weights.is_some())
    }

    fn owned(&self) -> OwnedNurbs {
        OwnedNurbs {
            degree: self.degree,
            ctrl: self.ctrl.to_vec(),
            knots: self.knots.to_vec(),
            weights: self.weights.map(<[f64]>::to_vec),
        }
    }

    /// The curve cut at `u` inside the domain: the part before and the part after, each the same curve.
    pub fn split(&self, u: f64) -> Option<(OwnedNurbs, OwnedNurbs)> {
        let (lo, hi) = self.domain();
        if !(u > lo && u < hi) {
            return None;
        }
        let p = self.degree;
        let full = self.insert(u, p);
        let first = full.knots.iter().position(|x| *x == u)?;
        // u now appears p times from `first` (or more, where it already was a knot of multiplicity ≥ p).
        let mut left_knots = full.knots[..first + p].to_vec();
        left_knots.push(u);
        let left_n = left_knots.len() - p - 1;
        let mut right_knots = vec![u];
        right_knots.extend_from_slice(&full.knots[first..]);
        let right_n = right_knots.len() - p - 1;
        let total = full.ctrl.len();
        let take = |range: std::ops::Range<usize>| OwnedNurbs {
            degree: p,
            ctrl: full.ctrl[range.clone()].to_vec(),
            knots: Vec::new(),
            weights: full.weights.as_ref().map(|w| w[range].to_vec()),
        };
        let mut left = take(0..left_n);
        left.knots = left_knots;
        let mut right = take(total - right_n..total);
        right.knots = right_knots;
        Some((left, right))
    }

    /// The part of the curve between parameters `a` < `b` in the domain.
    pub fn sub(&self, a: f64, b: f64) -> Option<OwnedNurbs> {
        let (lo, hi) = self.domain();
        let (a, b) = (js_max(a, lo), js_min(b, hi));
        if !(b > a) {
            return None;
        }
        let right = if a > lo {
            self.split(a)?.1
        } else {
            self.owned()
        };
        if b < hi {
            Some(right.view().split(b)?.0)
        } else {
            Some(right)
        }
    }

    /// The same curve run backwards.
    pub fn reversed(&self) -> OwnedNurbs {
        let (lo, hi) = (self.knots[0], self.knots[self.knots.len() - 1]);
        let mut ctrl = self.ctrl.to_vec();
        ctrl.reverse();
        let mut knots: Vec<f64> = self.knots.iter().map(|k| lo + hi - k).collect();
        knots.reverse();
        let weights = self.weights.map(|w| {
            let mut w = w.to_vec();
            w.reverse();
            w
        });
        OwnedNurbs {
            degree: self.degree,
            ctrl,
            knots,
            weights,
        }
    }
}

fn from_homogeneous(p: usize, knots: Vec<f64>, qw: &[[f64; 3]], rational: bool) -> OwnedNurbs {
    let ctrl = qw.iter().map(|q| v(q[0] / q[2], q[1] / q[2])).collect();
    let weights = rational.then(|| qw.iter().map(|q| q[2]).collect());
    OwnedNurbs {
        degree: p,
        ctrl,
        knots,
        weights,
    }
}

fn chord_distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    if l2 == 0.0 {
        return (sq(p.x - a.x) + sq(p.y - a.y)).sqrt();
    }
    ((p.x - a.x) * dy - (p.y - a.y) * dx).abs() / l2.sqrt()
}

fn sq(x: f64) -> f64 {
    x * x
}

/// A NURBS shape's curve, when its numbers make one.
pub fn of(shape: &crate::entity::Shape) -> Option<Nurbs<'_>> {
    match shape {
        crate::entity::Shape::Nurbs {
            degree,
            ctrl,
            knots,
            weights,
            ..
        } => {
            let n = Nurbs {
                degree: *degree,
                ctrl,
                knots,
                weights: weights.as_deref(),
            };
            n.valid().then_some(n)
        }
        _ => None,
    }
}

/// A NURBS shape with another curve: its closed and periodic words kept, its fit points dropped (the
/// curve is no longer the one drawn through them).
pub fn with_curve(
    like: &crate::entity::Shape,
    c: OwnedNurbs,
    closed: bool,
) -> crate::entity::Shape {
    let periodic = matches!(like, crate::entity::Shape::Nurbs { periodic: true, .. }) && closed;
    crate::entity::Shape::Nurbs {
        degree: c.degree,
        ctrl: c.ctrl,
        knots: c.knots,
        weights: c.weights,
        closed,
        periodic,
        fit: None,
        fit_tolerance: None,
        tangents: None,
    }
}

/// The curve as points for drawing and hit tests: `tessellate` at the drawing's tolerance, or the
/// control polygon when the numbers make no curve (shown, not hidden).
pub fn outline(shape: &crate::entity::Shape) -> Vec<Vec2> {
    match (of(shape), shape) {
        (Some(n), _) => n.tessellate(TOLERANCE),
        (None, crate::entity::Shape::Nurbs { ctrl, .. }) => ctrl.clone(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_1_SQRT_2, PI};

    fn quarter() -> (Vec<Vec2>, Vec<f64>, Vec<f64>) {
        (
            vec![v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)],
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            vec![1.0, FRAC_1_SQRT_2, 1.0],
        )
    }

    #[test]
    fn a_rational_quarter_circle_is_a_circle() {
        let (c, k, w) = quarter();
        let n = Nurbs {
            degree: 2,
            ctrl: &c,
            knots: &k,
            weights: Some(&w),
        };
        assert!(n.valid());
        for i in 0..=10 {
            let p = n.point(i as f64 / 10.0);
            assert!(((p.x * p.x + p.y * p.y).sqrt() - 1.0).abs() < 1e-15);
        }
        assert!((n.length() - PI / 2.0).abs() < 1e-14, "{}", n.length());
        // The tangent at the start is vertical, the curvature's sign towards the centre.
        let d = n.derivs(0.0, 2);
        assert!(d[1].x.abs() < 1e-15 && d[1].y > 0.0);
        let mid = n.point(n.param_at_length(PI / 4.0));
        assert!(
            (mid.x - FRAC_1_SQRT_2).abs() < 1e-12 && (mid.y - FRAC_1_SQRT_2).abs() < 1e-12,
            "{mid:?}"
        );
        let u = n.closest(v(2.0, 2.0));
        let q = n.point(u);
        assert!(
            (q.x - FRAC_1_SQRT_2).abs() < 1e-12 && (q.y - FRAC_1_SQRT_2).abs() < 1e-12,
            "{q:?}"
        );
    }

    #[test]
    fn a_split_keeps_the_curve() {
        let c = vec![
            v(0.0, 0.0),
            v(1.0, 3.0),
            v(3.0, 3.0),
            v(4.0, 0.0),
            v(6.0, -2.0),
        ];
        let k = vec![0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0];
        let w = [1.0, 2.0, 0.5, 1.0, 1.5];
        for weights in [None, Some(&w[..])] {
            let n = Nurbs {
                degree: 3,
                ctrl: &c,
                knots: &k,
                weights,
            };
            for cut in [0.2, 0.5, 0.77] {
                let (a, b) = n.split(cut).expect("a split");
                assert!(a.view().valid() && b.view().valid(), "{a:?} {b:?}");
                for i in 0..=20 {
                    let t = i as f64 / 20.0;
                    let ua = cut * t;
                    let pa = a.view().point(ua);
                    let pn = n.point(ua);
                    assert!((pa.x - pn.x).abs() < 1e-12 && (pa.y - pn.y).abs() < 1e-12);
                    let ub = cut + (1.0 - cut) * t;
                    let pb = b.view().point(ub);
                    let pn = n.point(ub);
                    assert!((pb.x - pn.x).abs() < 1e-12 && (pb.y - pn.y).abs() < 1e-12);
                }
                let total = a.view().length() + b.view().length();
                assert!((total - n.length()).abs() < 1e-11);
            }
            let r = n.reversed();
            let (lo, hi) = r.view().domain();
            let p = r.view().point(lo);
            assert_eq!((p.x, p.y), (6.0, -2.0));
            assert!((r.view().length() - n.length()).abs() < 1e-12);
            assert_eq!(r.view().point(hi), v(0.0, 0.0));
        }
    }

    #[test]
    fn tessellation_stays_within_the_tolerance() {
        let (c, k, w) = quarter();
        let n = Nurbs {
            degree: 2,
            ctrl: &c,
            knots: &k,
            weights: Some(&w),
        };
        let pts = n.tessellate(1e-5);
        for pair in pts.windows(2) {
            let m = v((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0);
            assert!(1.0 - (m.x * m.x + m.y * m.y).sqrt() <= 1e-5);
        }
        assert_eq!(pts[0], v(1.0, 0.0));
        assert_eq!(*pts.last().unwrap(), v(0.0, 1.0));
    }
}

#[cfg(test)]
mod bounded_tests {
    use super::*;
    #[test]
    fn bounded_tessellation_is_complete_or_fails_without_a_partial_curve() {
        let ctrl = [Vec2::new(1., 0.), Vec2::new(1., 1.), Vec2::new(0., 1.)];
        let knots = [0., 0., 0., 1., 1., 1.];
        let weights = [1., core::f64::consts::FRAC_1_SQRT_2, 1.];
        let n = Nurbs {
            degree: 2,
            ctrl: &ctrl,
            knots: &knots,
            weights: Some(&weights),
        };
        assert!(n.tessellate_bounded(1e-8, 2).is_none());
        assert!(n.tessellate_bounded(0., 1000).is_none());
        let points = n.tessellate_bounded(1e-4, 1000).unwrap();
        assert_eq!(points.first(), Some(&ctrl[0]));
        assert_eq!(points.last(), Some(&ctrl[2]));
        for p in points {
            assert!((p.x.hypot(p.y) - 1.).abs() < 1e-12);
        }
    }
}
