//! The elementary functions, correctly rounded (docs/adr/9403 §2): each gives the double nearest to the exact value
//! (a tie to the even one), so the same double on every platform and the one an independent reference gives.
//!
//! The arithmetic is `pxfm`'s, measured against a high-precision reference before it was taken (the ADR's tables)
//! and held to it since, bit for bit, by `fixtures/numeric/v1/elementary.json`. What IEEE 754 fixes without
//! arithmetic (a power's and an arc tangent's zeros, infinities and NaN) is decided here: the library's answers
//! parted from the standard's in those places.
//!
//! [`lgamma`] is the one function whose authors state a bound above half a last place (0.52); it gave the nearest
//! double on every case measured and on every case of the fixture.

/// The sine of an angle in radians.
pub fn sin(x: f64) -> f64 {
    pxfm::f_sin(x)
}

/// The cosine of an angle in radians.
pub fn cos(x: f64) -> f64 {
    pxfm::f_cos(x)
}

/// The sine and the cosine of an angle in radians: the two functions' own values.
pub fn sin_cos(x: f64) -> (f64, f64) {
    pxfm::f_sincos(x)
}

/// The tangent of an angle in radians.
pub fn tan(x: f64) -> f64 {
    pxfm::f_tan(x)
}

/// The arc sine, in [−π/2, π/2]; NaN outside [−1, 1].
pub fn asin(x: f64) -> f64 {
    pxfm::f_asin(x)
}

/// The arc cosine, in [0, π]; NaN outside [−1, 1].
pub fn acos(x: f64) -> f64 {
    pxfm::f_acos(x)
}

/// The arc tangent, in [−π/2, π/2].
pub fn atan(x: f64) -> f64 {
    pxfm::f_atan(x)
}

/// The angle of the point (x, y) from the x axis, in [−π, π]: the arc tangent of y/x in the point's quadrant.
pub fn atan2(y: f64, x: f64) -> f64 {
    let angle = pxfm::f_atan2(y, x);
    // A quotient below the smallest double is a zero of y's sign (the library gave +0 for a negative y).
    if angle == 0.0 {
        0.0_f64.copysign(y)
    } else {
        angle
    }
}

/// e to the power x.
pub fn exp(x: f64) -> f64 {
    pxfm::f_exp(x)
}

/// 2 to the power x.
pub fn exp2(x: f64) -> f64 {
    pxfm::f_exp2(x)
}

/// The natural logarithm: −∞ at a zero, NaN below it.
pub fn ln(x: f64) -> f64 {
    pxfm::f_log(x)
}

/// The logarithm to base 2.
pub fn log2(x: f64) -> f64 {
    pxfm::f_log2(x)
}

/// The logarithm to base 10.
pub fn log10(x: f64) -> f64 {
    pxfm::f_log10(x)
}

/// The cube root, of the argument's sign.
pub fn cbrt(x: f64) -> f64 {
    pxfm::f_cbrt(x)
}

/// x to the power y, with IEEE 754's special values (9.2.1): anything to the power zero is 1, and 1 to any power;
/// a zero's and an infinity's power takes the base's sign only for an odd whole exponent; a negative base has a
/// power only for a whole exponent.
pub fn pow(x: f64, y: f64) -> f64 {
    if y == 0.0 || x == 1.0 {
        return 1.0;
    }
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    let odd = odd_integer(y);
    if x == 0.0 {
        let size = if y < 0.0 { f64::INFINITY } else { 0.0 };
        return if odd { size.copysign(x) } else { size };
    }
    if y.is_infinite() {
        let size = x.abs();
        return if size == 1.0 {
            1.0
        } else if (size < 1.0) == (y < 0.0) {
            f64::INFINITY
        } else {
            0.0
        };
    }
    if x.is_infinite() {
        let size = if y < 0.0 { 0.0 } else { f64::INFINITY };
        return if x < 0.0 && odd { -size } else { size };
    }
    if x < 0.0 {
        if y != y.trunc() {
            return f64::NAN;
        }
        // The positive base's power with the sign of an odd exponent: rounding to nearest does not know the sign.
        let size = positive_power(-x, y);
        return if odd { -size } else { size };
    }
    positive_power(x, y)
}

/// x to the power y for a finite x > 0 and a finite y ≠ 0.
fn positive_power(x: f64, y: f64) -> f64 {
    exact_power(x, y).unwrap_or_else(|| pxfm::f_pow(x, y))
}

/// The power when it is a whole number times a power of two, rounded from that number itself.
///
/// Such a value can be exactly the middle of two doubles ((k²)^1.5 = k³, 3³⁴: an odd number of 54 bits), where
/// the library's rounding goes either way and the standard's to the even one. It happens only for y = n/2ᵏ with
/// k ≤ 5 and x = s^(2ᵏ)·2ᵉ, s odd: the power is sⁿ·2^(e·n/2ᵏ). Taken here while sⁿ fits 128 bits; a larger one is
/// no middle (it is odd and longer than 54 bits).
fn exact_power(x: f64, y: f64) -> Option<f64> {
    // y = n/2ᵏ in lowest terms, k ≤ 5; sⁿ < 2⁵⁴ with s ≥ 3 asks n ≤ 34.
    let t = y * 32.0;
    if !(t > 0.0 && t <= 1088.0 && t == t.trunc()) {
        return None;
    }
    let (mut n, mut k) = (t as u32, 5u32);
    while k > 0 && n.is_multiple_of(2) {
        n /= 2;
        k -= 1;
    }
    // x = m·2ᵉ, m odd.
    let bits = x.to_bits();
    let (field, fraction) = (((bits >> 52) & 0x7ff) as i32, bits & ((1u64 << 52) - 1));
    let (whole, power) = if field == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), field - 1075)
    };
    let zeros = whole.trailing_zeros();
    let (m, e) = (whole >> zeros, power + zeros as i32);
    if m == 1 {
        // A power of two: its power is one too, or irrational; the library's.
        return None;
    }
    // m = s^(2ᵏ) and 2ᵏ divides e.
    let mut s = m;
    for _ in 0..k {
        let root = (s as f64).sqrt() as u64;
        if root * root != s {
            return None;
        }
        s = root;
    }
    if e.rem_euclid(1 << k) != 0 {
        return None;
    }
    let number = u128::from(s).checked_pow(n)?;
    Some(rounded(number, (e >> k) * n as i32))
}

/// `p · 2ᵗ` as the nearest double, a tie to the even one; zero below the smallest double's half, infinite past the
/// largest's.
fn rounded(p: u128, t: i32) -> f64 {
    let length = 128 - p.leading_zeros() as i32;
    // The place of the last bit a double keeps: 53 bits from the top, or the smallest double's.
    let last = (length + t - 53).max(-1074);
    let drop = last - t;
    if drop <= 0 {
        return crate::hypot::scaled((p << -drop) as u64, last);
    }
    if drop > length {
        return 0.0;
    }
    let (kept, rest) = if drop == 128 {
        (0, p)
    } else {
        (p >> drop, p & ((1u128 << drop) - 1))
    };
    let half = 1u128 << (drop - 1);
    let up = rest > half || (rest == half && kept & 1 == 1);
    crate::hypot::scaled(kept as u64 + u64::from(up), last)
}

/// Whether a finite y is an odd whole number (from 2⁵³ on every double is even).
fn odd_integer(y: f64) -> bool {
    let half = y / 2.0;
    y.is_finite() && y == y.trunc() && half != half.trunc()
}

/// The hyperbolic sine.
pub fn sinh(x: f64) -> f64 {
    pxfm::f_sinh(x)
}

/// The hyperbolic cosine.
pub fn cosh(x: f64) -> f64 {
    pxfm::f_cosh(x)
}

/// The hyperbolic tangent.
pub fn tanh(x: f64) -> f64 {
    pxfm::f_tanh(x)
}

/// The inverse hyperbolic sine.
pub fn asinh(x: f64) -> f64 {
    pxfm::f_asinh(x)
}

/// The inverse hyperbolic tangent: ±∞ at ±1, NaN beyond.
pub fn atanh(x: f64) -> f64 {
    pxfm::f_atanh(x)
}

/// The complementary error function, 1 − erf(x).
pub fn erfc(x: f64) -> f64 {
    pxfm::f_erfc(x)
}

/// The natural logarithm of the gamma function's absolute value.
pub fn lgamma(x: f64) -> f64 {
    pxfm::f_lgamma(x)
}
