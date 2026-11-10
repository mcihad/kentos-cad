//! Dayanıklı (robust) kestirim (plan/map-cad.md MAP-05, docs/adr/4960): iteratively reweighted least squares. Each
//! round the network is adjusted with the observations' weights multiplied by a factor, each observation's
//! standardised residual w = |v| / √q_vv (a priori σ0 = 1) is computed, and the next factor is a weight function of it,
//! taken against the original weight (not compounded). The rounds stop when no factor moves by more than
//! [`TOLERANCE`]; the solution reported is the one the reported factors gave. Nothing is removed: a factor below 1
//! marks an observation the regulation wants checked and measured again, the user decides.
//!
//! The weight functions, |w| ≤ c gives 1:
//! - Huber (1964): c / |w|;
//! - Danish (Krarup, Juhl and Kubik 1980), in the form exp(−((|w| − c) / c)²).

use crate::jsmath::exp;

/// The weight function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Huber,
    Danish,
}

impl Method {
    /// The name the commands use.
    pub fn name(self) -> &'static str {
        match self {
            Method::Huber => "huber",
            Method::Danish => "danish",
        }
    }

    /// The constant c when none is given (docs/adr/4960, Dayanak): Huber 1.5, Danish 2.0.
    pub fn default_c(self) -> f64 {
        match self {
            Method::Huber => 1.5,
            Method::Danish => 2.0,
        }
    }
}

/// A robust estimation: its weight function and constant c (> 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Robust {
    pub method: Method,
    pub c: f64,
}

/// The rounds stop when no factor moves by more than this.
pub const TOLERANCE: f64 = 1e-9;
/// The most rounds; past them the answer says the factors did not settle.
pub const MAX_ROUNDS: usize = 100;

impl Robust {
    /// The factor an observation's weight is multiplied by for standardised residual `w` (none: not checked, 1).
    pub fn factor(&self, w: Option<f64>) -> f64 {
        let Some(w) = w.filter(|w| w.is_finite()) else {
            return 1.0;
        };
        let a = w.abs();
        if a <= self.c {
            return 1.0;
        }
        match self.method {
            Method::Huber => self.c / a,
            Method::Danish => {
                let t = (a - self.c) / self.c;
                exp(-(t * t))
            }
        }
    }
}

/// How the rounds went: how many, whether the factors settled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rounds {
    pub robust: Robust,
    pub rounds: usize,
    pub converged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huber_and_danish_keep_small_residuals() {
        let h = Robust { method: Method::Huber, c: 1.5 };
        assert_eq!(h.factor(Some(1.5)), 1.0);
        assert_eq!(h.factor(Some(-3.0)), 0.5);
        assert_eq!(h.factor(None), 1.0);
        let d = Robust { method: Method::Danish, c: 2.0 };
        assert_eq!(d.factor(Some(2.0)), 1.0);
        assert!((d.factor(Some(4.0)) - (-1.0f64).exp()).abs() < 1e-15);
    }
}
