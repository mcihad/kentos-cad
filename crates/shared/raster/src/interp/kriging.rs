//! Kriging (docs/adr/0232 §9): ordinary kriging over a cell's nearest
//! points with a spherical, exponential or Gaussian variogram, given or
//! fitted to the empirical variogram by weighted least squares.

use kentos_geometry_core::vec2::Vec2;

use crate::solve::Lu;

/// The variogram's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Spherical,
    Exponential,
    Gaussian,
}

impl Model {
    /// f(u), u = h / range.
    pub fn f(self, u: f64) -> f64 {
        match self {
            Model::Spherical if u < 1.0 => 1.5 * u - 0.5 * u * u * u,
            Model::Spherical => 1.0,
            Model::Exponential => 1.0 - libm::exp(-3.0 * u),
            Model::Gaussian => 1.0 - libm::exp(-3.0 * u * u),
        }
    }

    /// The model's Turkish name.
    pub fn name(self) -> &'static str {
        match self {
            Model::Spherical => "Küresel",
            Model::Exponential => "Üstel",
            Model::Gaussian => "Gauss",
        }
    }
}

/// A variogram: γ(0) = 0, γ(h) = nugget + sill · f(h / range) for h > 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variogram {
    pub model: Model,
    pub nugget: f64,
    pub sill: f64,
    pub range: f64,
}

impl Variogram {
    pub fn at(&self, h: f64) -> f64 {
        if h > 0.0 {
            self.nugget + self.sill * self.model.f(h / self.range)
        } else {
            0.0
        }
    }

    /// Why the variogram cannot be kriged with, if it cannot.
    pub fn problem(&self) -> Option<String> {
        if !(self.range > 0.0) || !self.range.is_finite() {
            return Some("Variogramın erimi 0'dan büyük olmalı.".into());
        }
        if !(self.nugget >= 0.0)
            || !(self.sill >= 0.0)
            || !self.nugget.is_finite()
            || !self.sill.is_finite()
        {
            return Some("Variogramın külçesi ve kısmi eşiği eksi olamaz.".into());
        }
        if self.nugget + self.sill == 0.0 {
            return Some(
                "Variogram düz (külçe ve kısmi eşik 0): değerler aynı, kriging yapılamaz.".into(),
            );
        }
        None
    }
}

/// The empirical variogram's bins: their mean distance, semivariance and pair count.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bins {
    pub h: Vec<f64>,
    pub gamma: Vec<f64>,
    pub n: Vec<f64>,
}

/// The points used to fit at most.
pub const FIT_POINTS: usize = 4_000;

/// The empirical variogram (§9): the points (every ⌈n / 4000⌉-th past 4000),
/// pairs i < j up to L = half the box's diagonal in `lags` equal bins.
pub fn bins(xy: &[Vec2], v: &[f64], lags: usize) -> Bins {
    let n = xy.len();
    let step = n.div_ceil(FIT_POINTS).max(1);
    let idx: Vec<usize> = (0..n).step_by(step).collect();
    let b = crate::points::bounds_of(idx.iter().map(|&i| xy[i])).unwrap_or([0.0; 4]);
    let diag = ((b[2] - b[0]) * (b[2] - b[0]) + (b[3] - b[1]) * (b[3] - b[1])).sqrt();
    let most = diag / 2.0;
    let lags = lags.max(1);
    let w = most / lags as f64;
    let mut count = vec![0.0f64; lags];
    let mut sum_h = vec![0.0f64; lags];
    let mut sum_g = vec![0.0f64; lags];
    if w > 0.0 {
        for (a, &i) in idx.iter().enumerate() {
            for &j in &idx[a + 1..] {
                let (dx, dy) = (xy[i].x - xy[j].x, xy[i].y - xy[j].y);
                let h = (dx * dx + dy * dy).sqrt();
                if h > most || h == 0.0 {
                    continue;
                }
                let k = ((h / w).floor() as usize).min(lags - 1);
                let d = v[i] - v[j];
                count[k] += 1.0;
                sum_h[k] += h;
                sum_g[k] += d * d / 2.0;
            }
        }
    }
    let mut out = Bins::default();
    for k in 0..lags {
        if count[k] > 0.0 {
            out.h.push(sum_h[k] / count[k]);
            out.gamma.push(sum_g[k] / count[k]);
            out.n.push(count[k]);
        }
    }
    out
}

/// The best nugget and sill (both ≥ 0) for `range`, and the weighted squared error.
pub fn best_at(bins: &Bins, model: Model, range: f64) -> (f64, f64, f64) {
    let f: Vec<f64> = bins.h.iter().map(|h| model.f(h / range)).collect();
    let (mut sn, mut snf, mut snff, mut sng, mut snfg) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for ((&fk, &n), &g) in f.iter().zip(&bins.n).zip(&bins.gamma) {
        sn += n;
        snf += n * fk;
        snff += n * fk * fk;
        sng += n * g;
        snfg += n * fk * g;
    }
    let err = |c0: f64, s: f64| {
        (0..f.len())
            .map(|k| {
                let r = bins.gamma[k] - c0 - s * f[k];
                bins.n[k] * r * r
            })
            .sum::<f64>()
    };
    // Only the sill (nugget 0), only the nugget (sill 0): the bounds' solutions.
    let sill_only = if snff > 0.0 {
        (snfg / snff).max(0.0)
    } else {
        0.0
    };
    let nugget_only = if sn > 0.0 { (sng / sn).max(0.0) } else { 0.0 };
    let det = sn * snff - snf * snf;
    if det > 1e-12 * sn * snff {
        let c0 = (snff * sng - snf * snfg) / det;
        let s = (sn * snfg - snf * sng) / det;
        if c0 >= 0.0 && s >= 0.0 {
            return (c0, s, err(c0, s));
        }
    }
    let (e1, e2) = (err(0.0, sill_only), err(nugget_only, 0.0));
    if e1 <= e2 {
        (0.0, sill_only, e1)
    } else {
        (nugget_only, 0.0, e2)
    }
}

/// The variogram of `model` fitted to the empirical one (§9): 200 ranges
/// from w/2 to 2L evenly on a log scale, the best's neighbours searched by
/// the golden ratio for 100 steps.
pub fn fit(xy: &[Vec2], v: &[f64], model: Model, lags: usize) -> Result<Variogram, String> {
    let b = bins(xy, v, lags);
    if b.h.len() < 3 {
        return Err("Variogram için yeterli nokta çifti yok (üçten az dolu aralık): Variogram'ı elle girin.".into());
    }
    let n = xy.len();
    let step = n.div_ceil(FIT_POINTS).max(1);
    let bounds = crate::points::bounds_of((0..n).step_by(step).map(|i| xy[i])).unwrap_or([0.0; 4]);
    let (bw, bh) = (bounds[2] - bounds[0], bounds[3] - bounds[1]);
    let diag = (bw * bw + bh * bh).sqrt();
    let most = diag / 2.0;
    let w = most / lags.max(1) as f64;
    let (lo, hi) = (libm::log(w / 2.0), libm::log(2.0 * most));
    let candidate = |m: usize| libm::exp(lo + (hi - lo) * m as f64 / 199.0);
    let mut best = (0usize, f64::INFINITY);
    for m in 0..200 {
        let e = best_at(&b, model, candidate(m)).2;
        if e < best.1 {
            best = (m, e);
        }
    }
    let (mut a, mut c) = (
        candidate(best.0.saturating_sub(1)),
        candidate((best.0 + 1).min(199)),
    );
    let ratio = (5f64.sqrt() - 1.0) / 2.0;
    let mut x1 = c - ratio * (c - a);
    let mut x2 = a + ratio * (c - a);
    let mut f1 = best_at(&b, model, x1).2;
    let mut f2 = best_at(&b, model, x2).2;
    for _ in 0..100 {
        if f1 <= f2 {
            c = x2;
            x2 = x1;
            f2 = f1;
            x1 = c - ratio * (c - a);
            f1 = best_at(&b, model, x1).2;
        } else {
            a = x1;
            x1 = x2;
            f1 = f2;
            x2 = a + ratio * (c - a);
            f2 = best_at(&b, model, x2).2;
        }
    }
    let range = (a + c) / 2.0;
    let (nugget, sill, _) = best_at(&b, model, range);
    let g = Variogram {
        model,
        nugget,
        sill,
        range,
    };
    match g.problem() {
        Some(p) => Err(p),
        None => Ok(g),
    }
}

/// Ordinary kriging's system for some points (Γ bordered by ones), factored; none when singular.
pub fn system(g: &Variogram, pts: &[Vec2]) -> Option<Lu> {
    let m = pts.len();
    let n = m + 1;
    let mut a = vec![0.0; n * n];
    for i in 0..m {
        for j in 0..m {
            if i != j {
                let (dx, dy) = (pts[i].x - pts[j].x, pts[i].y - pts[j].y);
                a[i * n + j] = g.at((dx * dx + dy * dy).sqrt());
            }
        }
        a[i * n + m] = 1.0;
        a[m * n + i] = 1.0;
    }
    Lu::factor(a, n)
}

/// The prediction and its variance at `q` from the factored system of `pts` with values `z`.
pub fn predict(
    g: &Variogram,
    lu: &Lu,
    pts: &[Vec2],
    z: &[f64],
    q: Vec2,
    b: &mut Vec<f64>,
    tmp: &mut Vec<f64>,
) -> (f64, f64) {
    let m = pts.len();
    b.clear();
    for p in pts {
        let (dx, dy) = (p.x - q.x, p.y - q.y);
        b.push(g.at((dx * dx + dy * dy).sqrt()));
    }
    b.push(1.0);
    let gamma0: Vec<f64> = b[..m].to_vec();
    lu.solve(b, tmp);
    let mut pred = 0.0;
    let mut var = 0.0;
    for i in 0..m {
        pred += b[i] * z[i];
        var += b[i] * gamma0[i];
    }
    var += b[m];
    (pred, var)
}
