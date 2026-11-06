//! A curve's superelevation transition proposed (docs/adr/0940, plan/road.md Y34; ROAD-DEVER-04, 12, ROAD-ALIGN-11):
//! how long the runoff must be for the traveled way's edges to rise no steeper than the largest relative gradient, the
//! runout that follows from it, how long a clothoid must be for the driver's comfort and for its shift to be seen, and
//! whether the transitions of two curves fit the tangent between them.
//!
//! - **Runoff by the relative gradient.** Every slope and width change of a curve's transition runs over the runoff L
//!   or over the runout `L·n/e`, both in proportion to L; the edges' relative gradients are therefore `G(L) = G(L₀)·L₀/L`
//!   and the shortest runoff within Δ is `L₀·G(L₀)/Δ` ([`runoff_for`]). `G(L₀)` is read on the schedule as it is built
//!   (`crate::superelevation_physics::relative_gradients`), so the axis of rotation and the widening are in it.
//! - **Runout.** From the normal crown to the level crown the outside turns by its normal slope n at the runoff's
//!   rate: `L·|n|/e` ([`runout`]).
//! - **Clothoid by comfort (Shortt).** Driving the clothoid at v (m/s) the lateral acceleration not balanced by the
//!   superelevation, `v²/R − g·e`, grows from 0 over the time `L/v`; at a rate C (m/s³): `L = v·(v²/R − g·e)/C`
//!   ([`comfort_spiral`]). With V in km/h, `v³ = V³/46.656` and `g·v = 2.724·e·V`: AASHTO's metric
//!   `0.0214·(V³/R − 127·e·V)/C`. Without e it is Shortt's 1909 railway rule `V³/(46.656·R·C)`.
//! - **Clothoid by its shift.** A clothoid of length L on radius R shifts the circle inward by `p ≈ L²/(24R)`; a shift
//!   to be seen asks `L ≥ √(24·p·R)` and one not to waste the lane `L ≤ √(24·p_max·R)` ([`shift_spiral`]).
//! - **The tangent between curves.** Before a curve the transition takes its runout and the runoff's share on the
//!   tangent; after it the same. Two curves need the sum of the first's exit and the second's entry between ST and
//!   the next TS ([`tangent_need`]); a schedule turns each curve back to the normal crown, so a shorter tangent does
//!   not build.
//!
//! The largest relative gradient, the comfort rate C, the shifts and the share on the tangent are the standard's
//! (AASHTO: Δ 1:128 at 25 km/h to 1:286 at 130 km/h, C from 1 to 3 m/s³ on highways, the share 0.6–0.8); KentOS
//! gives none of them.

use crate::superelevation_physics::G;

/// The shortest runoff whose relative gradient is at most `most` (per one): `runoff·gradient/most`, where `gradient`
/// is the largest relative gradient of the transition built with `runoff`.
pub fn runoff_for(runoff: f64, gradient: f64, most: f64) -> f64 {
    runoff * gradient / most
}

/// The runout of a runoff `runoff` turning the outside from its normal slope `normal` to the rate `rate` (grades).
pub fn runout(runoff: f64, normal: f64, rate: f64) -> f64 {
    runoff * normal.abs() / rate
}

/// The clothoid a driver at `speed` (km/h) takes comfortably into radius `radius` (m) with the superelevation `e`
/// (per one) at the rate `comfort` (m/s³): `v·(v²/R − g·e)/C`, none below 0 (the superelevation balances it all).
pub fn comfort_spiral(speed: f64, radius: f64, e: f64, comfort: f64) -> f64 {
    let v = speed / 3.6;
    (v * (v * v / radius - G * e) / comfort).max(0.0)
}

/// The clothoid whose shift on radius `radius` is `shift` (m): `√(24·p·R)`.
pub fn shift_spiral(shift: f64, radius: f64) -> f64 {
    (24.0 * shift * radius).sqrt()
}

/// `v` rounded up to the next multiple of `step` (none: as it is).
pub fn round_up(v: f64, step: Option<f64>) -> f64 {
    match step {
        Some(s) if s > 0.0 => (v / s - 1e-9).ceil() * s,
        _ => v,
    }
}

/// What a curve's transition takes on its tangents: its runout and the runoff's share before the curve.
pub fn tangent_need(runoff: f64, on_tangent: f64, normal: f64, rate: f64) -> f64 {
    runout(runoff, normal, rate) + on_tangent * runoff
}

/// The length of a curve held at full superelevation: from the end of the entry's runoff to the start of the exit's,
/// `(ST − TS) − 2·(1 − share)·runoff`; below 0 the transitions overlap on the curve.
pub fn full_length(ts: f64, st: f64, runoff: f64, on_tangent: f64) -> f64 {
    (st - ts) - 2.0 * (1.0 - on_tangent) * runoff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortt_without_superelevation_is_the_railway_rule() {
        // V³/(46.656·R·C): 80 km/h on R 250 at C 0.6.
        let l = comfort_spiral(80.0, 250.0, 0.0, 0.6);
        assert!((l - 80f64.powi(3) / (46.656 * 250.0 * 0.6)).abs() < 1e-9, "{l}");
        // With e the unbalanced part only; full balance asks none.
        let v = 80.0 / 3.6;
        assert_eq!(comfort_spiral(80.0, 250.0, v * v / (250.0 * G), 0.6), 0.0);
    }

    #[test]
    fn the_shift_rule_inverts_p_l2_over_24r() {
        let l = shift_spiral(0.2, 300.0);
        assert!((l * l / (24.0 * 300.0) - 0.2).abs() < 1e-12);
    }
}
