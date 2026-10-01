//! How a number is written for the user (docs/adr/0149, display v1): on
//! screen, in a dimension's value, in a label written into the drawing. The
//! value is first rounded to seven decimals (from the float's exact binary
//! value, an exact half away from zero), then that decimal to the digits
//! shown, a half away from zero, as on paper. Seven decimals (0.1 µm) are a
//! hundred times the noise of a measure taken from Transverse Mercator
//! coordinates (about 10⁻⁹ m) and ten thousand times finer than a
//! millimetre: a value that is a half at the digits shown (12.125, a typed
//! 5.0005) is written the same way whichever way it was computed. With seven
//! digits or more the value is rounded once, to them. A value that rounds to
//! zero is written without a minus sign.
//!
//! The web's twin is `apps/web/src/core/displayNumber.ts`; both pass
//! `fixtures/numeric/v1/display.json`, which `scripts/fixtures/numeric_display.py`
//! writes from the rule alone. Display only: a written value is never read
//! back into a computation (CLAUDE.md §23.2).

use crate::api::Op;
use crate::op;

/// The decimals the noise is rounded away at.
pub const NOISE_DECIMALS: usize = 7;

/// `v` written with `d` decimals by the display rule.
pub fn fixed(v: f64, d: usize) -> String {
    if v.is_nan() {
        return "NaN".to_owned();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let x = v.abs();
    let body = if d >= NOISE_DECIMALS {
        exact_fixed(x, d)
    } else {
        round_text(&exact_fixed(x, NOISE_DECIMALS), d)
    };
    if v < 0.0 && body.bytes().any(|b| (b'1'..=b'9').contains(&b)) {
        format!("-{body}")
    } else {
        body
    }
}

/// `x` (finite, not negative) with `d` decimals from its exact binary value,
/// an exact half away from zero (JavaScript's `toFixed`; Rust's own
/// formatting rounds an exact half to even).
fn exact_fixed(x: f64, d: usize) -> String {
    if exact_half(x, d) {
        // The digits up to the half are exact; drop the 5 and add one in the last place.
        let long = format!("{x:.prec$}", prec = d + 1);
        up_one(long[..long.len() - 1].trim_end_matches('.'))
    } else {
        format!("{x:.d$}")
    }
}

/// A decimal text with more than `d` decimals rounded to `d`, a half away
/// from zero: up when the first dropped digit is 5 or more.
fn round_text(text: &str, d: usize) -> String {
    let Some(dot) = text.find('.') else {
        return text.to_owned();
    };
    let keep = if d == 0 { dot } else { dot + 1 + d };
    if keep >= text.len() {
        return text.to_owned();
    }
    let first_dropped = text.as_bytes()[if d == 0 { dot + 1 } else { keep }];
    let kept = &text[..keep];
    if first_dropped >= b'5' {
        up_one(kept)
    } else {
        kept.to_owned()
    }
}

/// Whether `x` (finite, not negative) lies exactly halfway between two
/// multiples of 10^-d: `x × 10^d` has a fractional part of exactly one half.
fn exact_half(x: f64, d: usize) -> bool {
    if x == 0.0 || d > 12 {
        return false;
    }
    let bits = x.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i64;
    let fraction = bits & ((1 << 52) - 1);
    // x = mantissa × 2^power, exactly.
    let (mantissa, power) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), exponent - 1075)
    };
    if power >= 0 {
        return false;
    }
    // mantissa < 2^53 and 10^12 < 2^40: the product fits in 93 bits, so a
    // half (2^(k-1)) is out of reach once k exceeds 93.
    let k = -power;
    if k > 93 {
        return false;
    }
    let scaled = u128::from(mantissa) * 10u128.pow(d as u32);
    scaled % (1u128 << k) == 1u128 << (k - 1)
}

/// A decimal text plus one unit in its last place: `9.99` → `10.00`.
fn up_one(digits: &str) -> String {
    let mut chars: Vec<char> = digits.chars().collect();
    let mut i = chars.len();
    loop {
        if i == 0 {
            chars.insert(0, '1');
            break;
        }
        i -= 1;
        match chars[i] {
            '.' => {}
            '9' => chars[i] = '0',
            c => {
                chars[i] = char::from(c as u8 + 1);
                break;
            }
        }
    }
    chars.into_iter().collect()
}

pub static OPS: &[Op] = &[op!("displayFixed", |v: f64, d: f64| {
    fixed(
        v,
        if d.is_finite() && d > 0.0 {
            d as usize
        } else {
            0
        },
    )
})];

#[cfg(test)]
mod tests {
    use super::fixed;

    #[test]
    fn a_half_at_the_digits_shown_rounds_up_however_it_was_computed() {
        // 12.125 exactly, and a hair either side of it (the noise of a measure at TM coordinates).
        for v in [12.125, 12.124_999_999_7, 12.125_000_000_3] {
            assert_eq!(fixed(v, 2), "12.13", "{v}");
        }
        // A typed 5.0005 is 5.000499999… in binary: written as typed, rounded as on paper.
        assert_eq!(fixed(5.0005, 3), "5.001");
        assert_eq!(fixed(7.8465, 3), "7.847");
        assert_eq!(fixed(2.5, 0), "3");
        assert_eq!(fixed(-2.5, 0), "-3");
        assert_eq!(fixed(487_012.062_5, 3), "487012.063");
        assert_eq!(fixed(4_420_000.000_5, 3), "4420000.001");
    }

    #[test]
    fn values_away_from_a_half_round_to_the_nearest() {
        assert_eq!(fixed(7.846_512_3, 3), "7.847");
        assert_eq!(fixed(15.693_024_6, 3), "15.693");
        assert_eq!(fixed(12.124_9, 2), "12.12");
        assert_eq!(fixed(9.999_5, 3), "10.000");
        assert_eq!(fixed(0.0, 3), "0.000");
        assert_eq!(fixed(12.0, 0), "12");
    }

    #[test]
    fn zero_has_no_sign_and_fine_digits_round_once() {
        assert_eq!(fixed(-0.0001, 3), "0.000");
        assert_eq!(fixed(-0.0, 2), "0.00");
        assert_eq!(fixed(-0.0006, 3), "-0.001");
        // Seven digits or more: once, from the exact value.
        // 1/256 is an exact half at the seventh decimal.
        assert_eq!(fixed(0.003_906_25, 7), "0.0039063");
        assert_eq!(fixed(1.0, 9), "1.000000000");
        assert_eq!(fixed(f64::NAN, 2), "NaN");
        assert_eq!(fixed(f64::NEG_INFINITY, 2), "-Infinity");
    }
}
