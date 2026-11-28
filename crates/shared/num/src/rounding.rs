//! Rounding to a whole number by a rule that is written down (docs/adr/9403 §1).

/// The nearest whole number, a half toward +∞ (2.5 → 3, −2.5 → −2): a lattice's rule. Pixels, grid cells and steps
/// are counted from an origin, and by this rule a value moved by a whole number rounds to its rounding moved by that
/// number, on either side of zero. A negative value that rounds to zero keeps its sign (−0.4 → −0).
///
/// A value shown or kept as a decimal is rounded with a half away from zero instead (`f64::round`, the display
/// rule's): the two differ at the negative halves only.
pub fn round_half_up(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let floor = x.floor();
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded == 0.0 && x.is_sign_negative() {
        -0.0
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_half_goes_up_on_either_side_of_zero() {
        for (x, want) in [
            (2.5, 3.0),
            (-2.5, -2.0),
            (0.49999999999999994, 0.0),
            (-0.5, -0.0),
            (1.4999999999999998, 1.0),
            (4503599627370495.5, 4503599627370496.0),
            (-7.6, -8.0),
            (9007199254740993.0, 9007199254740992.0),
        ] {
            let got = round_half_up(x);
            assert_eq!(
                got.to_bits(),
                f64::to_bits(want),
                "round_half_up({x}) = {got}"
            );
        }
        assert!(round_half_up(-0.4).is_sign_negative());
        assert!(round_half_up(f64::NAN).is_nan());
        assert_eq!(round_half_up(f64::INFINITY), f64::INFINITY);
        // Moved by a whole number, a value rounds to its rounding moved by it.
        for x in [-2.5, -1.5, -0.5, 0.5, 1.5, 2.5, -3.25, 0.75] {
            assert_eq!(round_half_up(x + 7.0), round_half_up(x) + 7.0, "{x}");
        }
    }
}
