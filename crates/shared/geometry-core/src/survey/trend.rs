//! A monitored point's velocity through its epochs (plan/map-cad.md MAP-19, docs/adr/4840): a straight line fitted by
//! least squares to its movement against time, d(t) = a + v·t, Trimble 4D Control's velocity condition (the regression
//! line's slope) with its standard deviation and Student's test:
//!
//! - v = Σ(t − t̄)(d − d̄) / Σ(t − t̄)²; a = d̄ − v·t̄;
//! - the residuals r = d − (a + v·t), s² = Σr² / (n − 2), σᵥ = s / √Σ(t − t̄)²;
//! - the component moves at confidence c when |v| > t(n − 2; (1 + c)/2)·σᵥ (strictly; a perfect line, σᵥ = 0, moves
//!   when v is not 0).
//!
//! The epochs are unweighted: their own standard deviations are not known here. Three epochs at least (n − 2 ≥ 1).

use super::adjust::t_quantile;

/// A component's fitted velocity: its rate and standard deviation, in the units of d per unit of t.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity {
    pub rate: f64,
    pub sigma: f64,
}

/// The velocity of `d` against `t` (as long as each other), three points or more at two times or more; none otherwise
/// or when a number is not finite.
pub fn velocity(t: &[f64], d: &[f64]) -> Option<Velocity> {
    let n = t.len();
    if n < 3 || d.len() != n || t.iter().chain(d).any(|v| !v.is_finite()) {
        return None;
    }
    let count = n as f64;
    let tm = t.iter().sum::<f64>() / count;
    let dm = d.iter().sum::<f64>() / count;
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for (ti, di) in t.iter().zip(d) {
        sxx += (ti - tm) * (ti - tm);
        sxy += (ti - tm) * (di - dm);
    }
    if !(sxx > 0.0) {
        return None;
    }
    let rate = sxy / sxx;
    let a = dm - rate * tm;
    let mut ss = 0.0;
    for (ti, di) in t.iter().zip(d) {
        let r = di - (a + rate * ti);
        ss += r * r;
    }
    // IEEE square root: the same bits on every target.
    let sigma = (ss / (count - 2.0) / sxx).sqrt();
    Some(Velocity { rate, sigma })
}

/// Whether a velocity fitted to `n` points moves at `confidence` (between 0 and 1): |v| > t(n − 2; (1 + c)/2)·σᵥ.
pub fn moves(v: Velocity, n: usize, confidence: f64) -> bool {
    if n < 3 {
        return false;
    }
    let t = t_quantile((n - 2) as f64, (1.0 + confidence) / 2.0);
    v.rate.abs() > t * v.sigma
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_line_has_its_slope_and_no_spread() {
        let t = [0.0, 0.25, 0.5, 1.0];
        let d: Vec<f64> = t.iter().map(|x| 0.002 + 0.012 * x).collect();
        let v = velocity(&t, &d).expect("a fit");
        assert!((v.rate - 0.012).abs() < 1e-15, "{v:?}");
        assert!(v.sigma < 1e-15, "{v:?}");
        assert!(moves(v, 4, 0.95));
    }

    /// Three points, by hand: t = 0, 1, 2 and d = 0, 1, 3 give v = 1.5, a = −1/6, residuals 1/6, −1/3, 1/6, s² = 1/6,
    /// σᵥ = √(1/12); with 1 degree of freedom t(0.975) = tan(0.475π) = 12.706…: 1.5 < 12.706 × 0.2887, it does not move.
    #[test]
    fn three_points_by_hand() {
        let v = velocity(&[0.0, 1.0, 2.0], &[0.0, 1.0, 3.0]).expect("a fit");
        assert!((v.rate - 1.5).abs() < 1e-15);
        assert!((v.sigma - (1.0f64 / 12.0).sqrt()).abs() < 1e-15);
        assert!(!moves(v, 3, 0.95));
        // At 50 % (t(0.75; 1) = 1): 1.5 > 0.2887.
        assert!(moves(v, 3, 0.5));
    }

    #[test]
    fn too_few_or_one_time_is_no_fit() {
        assert!(velocity(&[0.0, 1.0], &[0.0, 1.0]).is_none());
        assert!(velocity(&[1.0, 1.0, 1.0], &[0.0, 1.0, 2.0]).is_none());
        assert!(velocity(&[0.0, 1.0, f64::NAN], &[0.0, 1.0, 2.0]).is_none());
    }
}
