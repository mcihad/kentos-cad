//! Ağ dengelemesi ve kot ağı (docs/adr/0203): least squares adjustment of
//! a horizontal network ([`horizontal`]: directions and distances) and of a
//! levelling network ([`levelling`]: height differences), with what a
//! surveyor reads of them: the residuals, m0, the points' standard
//! deviations and error ellipses, the redundancy numbers, Baarda's test of
//! each observation and the model's χ² test. What both share is here: the
//! a priori standard deviations, an observation as the normal equations
//! take it, the statistics. The independent reference is
//! `scripts/fixtures/network_adjust_cases.py` (mpmath, 50 digits).

pub mod chi2;
pub mod horizontal;
pub mod levelling;
mod linalg;

use crate::api::Op;
use crate::text::edit::fold;

pub use chi2::chi2_quantile;
pub use linalg::Normal;

/// The a priori standard deviations (docs/adr/0203 §1), as the window
/// resolves them (the project's, else the defaults): a direction's and a
/// zenith angle's (radians), a distance's constant part (m) and its part
/// per million, each end's centering (m), geometric levelling's per √km (m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sigmas {
    pub direction: f64,
    pub distance: f64,
    pub ppm: f64,
    pub centering: f64,
    pub zenith: f64,
    pub levelling: f64,
}

crate::json_struct!(Sigmas {
    direction,
    distance,
    ppm,
    centering,
    zenith,
    levelling
});

impl Sigmas {
    /// A distance's standard deviation at `length` m: its constant part and
    /// the part per million added, then the two ends' centering.
    pub fn distance_at(&self, length: f64) -> f64 {
        let plain = self.distance + self.ppm * 1e-6 * length;
        (plain * plain + 2.0 * self.centering * self.centering).sqrt()
    }
}

/// A Baarda test value above this flags an observation (α₀ = 0.001, two-sided).
pub const W_LIMIT: f64 = 3.29;
/// A redundancy number below this: the observation is not checked.
pub const UNCONTROLLED: f64 = 1e-4;
/// Gauss–Newton's most iterations.
pub const MAX_ITERATIONS: usize = 30;
/// The most unknowns a network may have (the normal equations are dense).
pub const MAX_UNKNOWNS: usize = 1500;

/// A point's name as names are compared (docs/adr/0203 §2): trimmed, its
/// letters folded the Turkish way.
pub fn name_key(name: &str) -> String {
    name.trim().chars().map(fold).collect()
}

/// An observation as the normal equations take it: its kind, the row (or
/// known point) it came from, its design row (unknown, coefficient), its
/// misclosure (observed − computed) and its standard deviation.
#[derive(Clone, Debug)]
pub(crate) struct Observed {
    pub kind: &'static str,
    pub row: usize,
    pub coef: Vec<(usize, f64)>,
    pub l: f64,
    pub sigma: f64,
}

/// An observation adjusted: its kind (`direction`, `distance`, `y`, `x`,
/// `dh`, `h`), its row, the residual (computed − observed: radians or m),
/// its a priori standard deviation, its redundancy number, Baarda's test
/// value (none: not checked) and what that says (`ok`, `blunder`,
/// `uncontrolled`).
#[derive(Clone, Debug, PartialEq)]
pub struct ObservationResult {
    pub kind: &'static str,
    pub row: usize,
    pub v: f64,
    pub sigma: f64,
    pub r: f64,
    pub w: Option<f64>,
    pub flag: &'static str,
}

crate::json_struct!(out ObservationResult {
    kind,
    row,
    v,
    sigma,
    r,
    w,
    flag
});

/// What the adjustment says of the network as a whole: the observations,
/// the flagged one with the greatest test value, the counts (observations,
/// unknowns, degrees of freedom), vᵀPv, m0 (none without degrees of
/// freedom), the χ² quantile and whether the model test passed.
#[derive(Clone, Debug, PartialEq)]
pub struct Statistics {
    pub observations: Vec<ObservationResult>,
    pub worst: Option<usize>,
    pub n: usize,
    pub u: usize,
    pub f: usize,
    pub omega: f64,
    pub m0: Option<f64>,
    pub chi2: Option<f64>,
    pub passed: Option<bool>,
}

impl Statistics {
    /// The standard deviation of unit weight the accuracies are scaled by:
    /// m0, or 1 without degrees of freedom.
    pub fn sigma0(&self) -> f64 {
        self.m0.unwrap_or(1.0)
    }
}

/// The statistics of `obs` at the solution, `q` the inverse of the normal
/// matrix there (docs/adr/0203 §4).
pub(crate) fn statistics(obs: &[Observed], q: &Normal) -> Statistics {
    let n = obs.len();
    let u = q.size();
    let f = n.saturating_sub(u);
    let mut omega = 0.0;
    let mut rows = Vec::with_capacity(n);
    for o in obs {
        let v = -o.l;
        let p = 1.0 / (o.sigma * o.sigma);
        omega += p * v * v;
        let mut aqa = 0.0;
        for &(i, ai) in &o.coef {
            for &(j, aj) in &o.coef {
                aqa += ai * q.get(i, j) * aj;
            }
        }
        let qvv = 1.0 / p - aqa;
        let r = p * qvv;
        let (w, flag) = if !(r >= UNCONTROLLED) {
            (None, "uncontrolled")
        } else {
            let w = v.abs() / qvv.sqrt();
            (Some(w), if w > W_LIMIT { "blunder" } else { "ok" })
        };
        rows.push(ObservationResult {
            kind: o.kind,
            row: o.row,
            v,
            sigma: o.sigma,
            r,
            w,
            flag,
        });
    }
    let mut worst: Option<usize> = None;
    for (i, o) in rows.iter().enumerate() {
        if o.flag == "blunder" && worst.is_none_or(|k| o.w > rows[k].w) {
            worst = Some(i);
        }
    }
    let m0 = (f > 0).then(|| (omega / f as f64).sqrt());
    let chi2 = (f > 0).then(|| chi2_quantile(f as f64, 0.95));
    Statistics {
        observations: rows,
        worst,
        n,
        u,
        f,
        omega,
        m0,
        chi2,
        passed: chi2.map(|c| omega <= c),
    }
}

pub(crate) static OPS: &[Op] = &[horizontal::OP, levelling::OP, chi2::OP];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_fold_the_turkish_way() {
        assert_eq!(name_key(" Pİ1 "), name_key("pi1"));
        assert_eq!(name_key("PI"), name_key("pı"));
        assert_ne!(name_key("PI"), name_key("pi"));
    }

    #[test]
    fn a_distance_sigma_adds_its_parts() {
        let s = Sigmas {
            direction: 1e-5,
            distance: 0.002,
            ppm: 2.0,
            centering: 0.0,
            zenith: 1e-5,
            levelling: 0.002,
        };
        assert!((s.distance_at(1000.0) - 0.004).abs() < 1e-15);
    }
}
