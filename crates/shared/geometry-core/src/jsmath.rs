//! JavaScript's number semantics, from the port of the TypeScript core
//! (docs/adr/0008): `Math.round` rounds halves up, and `Math.sign`/`min`/`max`
//! keep NaN and signed zeros their own way. The numeric base takes their place
//! family by family (`kentos-num`, docs/adr/9403); a length is its already.
//! Transcendental functions are the numeric base's too: correctly rounded, so
//! every system gives the same bits (CLAUDE.md §23.4); `clippy.toml` forbids
//! the std methods, which call the platform's library.

use std::cmp::Ordering;

pub use kentos_num::{
    acos, asin, atan, atan2, cos, exp, hypot, ln as log, log2, log10, maximum, maximum_of, minimum,
    minimum_of, pow, sign, sin, tan,
};

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

/// The length of a vector of any number of parts: every part divided by the largest, the squares summed with
/// Kahan compensation, then scaled back (the port's form; two parts go to [`hypot`], which is correctly rounded).
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
/// with a buffer. One copy of the code per element type, not per comparator.
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
    fn the_sum_of_many_squares_is_scaled_and_compensated() {
        assert_eq!(js_hypot_n(&[3.0, 4.0]), 5.0);
        assert_eq!(js_hypot_n(&[2.0, 3.0, 6.0]), 7.0);
        assert_eq!(js_hypot_n(&[0.0, -0.0]), 0.0);
        assert_eq!(js_hypot_n(&[f64::NAN, f64::INFINITY]), f64::INFINITY);
        assert!(js_hypot_n(&[f64::NAN, 1.0]).is_nan());
        // Large coordinates do not overflow.
        assert_eq!(js_hypot_n(&[1e300, 1e300]), 1e300 * 2f64.sqrt());
    }

    /// A length is the numeric base's: the nearest double, where the port's scaled sum was one place off (the
    /// base's own cases are in fixtures/numeric/v1/hypot.json).
    #[test]
    fn a_length_is_the_nearest_double() {
        assert_eq!(hypot(3.0, 4.0), 5.0);
        assert_eq!(hypot(f64::NAN, f64::INFINITY), f64::INFINITY);
        let (x, y) = (896.809_549_769_400_7, -294.254_883_547_293_4);
        assert_eq!(hypot(x, y), 943.850_255_627_992_3);
    }
}
