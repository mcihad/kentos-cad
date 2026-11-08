//! Student's t quantile for a parameter's test (docs/adr/2160): the regularized incomplete beta function by Lentz's
//! continued fraction (as `chi2.rs`'s gamma), and the quantile by bisection. libm's lgamma, exp and log give every
//! target the same bits.

use crate::jsmath::{exp, log};

const TINY: f64 = 1e-300;

/// The continued fraction of the incomplete beta function (modified Lentz).
fn fraction(a: f64, b: f64, x: f64) -> f64 {
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..2000 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-16 {
            break;
        }
    }
    h
}

/// Iₓ(a, b): the regularized incomplete beta function, x in [0, 1].
pub fn incomplete_beta(a: f64, b: f64, x: f64) -> f64 {
    if !(x > 0.0) {
        return 0.0;
    }
    if !(x < 1.0) {
        return 1.0;
    }
    let front = exp(libm::lgamma(a + b) - libm::lgamma(a) - libm::lgamma(b) + a * log(x) + b * log(1.0 - x));
    if x < (a + 1.0) / (a + b + 2.0) {
        front * fraction(a, b, x) / a
    } else {
        1.0 - front * fraction(b, a, 1.0 - x) / b
    }
}

/// P(T ≤ t) for Student's t with `f` degrees of freedom.
pub fn t_cdf(f: f64, t: f64) -> f64 {
    let tail = 0.5 * incomplete_beta(f / 2.0, 0.5, f / (f + t * t));
    if t >= 0.0 { 1.0 - tail } else { tail }
}

/// The t a Student's t with `f` degrees of freedom stays below with probability `p` (p ≥ 0.5): a two-sided test at
/// confidence 1 − α takes p = 1 − α/2.
pub fn t_quantile(f: f64, p: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    while t_cdf(f, hi) < p && hi < 1e9 {
        hi *= 2.0;
    }
    for _ in 0..400 {
        let mid = (lo + hi) / 2.0;
        if !(mid > lo && mid < hi) {
            break;
        }
        if t_cdf(f, mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// scipy.stats.t.ppf's values (scipy 1.18.0, QGIS's): two-sided 95 % at 2, 5, 10 and 30 degrees of freedom,
    /// one-sided 95 % at 5; scipy.special.betainc(2, 3, 0.4).
    #[test]
    fn the_quantiles_are_scipys() {
        for (f, p, want) in [
            (2.0, 0.975, 4.302652729749462),
            (5.0, 0.975, 2.5705818356363146),
            (10.0, 0.975, 2.228138851986274),
            (30.0, 0.975, 2.0422724563012378),
            (5.0, 0.95, 2.015048373333023),
        ] {
            let got = t_quantile(f, p);
            assert!((got - want).abs() < 1e-9, "t({p}, {f}) = {got}, scipy {want}");
        }
        assert!((t_cdf(4.0, 0.0) - 0.5).abs() < 1e-15);
        assert!((incomplete_beta(2.0, 3.0, 0.4) - 0.5247999999999999).abs() < 1e-12, "I_0.4(2, 3)");
    }
}
