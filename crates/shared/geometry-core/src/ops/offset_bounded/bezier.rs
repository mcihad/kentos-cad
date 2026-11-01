//! Rational Bézier interval bounds for a parallel curve. Bounds use positive
//! Bernstein weights; subdivision refines bounds, never a sampled error guess.

use crate::jsmath::{js_hypot, js_max, js_min};
use crate::vec2::Vec2;

#[derive(Clone, Debug)]
pub(super) struct Bezier(pub Vec<[f64; 3]>);

fn norm(v: [f64; 2]) -> f64 {
    js_hypot(v[0], v[1])
}
fn derivative(c: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let n = (c.len() - 1) as f64;
    if n == 0. {
        return vec![[0.; 3]];
    }
    c.windows(2)
        .map(|w| std::array::from_fn(|i| n * (w[1][i] - w[0][i])))
        .collect()
}
fn choose(n: usize, k: usize) -> f64 {
    (0..k.min(n - k)).fold(1., |v, i| v * (n - i) as f64 / (i + 1) as f64)
}
/// Coefficients of A'W − AW' (rational velocity times W²).
fn velocity(c: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let d = derivative(c);
    let (p, q) = (d.len() - 1, c.len() - 1);
    let mut out = vec![[0.; 3]; p + q + 1];
    for (i, a) in d.iter().enumerate() {
        for (j, b) in c.iter().enumerate() {
            let f = choose(p, i) * choose(q, j) / choose(p + q, i + j);
            for k in 0..2 {
                out[i + j][k] += f * (a[k] * b[2] - b[k] * a[2]);
            }
        }
    }
    out
}
fn cross_bounds(v: &[[f64; 3]]) -> (f64, f64) {
    let d = derivative(v);
    let (p, q) = (v.len() - 1, d.len() - 1);
    let mut coeff = vec![0.; p + q + 1];
    for (i, a) in v.iter().enumerate() {
        for (j, b) in d.iter().enumerate() {
            let f = choose(p, i) * choose(q, j) / choose(p + q, i + j);
            coeff[i + j] += f * (a[0] * b[1] - a[1] * b[0]);
        }
    }
    (
        coeff.iter().copied().fold(f64::INFINITY, js_min),
        coeff.iter().copied().fold(f64::NEG_INFINITY, js_max),
    )
}

pub(super) struct Bounds {
    /// Supremum of |q''| on this span's [0,1] parameter interval.
    pub second: f64,
    /// Lower/upper factors 1 − signed_distance * curvature. A nonpositive
    /// factor is an inward fold/cusp; a mere unresolved bound is subdivided.
    pub regular: (f64, f64),
}

impl Bezier {
    pub fn from_points(pts: &[Vec2], weights: Option<&[f64]>) -> Self {
        let max = weights.map_or(1., |w| w.iter().copied().fold(0., js_max));
        Self(
            pts.iter()
                .enumerate()
                .map(|(i, p)| {
                    let w = weights.map_or(1., |w| w[i]) / max;
                    [p.x * w, p.y * w, w]
                })
                .collect(),
        )
    }
    pub fn end(&self, last: bool, d: f64) -> Option<Vec2> {
        let i = if last { self.0.len() - 1 } else { 0 };
        let p = self.0[i];
        let t = if last { self.0[i - 1] } else { self.0[1] };
        let sign = if last { -1. } else { 1. };
        let v = [
            sign * (t[0] * p[2] - p[0] * t[2]),
            sign * (t[1] * p[2] - p[1] * t[2]),
        ];
        let speed = norm(v);
        if !(speed > 0.) || !speed.is_finite() {
            return None;
        }
        Some(Vec2::new(
            p[0] / p[2] - d * v[1] / speed,
            p[1] / p[2] + d * v[0] / speed,
        ))
    }
    pub fn split(&self) -> (Self, Self) {
        let mut c = self.0.clone();
        let mut left = vec![c[0]];
        let mut right = vec![c[c.len() - 1]];
        while c.len() > 1 {
            c = c
                .windows(2)
                .map(|w| std::array::from_fn(|k| w[0][k] * 0.5 + w[1][k] * 0.5))
                .collect();
            left.push(c[0]);
            right.push(c[c.len() - 1]);
        }
        right.reverse();
        (Self(left), Self(right))
    }
    pub fn bounds(&self, d: f64) -> Option<Bounds> {
        // Rebase before derivatives: subtracting large map coordinates must
        // not dominate the normal/curvature test.
        let o = [self.0[0][0] / self.0[0][2], self.0[0][1] / self.0[0][2]];
        let c: Vec<_> = self
            .0
            .iter()
            .map(|p| [p[0] - o[0] * p[2], p[1] - o[1] * p[2], p[2]])
            .collect();
        let wmin = c.iter().map(|p| p[2]).fold(f64::INFINITY, js_min);
        let wmax = c.iter().map(|p| p[2]).fold(0., js_max);
        if !(wmin > 0.) {
            return None;
        }
        let v = velocity(&c);
        let last = c.last()?;
        let axis = [last[0] / last[2], last[1] / last[2]];
        let len = norm(axis);
        if !(len > 0.) {
            return None;
        }
        let lower = v
            .iter()
            .map(|p| (p[0] * axis[0] + p[1] * axis[1]) / len)
            .fold(f64::INFINITY, js_min);
        let upper = v.iter().map(|p| norm([p[0], p[1]])).fold(0., js_max);
        // Leave a floating point guard on the convex-hull bounds.
        let lower = lower - upper * 1e-11;
        if !(lower > 0.) {
            return None;
        }
        let speed = lower / (wmax * wmax);
        let mut derivatives = Vec::new();
        let mut dc = c.clone();
        let b0 = c
            .iter()
            .map(|p| norm([p[0] / p[2], p[1] / p[2]]))
            .fold(0., js_max);
        for _ in 0..3 {
            dc = derivative(&dc);
            derivatives.push((
                dc.iter().map(|p| norm([p[0], p[1]])).fold(0., js_max),
                dc.iter().map(|p| p[2].abs()).fold(0., js_max),
            ));
        }
        let [(a1, w1), (a2, w2), (a3, w3)] = derivatives[..] else {
            unreachable!()
        };
        let b1 = (a1 + w1 * b0) / wmin;
        let b2 = (a2 + 2. * w1 * b1 + w2 * b0) / wmin;
        let b3 = (a3 + 3. * w1 * b2 + 3. * w2 * b1 + w3 * b0) / wmin;
        let second =
            (b2 + d.abs() * (2. * b3 / speed + 6. * ((b2 / speed) * (b2 / speed)))) * (1. + 1e-9);
        let (kmin, kmax) = cross_bounds(&v);
        let margin = upper * upper * (c.len() as f64) * (c.len() as f64) * 1e-10;
        let factors = [
            (wmin * wmin) / (upper * upper * upper),
            (wmax * wmax) / (lower * lower * lower),
        ];
        let ks = [
            (kmin - margin) * factors[0],
            (kmin - margin) * factors[1],
            (kmax + margin) * factors[0],
            (kmax + margin) * factors[1],
        ];
        let lo = ks
            .into_iter()
            .map(|k| 1. - d * k)
            .fold(f64::INFINITY, js_min);
        let hi = ks
            .into_iter()
            .map(|k| 1. - d * k)
            .fold(f64::NEG_INFINITY, js_max);
        (second.is_finite() && lo.is_finite() && hi.is_finite()).then_some(Bounds {
            second,
            regular: (lo, hi),
        })
    }
}
