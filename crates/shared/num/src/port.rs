//! What is left of the helpers the geometry core was ported from TypeScript with (docs/adr/9403 §1): each still
//! waits for a rule of its own. A comparison that calls a NaN equal, a number taken as "given" when it is neither
//! zero nor NaN, a sort that bears such a comparison, and a length of many parts by a scaled sum. They are here so
//! that one crate holds every number rule; a new use is not written (`make guard` counts them).

use std::cmp::Ordering;

/// The length of a vector of any number of parts: every part divided by the largest, the squares summed with
/// Kahan compensation, then scaled back (the port's form; two parts go to [`crate::hypot`], which is correctly rounded).
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
}
