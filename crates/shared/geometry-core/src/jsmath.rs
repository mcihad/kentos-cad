//! JavaScript's number semantics, for a faithful port of the TypeScript core
//! (docs/adr/0008). Rust and JavaScript agree on IEEE arithmetic, `sqrt`,
//! `floor`, `abs` and `%`, but not on these: `Math.round` rounds halves up,
//! `Math.sign`/`min`/`max` keep NaN and signed zeros their own way, and V8's
//! `Math.hypot` is a scaled, compensated sum rather than the C function.
//! Transcendental functions come from `libm` on every target (never the
//! platform's), so native and WASM give the same bits (CLAUDE.md §23.4);
//! `clippy.toml` forbids the std methods.

use std::cmp::Ordering;

pub use libm::{acos, asin, atan, atan2, cos, exp, log, pow, sin, tan};

pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = PI * 2.0;

/// `Math.round`: the nearest integer, halves toward +∞ (−2.5 → −2, 0.49999999999999994 → 0).
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    // Math.round keeps the sign of a zero result from a negative input (−0.4 → −0).
    if r == 0.0 && (x < 0.0 || x.is_sign_negative()) {
        -0.0
    } else {
        r
    }
}

/// `Math.floor` (the same as f64::floor; named for symmetry with the other helpers).
pub fn js_floor(x: f64) -> f64 {
    x.floor()
}

/// `Math.sign`: NaN stays NaN and a zero keeps its sign.
pub fn js_sign(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        x
    } else if x > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// `Math.min(a, b)`: NaN wins, and −0 is smaller than +0.
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { a } else { b }
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.max(a, b)`: NaN wins, and +0 is larger than −0.
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { b } else { a }
    } else if a > b {
        a
    } else {
        b
    }
}

/// `Math.min(...xs)`; +∞ for none.
pub fn js_min_all(xs: impl IntoIterator<Item = f64>) -> f64 {
    xs.into_iter().fold(f64::INFINITY, js_min)
}

/// `Math.max(...xs)`; −∞ for none.
pub fn js_max_all(xs: impl IntoIterator<Item = f64>) -> f64 {
    xs.into_iter().fold(f64::NEG_INFINITY, js_max)
}

/// V8's `Math.hypot`: every argument divided by the largest, the squares
/// summed with Kahan compensation, then scaled back. Bit for bit what the
/// TypeScript core computed.
pub fn js_hypot_n(xs: &[f64]) -> f64 {
    let mut max = 0.0_f64;
    let mut nan = false;
    for &x in xs {
        let a = x.abs();
        if a.is_nan() {
            nan = true;
        } else if a > max {
            max = a;
        }
    }
    if max == f64::INFINITY {
        return f64::INFINITY;
    }
    if nan {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    for &x in xs {
        let n = x.abs() / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

/// `Math.hypot(a, b)`.
pub fn js_hypot(a: f64, b: f64) -> f64 {
    js_hypot_n(&[a, b])
}

/// The ordering a JavaScript comparator `(a, b) => a − b` gives: NaN compares
/// equal, so a stable sort keeps such items in place.
pub fn js_cmp(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// A JavaScript truthiness test on a number (`x || fallback` falls back on 0, −0 and NaN).
pub fn truthy(x: f64) -> bool {
    !(x == 0.0 || x.is_nan())
}

/// `x || fallback` for numbers.
pub fn or(x: f64, fallback: f64) -> f64 {
    if truthy(x) { x } else { fallback }
}

/// A stable sort, as JavaScript's `Array.prototype.sort` is: merge sort
/// with a buffer. One copy of the code per element type, not per comparator
/// (std's `sort_by` is instantiated for every closure and weighs ~6 KB each
/// in the WASM package).
pub fn stable_sort<T: Clone>(v: &mut [T], cmp: &mut dyn FnMut(&T, &T) -> Ordering) {
    if v.len() < 2 {
        return;
    }
    let mut buf = v.to_vec();
    merge_sort(v, &mut buf, cmp);
}

fn merge_sort<T: Clone>(v: &mut [T], buf: &mut [T], cmp: &mut dyn FnMut(&T, &T) -> Ordering) {
    let n = v.len();
    if n <= 12 {
        // Insertion sort: stable, and fastest on the short lists geometry sorts.
        for i in 1..n {
            let mut j = i;
            while j > 0 && cmp(&v[j - 1], &v[j]) == Ordering::Greater {
                v.swap(j - 1, j);
                j -= 1;
            }
        }
        return;
    }
    let mid = n / 2;
    {
        let (left, right) = v.split_at_mut(mid);
        let (bl, br) = buf.split_at_mut(mid);
        merge_sort(left, bl, cmp);
        merge_sort(right, br, cmp);
    }
    // Merge v[..mid] and v[mid..] through the buffer; ties take the left one.
    buf[..n].clone_from_slice(v);
    let (mut i, mut j, mut k) = (0, mid, 0);
    while i < mid && j < n {
        if cmp(&buf[j], &buf[i]) == Ordering::Less {
            v[k] = buf[j].clone();
            j += 1;
        } else {
            v[k] = buf[i].clone();
            i += 1;
        }
        k += 1;
    }
    while i < mid {
        v[k] = buf[i].clone();
        i += 1;
        k += 1;
    }
    while j < n {
        v[k] = buf[j].clone();
        j += 1;
        k += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_matches_math_round() {
        for (x, want) in [
            (2.5, 3.0),
            (-2.5, -2.0),
            (0.49999999999999994, 0.0),
            (-0.5, -0.0),
            (1.4999999999999998, 1.0),
            (4503599627370495.5, 4503599627370496.0),
            (-7.6, -8.0),
        ] {
            let got = js_round(x);
            assert_eq!(got.to_bits(), f64::to_bits(want), "round({x}) = {got}");
        }
        assert!(js_round(-0.4).is_sign_negative());
        assert!(js_round(f64::NAN).is_nan());
    }

    #[test]
    fn sign_min_max_keep_nan_and_signed_zeros() {
        assert!(js_sign(-0.0).is_sign_negative() && js_sign(-0.0) == 0.0);
        assert!(js_sign(f64::NAN).is_nan());
        assert_eq!(js_sign(-3.0), -1.0);
        assert!(js_min(0.0, -0.0).is_sign_negative());
        assert!(!js_max(-0.0, 0.0).is_sign_negative());
        assert!(js_min(f64::NAN, 1.0).is_nan());
        assert!(js_max(1.0, f64::NAN).is_nan());
        assert_eq!(js_min_all([]), f64::INFINITY);
    }

    #[test]
    fn stable_sort_keeps_ties_in_order_like_javascript() {
        let mut v: Vec<(i32, usize)> = (0..200).map(|i: i32| (i * 7919 % 13, i as usize)).collect();
        let mut want = v.clone();
        want.sort_by_key(|p| p.0); // std's stable sort
        stable_sort(&mut v, &mut |a, b| a.0.cmp(&b.0));
        assert_eq!(v, want);
        let mut f = vec![3.0, f64::NAN, 1.0, 2.0];
        stable_sort(&mut f, &mut |a, b| js_cmp(*a, *b));
        assert_eq!(f.len(), 4);
    }

    #[test]
    fn hypot_is_v8s_compensated_sum() {
        assert_eq!(js_hypot(3.0, 4.0), 5.0);
        assert_eq!(js_hypot(0.0, -0.0), 0.0);
        assert_eq!(js_hypot(f64::NAN, f64::INFINITY), f64::INFINITY);
        assert!(js_hypot(f64::NAN, 1.0).is_nan());
        // Large coordinates do not overflow.
        assert_eq!(js_hypot(1e300, 1e300), 1e300 * 2f64.sqrt());
        // Recorded from V8 (node 24): Math.hypot(1, 1e-8), Math.hypot(4400000.123, 486512.34).
        assert_eq!(js_hypot(1.0, 1e-8), 1.0);
        assert_eq!(js_hypot(4400000.123, 486512.34), 4426815.485128365);
    }
}
