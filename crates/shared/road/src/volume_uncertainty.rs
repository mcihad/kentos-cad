//! An earthwork volume's uncertainty budget (docs/adr/0941, plan/road.md Y35; ROAD-VOL-14): what the survey's errors,
//! the method and the arithmetic leave in a cut and a fill.
//!
//! - **Random survey error.** Each ground point's height errs by an independent εⱼ of standard deviation σⱼ. The volume
//!   is linear in the heights to first order (`crate::volume::sensitivity` gives `wⱼ = ∂V/∂zⱼ`), so
//!   `σ_V = √(Σ wⱼ²·σⱼ²)` ([`spread`]): the propagation of variances. Points far apart average out; a point that
//!   carries a large area (a sparse survey under a wide cut) weighs in. At 95 % a normal error stays within 1.96 σ.
//! - **Systematic error.** A shift β of every height (a datum, a GNSS antenna height) moves the volume by `β·Σwⱼ`,
//!   which is β times the plan area (Σφ = 1): `β·A` ([`bias`]). It does not average out and is added whole.
//! - **The method.** The average end area and the prismoidal formula differ by what the sections between them leave
//!   out; their difference is reported as the method's share (an indicator, not a bound).
//! - **The arithmetic.** A sum of n terms in binary64 errs by at most `n·u·Σ|vᵢ|` (u = 2⁻⁵³, Higham's bound for
//!   recursive summation to first order) ([`numeric_bound`]): far under a litre for any road.
//!
//! The bound at 95 % is `|bias| + 1.96·σ` ([`bound95`]); a volume is accepted when that is within the share of the
//! volume the contract allows. The survey's σ and β and the share are the project's; KentOS gives none.

/// The two-sided 95 % quantile of the normal distribution.
pub const Z95: f64 = 1.959_963_984_540_054;

/// The standard deviation of a volume whose sensitivities are `weights` (m³ per m) to heights erring independently by
/// `sigma` (m).
pub fn spread(weights: &[f64], sigma: f64) -> f64 {
    sigma * weights.iter().map(|w| w * w).sum::<f64>().sqrt()
}

/// What a shift of every height by `beta` (m) does to a volume of sensitivities `weights`: `β·Σwⱼ`.
pub fn bias(weights: &[f64], beta: f64) -> f64 {
    beta * weights.iter().sum::<f64>()
}

/// The bound at 95 %: the systematic part whole, the random part at 1.96 σ.
pub fn bound95(sigma: f64, bias: f64) -> f64 {
    bias.abs() + Z95 * sigma
}

/// The largest rounding error of summing `count` terms whose sizes add up to `sum_abs` in binary64.
pub fn numeric_bound(count: usize, sum_abs: f64) -> f64 {
    count as f64 * (f64::EPSILON / 2.0) * sum_abs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_errors_add_in_quadrature_and_a_shift_by_the_area() {
        assert!((spread(&[3.0, 4.0], 0.1) - 0.5).abs() < 1e-12);
        assert!((bias(&[3.0, 4.0], 0.02) - 0.14).abs() < 1e-12);
        assert!((bound95(1.0, -0.5) - (0.5 + Z95)).abs() < 1e-12);
    }
}
