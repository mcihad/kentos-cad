//! Spline (docs/adr/0232 §8): ArcGIS's regularized and tension splines
//! (Mitas and Mitasova 1988) over a cell's nearest points. The basis
//! functions need K₀, the modified Bessel function of the second kind: its
//! series up to x = 2; beyond, e^x·√x·K₀(x) as a Chebyshev series in
//! u = 4/x − 1 (Cephes's way, its coefficients from mpmath:
//! scripts/fixtures/bessel_k0.py); nothing past 700. The independent
//! reference is mpmath's `besselk` (scripts/fixtures/interpolation_cases.py).

use crate::solve::Lu;

/// Euler's constant.
pub const EULER: f64 = 0.577_215_664_901_532_9;

/// e^x·√x·K₀(x) = Σ cₖ Tₖ(4/x − 1) for x ≥ 2 (scripts/fixtures/bessel_k0.py writes it).
#[rustfmt::skip]
const K0_CHEBYSHEV: [f64; 30] = [
    1.2201515410329777,
    -0.0314481013119645,
    0.0015698838857300533,
    -0.00012849549581627802,
    1.39498137188765e-05,
    -1.8317555227191195e-06,
    2.766813639445015e-07,
    -4.660489897687948e-08,
    8.574034017414225e-09,
    -1.6975345093890614e-09,
    3.5773972814003283e-10,
    -7.957489244477396e-11,
    1.8559491149549264e-11,
    -4.514597883374519e-12,
    1.1403405882073441e-12,
    -2.9800969231481784e-13,
    8.032890775068375e-14,
    -2.2275133267462965e-14,
    6.340076476276646e-15,
    -1.848593377920907e-15,
    5.5120559994043335e-16,
    -1.6782311257549006e-16,
    5.2103917776435543e-17,
    -1.6475805939842632e-17,
    5.3004337711773354e-18,
    -1.7331712005821001e-18,
    5.755109202882729e-19,
    -1.9390956053183555e-19,
    6.624610534536147e-20,
    -2.2932197170560118e-20,
];

/// K₀(x) + γ + ln(x / 2), for x > 0: small near 0 (where K₀ ≈ −ln(x/2) − γ),
/// written so that it does not cancel there.
pub fn k0_tail(x: f64) -> f64 {
    if !(x > 0.0) {
        return 0.0;
    }
    if x <= 2.0 {
        // K₀ = −(ln(x/2) + γ)·I₀ + Σ_{k≥1} (x²/4)^k / (k!)² · H_k, I₀ = Σ_{k≥0} (x²/4)^k / (k!)²:
        // K₀ + γ + ln(x/2) = −(ln(x/2) + γ)·(I₀ − 1) + Σ_{k≥1} (x²/4)^k / (k!)² · H_k.
        let y = x * x / 4.0;
        let mut term = 1.0; // (x²/4)^k / (k!)²
        let mut h = 0.0; // H_k
        let mut i0m1 = 0.0;
        let mut series = 0.0;
        for k in 1..40 {
            let kf = f64::from(k);
            term *= y / (kf * kf);
            h += 1.0 / kf;
            i0m1 += term;
            series += term * h;
            if term < 1e-18 * series.abs().max(1e-300) {
                break;
            }
        }
        -(libm::log(x / 2.0) + EULER) * i0m1 + series
    } else {
        k0(x) + EULER + libm::log(x / 2.0)
    }
}

/// K₀(x) for x > 0; 0 past 700.
pub fn k0(x: f64) -> f64 {
    if x > 700.0 {
        return 0.0;
    }
    if !(x > 0.0) {
        return f64::INFINITY;
    }
    if x <= 2.0 {
        return k0_tail(x) - EULER - libm::log(x / 2.0);
    }
    // Clenshaw's recurrence for Σ cₖ Tₖ(u).
    let u = 4.0 / x - 1.0;
    let (mut b1, mut b2) = (0.0, 0.0);
    for &c in K0_CHEBYSHEV.iter().skip(1).rev() {
        let b0 = c + 2.0 * u * b1 - b2;
        b2 = b1;
        b1 = b0;
    }
    let series = K0_CHEBYSHEV[0] + u * b1 - b2;
    series * libm::exp(-x) / x.sqrt()
}

/// Which spline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Regularized,
    Tension,
}

/// A spline's basis: its kind and its parameter (τ for the regularized,
/// φ for the tension spline; their squares are the Ağırlık).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Basis {
    pub kind: Kind,
    /// τ or φ.
    pub p: f64,
}

impl Basis {
    /// The basis of `kind` with Ağırlık `weight`; why not when it is out of range.
    pub fn new(kind: Kind, weight: f64) -> Result<Basis, String> {
        match kind {
            Kind::Regularized if (0.0..=5.0).contains(&weight) => Ok(Basis {
                kind,
                p: weight.sqrt(),
            }),
            Kind::Regularized => {
                Err("Düzenlemeli spline'ın Ağırlık'ı 0 ile 5 arasında olmalı.".into())
            }
            Kind::Tension if weight > 0.0 && weight <= 100.0 => Ok(Basis {
                kind,
                p: weight.sqrt(),
            }),
            Kind::Tension => {
                Err("Gerilimli spline'ın Ağırlık'ı 0'dan büyük, en çok 100 olmalı.".into())
            }
        }
    }

    /// R(r); R(0) = 0. Past x = 40, K₀(x) is below half an ulp of γ: left
    /// out, the bits are the same; ln(r/2τ) and ln(x/2) are one value.
    pub fn r(&self, r: f64) -> f64 {
        if !(r > 0.0) {
            return 0.0;
        }
        let two_pi = 2.0 * std::f64::consts::PI;
        let tail = |x: f64, l: f64| {
            if x > 40.0 {
                EULER + l
            } else if x > 2.0 {
                k0(x) + EULER + l
            } else {
                k0_tail(x)
            }
        };
        match self.kind {
            // τ = 0: the thin-plate limit (the τ terms vanish; r²·const is absorbed by T).
            Kind::Regularized if self.p == 0.0 => r * r / 4.0 * libm::log(r) / two_pi,
            Kind::Regularized => {
                let tau = self.p;
                let x = r / tau;
                let l = libm::log(x / 2.0);
                (r * r / 4.0 * (l + EULER - 1.0) + tau * tau * tail(x, l)) / two_pi
            }
            Kind::Tension => {
                let phi = self.p;
                let x = r * phi;
                -tail(x, libm::log(x / 2.0)) / (two_pi * phi * phi)
            }
        }
    }

    /// How many terms T has: a₁ + a₂x + a₃y, or a₁.
    pub fn trend(&self) -> usize {
        match self.kind {
            Kind::Regularized => 3,
            Kind::Tension => 1,
        }
    }
}

/// A spline through some points (coordinates relative to their first): its λ's and T's terms.
#[derive(Clone, Debug, Default)]
pub struct Fit {
    pub lambda: Vec<f64>,
    pub trend: Vec<f64>,
}

/// The spline of `basis` through `pts` (relative coordinates) with values `z`; none when its system has no solution.
pub fn fit(basis: &Basis, pts: &[(f64, f64)], z: &[f64], tmp: &mut Vec<f64>) -> Option<Fit> {
    let m = pts.len();
    let t = basis.trend();
    if m < t {
        return None;
    }
    let n = m + t;
    let mut a = vec![0.0; n * n];
    for i in 0..m {
        // Symmetric: each pair once.
        for j in i + 1..m {
            let (dx, dy) = (pts[i].0 - pts[j].0, pts[i].1 - pts[j].1);
            let v = basis.r((dx * dx + dy * dy).sqrt());
            a[i * n + j] = v;
            a[j * n + i] = v;
        }
        let row = [1.0, pts[i].0, pts[i].1];
        for k in 0..t {
            a[i * n + m + k] = row[k];
            a[(m + k) * n + i] = row[k];
        }
    }
    let lu = Lu::factor(a, n)?;
    let mut b = vec![0.0; n];
    b[..m].copy_from_slice(z);
    lu.solve(&mut b, tmp);
    if b.iter().any(|v| !v.is_finite()) {
        return None;
    }
    Some(Fit {
        lambda: b[..m].to_vec(),
        trend: b[m..].to_vec(),
    })
}

impl Fit {
    /// The spline at (x, y) (relative coordinates), the points' terms in their order then T's.
    pub fn at(&self, basis: &Basis, pts: &[(f64, f64)], x: f64, y: f64) -> f64 {
        let mut s = 0.0;
        for (l, p) in self.lambda.iter().zip(pts) {
            let (dx, dy) = (x - p.0, y - p.1);
            s += l * basis.r((dx * dx + dy * dy).sqrt());
        }
        let row = [1.0, x, y];
        for (c, v) in self.trend.iter().zip(row) {
            s += c * v;
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k0_at_known_values() {
        // mpmath.besselk(0, x), 17 digits.
        for (x, want) in [
            (0.01, 4.721_244_730_161_095),
            (0.5, 0.924_419_071_227_665_9),
            (1.0, 0.421_024_438_240_708_3),
            (2.0, 0.113_893_872_749_533_44),
            (10.0, 1.778_006_231_616_765_4e-5),
            (100.0, 4.656_628_229_175_902e-45),
        ] {
            let got = if x <= 1.0 {
                k0_tail(x) - EULER - libm::log(x / 2.0)
            } else {
                k0(x)
            };
            assert!(
                ((got - want) / want).abs() < 2e-15,
                "K0({x}) = {got}, want {want}"
            );
        }
        assert_eq!(k0(800.0), 0.0);
        // Across the series' and the Chebyshev series' ranges (mpmath, 17 digits).
        for (x, want) in [
            (1.05, 0.392_162_980_372_192_5),
            (1.5, 0.213_805_562_647_525_73),
            (2.5, 0.062_347_553_200_366_19),
            (3.7, 0.015_630_659_921_626_66),
            (5.0, 0.003_691_098_334_042_594_2),
            (8.0, 0.000_146_470_705_222_815_4),
            (12.0, 2.200_825_397_311_491_6e-6),
            (15.9, 3.879_411_017_320_338e-8),
            (16.1, 3.156_694_217_415_958e-8),
            (20.0, 5.741_237_815_336_525e-10),
            (30.0, 2.132_477_496_463_056_3e-14),
            (50.0, 3.410_167_749_789_495_6e-23),
            (200.0, 1.225_681_979_776_533_6e-88),
            (400.0, 1.199_780_043_200_976e-175),
            (699.0, 1.270_284_188_032_741_8e-305),
        ] {
            let got = k0(x);
            assert!(
                ((got - want) / want).abs() < 4e-15,
                "K0({x}) = {got}, want {want}"
            );
        }
    }

    #[test]
    fn the_spline_goes_through_its_points() {
        let pts = [(0.0, 0.0), (10.0, 1.0), (3.0, 8.0), (12.0, 9.0), (6.0, 4.0)];
        let z = [1.0, 2.0, 0.5, 3.0, 1.7];
        for basis in [
            Basis::new(Kind::Regularized, 0.1).expect("weight"),
            Basis::new(Kind::Regularized, 0.0).expect("weight"),
            Basis::new(Kind::Tension, 0.1).expect("weight"),
        ] {
            let f = fit(&basis, &pts, &z, &mut Vec::new()).expect("a fit");
            for (p, v) in pts.iter().zip(z) {
                assert!((f.at(&basis, &pts, p.0, p.1) - v).abs() < 1e-9);
            }
        }
        // Three points on a line: no plane through them.
        let line = [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let b = Basis::new(Kind::Regularized, 0.1).expect("weight");
        assert!(fit(&b, &line, &[1.0, 2.0, 3.0], &mut Vec::new()).is_none());
    }
}
