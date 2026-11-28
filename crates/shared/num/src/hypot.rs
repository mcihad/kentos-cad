//! The length of a vector, correctly rounded (docs/adr/9403 §2). It is the base's own code: no library measured rounds
//! it correctly (the platform's, `libm`'s and a third all miss the nearest double for about one input in seven), and a
//! length is the most computed value of a drawing.

/// √(x² + y²), correctly rounded: the double nearest to the exact value, a tie to the even one (IEEE 754
/// `roundTiesToEven`). An infinity wins over a NaN, as IEEE 754 says of `hypot`.
///
/// Two paths give the one answer (docs/adr/9403 §2):
///
/// - the quick one ([`quick`]) works in doubles with error-free sums and products, bounds its own error and answers
///   when that bound cannot change the rounding;
/// - where it can (a result within 2⁻⁴⁴ of an ulp of the middle of two doubles: an exact tie, or the rare near one),
///   and for subnormal inputs, the squares are summed as integers and the root is decided exactly ([`exact`]).
///
/// The quick path answers only what it has proved, so the result does not depend on which path gave it.
pub fn hypot(x: f64, y: f64) -> f64 {
    let (x, y) = (x.abs(), y.abs());
    let (ex, ey) = (biased_exponent(x), biased_exponent(y));
    if ex.min(ey) > USUAL_LOW && ex.max(ey) < USUAL_HIGH {
        // The usual case, a drawing's and a map's sizes: nothing is scaled.
        let (a, b) = if x >= y { (x, y) } else { (y, x) };
        if (ex - ey).abs() > APART {
            return a;
        }
        return quick(a, b).unwrap_or_else(|| exact(a, b));
    }
    if x.is_infinite() || y.is_infinite() {
        return f64::INFINITY;
    }
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    let (a, b) = if x >= y { (x, y) } else { (y, x) };
    if b == 0.0 {
        return a;
    }
    let (ea, eb) = (biased_exponent(a), biased_exponent(b));
    if ea == 0 || eb == 0 {
        // A subnormal among them: the integers take every case.
        return exact(a, b);
    }
    if ea - eb > APART {
        return a;
    }
    // Both scaled by the power of two that brings a into [1, 2): exact, and b stays above 2⁻²⁸. Scaled back, the
    // answer is exact too, or past the largest double and so infinite, as the exact value's rounding is.
    let up = power_of_two(1023 - ea);
    match quick(a * up, b * up) {
        Some(root) => root * power_of_two(ea - 1023),
        None => exact(a, b),
    }
}

/// The quick path's range of exponent fields, ends excluded: 2⁻⁴⁷⁹ ≤ v < 2⁵⁰⁰. No square overflows there, and the
/// smallest product the error-free product forms (the square of the last place of 2⁻⁴⁷⁹, 2⁻¹⁰⁶²) is still a double.
const USUAL_LOW: i32 = 1023 - 480;
const USUAL_HIGH: i32 = 1023 + 500;

/// Exponent fields further apart than this leave the larger side as the answer: then b < 2⁻²⁸·a, and the root exceeds
/// a by less than b²/2a < 2⁻⁵⁷·a, under a sixteenth of a's last place.
const APART: i32 = 28;

/// The root for 2⁻⁴⁷⁹ ≤ b ≤ a < 2⁵⁰⁰ with b > 2⁻²⁹·a, when the doubles suffice to prove it; `None` when they do not.
///
/// With s = a² + b² held as `sh + sl` (exact but for `sl`'s two roundings) and r the rounded root of `sh`, the root of
/// s is r + δ, δ = (s − r²)/(√s + r); what is computed is (s − r²)/2r. Taking a in [1, 2), so that r ≥ 1 (every term
/// scales with a's power of two):
///
/// - s − r² comes out within 2⁻¹⁰¹ (the roundings of `sl` and of the last sum; `sh − rh` is exact, the two being within
///   a factor of two of each other), which the division by 2r ≥ 2 halves;
/// - the quotient's own rounding is at most 2⁻¹⁰⁴;
/// - 2r in place of √s + r changes it by δ²/2r, under 2⁻¹⁰³, δ being under 2⁻⁵⁰.
///
/// Less than 2⁻¹⁰¹ in all. The bound asked for is sixteen times that, 2⁻⁹⁷·r: when r plus the correction rounds to the
/// same double at both ends of the bound, that double is the nearest to the exact root.
fn quick(a: f64, b: f64) -> Option<f64> {
    let (xh, xl) = two_product(a, a);
    let (yh, yl) = two_product(b, b);
    let (sh, t) = two_sum(xh, yh);
    let sl = t + (xl + yl);
    let r = sh.sqrt();
    let (rh, rl) = two_product(r, r);
    let correction = (((sh - rh) - rl) + sl) / (2.0 * r);
    /// 2⁻⁹⁷.
    const BOUND: f64 = 1.0 / ((1u128 << 97) as f64);
    let bound = r * BOUND;
    let (low, high) = (r + (correction - bound), r + (correction + bound));
    (low == high).then_some(low)
}

/// The exponent field of a double: 0 for a subnormal (and zero), 1 to 2046 for a normal number, 2047 for an infinity
/// and a NaN.
fn biased_exponent(v: f64) -> i32 {
    ((v.to_bits() >> 52) & 0x7ff) as i32
}

/// 2ᵏ for −1074 ≤ k ≤ 1023, exactly.
fn power_of_two(k: i32) -> f64 {
    if k >= -1022 {
        f64::from_bits(((k + 1023) as u64) << 52)
    } else {
        f64::from_bits(1u64 << (k + 1074))
    }
}

/// a + b and the error of its rounding, exactly (Knuth's sum of two; nothing is asked of their sizes).
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let shadow = s - a;
    (s, (a - (s - shadow)) + (b - shadow))
}

/// a · b and the error of its rounding, exactly, for values whose product neither overflows nor reaches the
/// subnormals with its last place.
///
/// The error is a double, so both ways of computing it give the same one: a processor with a fused multiply-add
/// gets it in one operation, another by splitting the factors. Which is used changes the speed, never a bit.
fn two_product(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let error = if cfg!(any(target_arch = "aarch64", target_feature = "fma")) {
        fused_error(a, b, p)
    } else {
        split_error(a, b, p)
    };
    (p, error)
}

/// a · b − p in one fused operation: exact, because the difference is a double.
// The one place a fused multiply-add is right: here it rounds nothing (clippy.toml forbids it elsewhere).
#[allow(clippy::disallowed_methods)]
fn fused_error(a: f64, b: f64, p: f64) -> f64 {
    a.mul_add(b, -p)
}

/// a · b − p by Dekker's product over Veltkamp's splitting: each factor as a high and a low half of 26 bits, whose
/// four products are exact.
fn split_error(a: f64, b: f64, p: f64) -> f64 {
    /// 2²⁷ + 1.
    const SPLIT: f64 = 134_217_729.0;
    let (ca, cb) = (SPLIT * a, SPLIT * b);
    let (ah, bh) = (ca - (ca - a), cb - (cb - b));
    let (al, bl) = (a - ah, b - bh);
    ((ah * bh - p) + ah * bl + al * bh) + al * bl
}

/// A finite positive double as an integer of exactly 53 bits and a power of two: `m · 2ᵉ`, 2⁵² ≤ m < 2⁵³ (a
/// subnormal's bits are moved up and its power down).
fn normalised(v: f64) -> (u64, i32) {
    let bits = v.to_bits();
    let field = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    if field == 0 {
        let up = fraction.leading_zeros() as i32 - 11;
        (fraction << up, -1074 - up)
    } else {
        (fraction | (1u64 << 52), field - 1075)
    }
}

/// An integer of up to 256 bits as two halves; only what the exact root needs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Wide {
    high: u128,
    low: u128,
}

impl Wide {
    /// v², for any v.
    fn square(v: u128) -> Wide {
        const HALF: u128 = (1u128 << 64) - 1;
        let (h, l) = (v >> 64, v & HALF);
        let (ll, hl, hh) = (l * l, h * l, h * h);
        // v² = hh·2¹²⁸ + 2·hl·2⁶⁴ + ll.
        let (cross_low, cross_high) = (hl << 65, hl >> 63);
        let (low, carry) = ll.overflowing_add(cross_low);
        Wide {
            high: hh + cross_high + u128::from(carry),
            low,
        }
    }

    fn plus(self, other: Wide) -> Wide {
        let (low, carry) = self.low.overflowing_add(other.low);
        Wide {
            high: self.high + other.high + u128::from(carry),
            low,
        }
    }
}

/// The correctly rounded root by integers alone, for finite a ≥ b > 0.
fn exact(a: f64, b: f64) -> f64 {
    let ((ma, ea), (mb, eb)) = (normalised(a), normalised(b));
    let apart = ea - eb;
    if apart > APART {
        return a;
    }
    // a = A · 2^eb and b = B · 2^eb, A < 2⁸¹: the squares' sum is below 2¹⁶³ and its root below 2⁸².
    let n = Wide::square(u128::from(ma) << apart).plus(Wide::square(u128::from(mb)));
    let mut root: u128 = 0;
    for bit in (0..82).rev() {
        let with = root | (1u128 << bit);
        if Wide::square(with) <= n {
            root = with;
        }
    }
    // The root's bits past the 53rd, or past the smallest double's place, are rounded away.
    let length = 128 - root.leading_zeros() as i32;
    let drop = (length - 53).max(-1074 - eb).max(0);
    let mut kept = (root >> drop) as u64;
    let up = if drop == 0 {
        // √n ≥ kept + ½ exactly when 4n ≥ (2·kept + 1)²: an odd square is never 4n, so no tie.
        let four = Wide {
            high: (n.high << 2) | (n.low >> 126),
            low: n.low << 2,
        };
        four >= Wide::square(2 * u128::from(kept) + 1)
    } else {
        let middle = Wide::square((2 * u128::from(kept) + 1) << (drop - 1));
        n > middle || (n == middle && kept & 1 == 1)
    };
    if up {
        kept += 1;
    }
    scaled(kept, eb + drop)
}

/// `m · 2ᵉ` as a double for m ≤ 2⁵³ whose value is a double or past the largest one (then infinite).
pub(crate) fn scaled(m: u64, e: i32) -> f64 {
    if m == 0 {
        return 0.0;
    }
    let top = 63 - m.leading_zeros() as i32;
    let exponent = top + e;
    if exponent > 1023 {
        return f64::INFINITY;
    }
    if exponent >= -1022 {
        // The leading bit to place 52: m has at most 54 bits and, with 54, is 2⁵³.
        let mantissa = if top <= 52 {
            m << (52 - top)
        } else {
            m >> (top - 52)
        };
        return f64::from_bits(
            (((exponent + 1023) as u64) << 52) | (mantissa & ((1u64 << 52) - 1)),
        );
    }
    f64::from_bits(m << (e + 1074))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A value in [0, 1) from the sequence.
    fn unit(next: &mut impl FnMut() -> u64) -> f64 {
        (next() >> 11) as f64 / (1u64 << 53) as f64
    }

    #[test]
    fn whole_triangles_are_exact_and_special_values_are_ieee_s() {
        assert_eq!(hypot(3.0, 4.0), 5.0);
        assert_eq!(hypot(-5.0, 12.0), 13.0);
        assert_eq!(hypot(0.0, -0.0), 0.0);
        assert_eq!(hypot(0.0, 7.5), 7.5);
        assert_eq!(hypot(f64::NAN, f64::INFINITY), f64::INFINITY);
        assert_eq!(hypot(f64::NEG_INFINITY, f64::NAN), f64::INFINITY);
        assert!(hypot(f64::NAN, 1.0).is_nan());
        assert_eq!(hypot(f64::MAX, f64::MAX), f64::INFINITY);
        // The smallest doubles: 5e-324 is 2^-1074, and √2 of it rounds to it.
        assert_eq!(hypot(5e-324, 5e-324), 5e-324);
        assert_eq!(hypot(1.5e-323, 2e-323), 2.5e-323);
    }

    /// The input the measurement of docs/adr/9403 shows: three libraries give …924, the nearest double is …923.
    #[test]
    fn the_measured_input_is_rounded_to_the_nearest() {
        assert_eq!(
            hypot(896.809_549_769_400_7, -294.254_883_547_293_4),
            943.850_255_627_992_3
        );
    }

    /// The product's error is the same double by the fused operation and by the split factors: on a processor with
    /// a fused multiply-add the first is the processor's, elsewhere the library's exact one; either way they agree.
    #[test]
    fn the_product_s_error_is_the_same_both_ways() {
        let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            seed
        };
        for i in 0..200_000u32 {
            let scale = power_of_two((next() % 900) as i32 - 450);
            let (a, b) = (
                (1.0 + unit(&mut next)) * scale,
                (1.0 + unit(&mut next)) * scale,
            );
            let (a, b) = if i % 3 == 0 {
                (a, a)
            } else {
                (a, b * power_of_two(-((next() % 29) as i32)))
            };
            let p = a * b;
            assert_eq!(
                fused_error(a, b, p).to_bits(),
                split_error(a, b, p).to_bits(),
                "{a:e} · {b:e}"
            );
        }
    }

    /// The two paths agree on a fixed sequence of a million pairs: at drawing size, at map size with millimetre
    /// coordinates, over the whole range of exponents, and among the subnormals. Which double is the right one is the
    /// independent reference's to say (`tests/all/hypot.rs`); here the quick path is held to the exact one.
    #[test]
    fn the_two_paths_give_the_same_double() {
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            seed
        };
        let mut checked = 0;
        for i in 0..1_000_000u32 {
            let (x, y) = match i % 4 {
                0 => (
                    unit(&mut next) * 2000.0 - 1000.0,
                    unit(&mut next) * 2000.0 - 1000.0,
                ),
                1 => (
                    ((unit(&mut next) - 0.5) * 2e6).floor() / 1000.0,
                    ((unit(&mut next) - 0.5) * 2e6).floor() / 1000.0,
                ),
                2 => {
                    // Any exponent, any fraction; a few bits apart or far apart.
                    let a = f64::from_bits(next() & 0x7fef_ffff_ffff_ffff);
                    let apart = (next() % 64) as i32 - 2;
                    let b = a * power_of_two(-apart.clamp(-2, 60)) * (0.5 + unit(&mut next));
                    (a, b)
                }
                _ => (f64::from_bits(next() >> 12), f64::from_bits(next() >> 12)), // subnormals
            };
            if !(x.is_finite() && y.is_finite()) || x == 0.0 || y == 0.0 {
                continue;
            }
            let (a, b) = if x.abs() >= y.abs() {
                (x.abs(), y.abs())
            } else {
                (y.abs(), x.abs())
            };
            assert_eq!(
                hypot(x, y).to_bits(),
                exact(a, b).to_bits(),
                "hypot({x:e}, {y:e})"
            );
            checked += 1;
        }
        assert!(checked > 990_000, "{checked}");
    }
}
