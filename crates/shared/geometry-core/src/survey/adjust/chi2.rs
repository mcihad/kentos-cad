//! The χ² quantile of the model test (docs/adr/0203 §4): the regularized
//! lower incomplete gamma function, by its series below a + 1 and Lentz's
//! continued fraction above, and the quantile by bisection. libm's lgamma,
//! exp and log give every target the same bits.

use crate::api::Op;
use crate::jsmath::{exp, log};
use crate::op;

const TINY: f64 = 1e-300;

/// P(a, x): the regularized lower incomplete gamma function.
pub fn lower_gamma(a: f64, x: f64) -> f64 {
    if !(x > 0.0) {
        return 0.0;
    }
    let front = exp(-x + a * log(x) - libm::lgamma(a));
    if x < a + 1.0 {
        let mut ap = a;
        let mut del = 1.0 / a;
        let mut sum = del;
        for _ in 0..2000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-17 {
                break;
            }
        }
        sum * front
    } else {
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / TINY;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..2000 {
            let n = i as f64;
            let an = -n * (n - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < TINY {
                d = TINY;
            }
            c = b + an / c;
            if c.abs() < TINY {
                c = TINY;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-17 {
                break;
            }
        }
        1.0 - front * h
    }
}

/// The x a χ² distribution with `f` degrees of freedom stays below with
/// probability `p`.
pub fn chi2_quantile(f: f64, p: f64) -> f64 {
    let cdf = |x: f64| lower_gamma(f / 2.0, x / 2.0);
    let (mut lo, mut hi) = (0.0, 1.0);
    while cdf(hi) < p && hi < 1e9 {
        hi *= 2.0;
    }
    for _ in 0..400 {
        let mid = (lo + hi) / 2.0;
        if !(mid > lo && mid < hi) {
            break;
        }
        if cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

pub(crate) const OP: Op = op!("chi2Quantile", |f: f64, p: f64| chi2_quantile(f, p));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_textbook_quantiles() {
        for (f, x) in [
            (1.0, 3.841_458_820_694_124),
            (4.0, 9.487_729_036_781_154),
            (10.0, 18.307_038_053_275_146),
        ] {
            assert!((chi2_quantile(f, 0.95) - x).abs() < 1e-9 * x, "{f}");
        }
    }
}
