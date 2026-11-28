//! Which of two numbers is the smaller, and a number's sign, where a NaN and the two zeros are told apart: IEEE
//! 754-2019's `minimum` and `maximum` (§9.6). Rust's `f64::min` and `f64::max` drop a NaN and do not order the
//! zeros; a computation that must not lose a NaN on its way uses these.

/// The smaller of two numbers: a NaN when either is one, and −0 before +0.
pub fn minimum(a: f64, b: f64) -> f64 {
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

/// The larger of two numbers: a NaN when either is one, and +0 after −0.
pub fn maximum(a: f64, b: f64) -> f64 {
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

/// The smallest of any number of values; +∞ for none.
pub fn minimum_of(xs: impl IntoIterator<Item = f64>) -> f64 {
    xs.into_iter().fold(f64::INFINITY, minimum)
}

/// The largest of any number of values; −∞ for none.
pub fn maximum_of(xs: impl IntoIterator<Item = f64>) -> f64 {
    xs.into_iter().fold(f64::NEG_INFINITY, maximum)
}

/// A number's sign as −1 or 1; a zero stays the zero it is and a NaN a NaN.
pub fn sign(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        x
    } else if x > 0.0 {
        1.0
    } else {
        -1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nan_is_kept_and_the_zeros_are_told_apart() {
        assert!(sign(-0.0).is_sign_negative() && sign(-0.0) == 0.0);
        assert!(sign(0.0).is_sign_positive());
        assert!(sign(f64::NAN).is_nan());
        assert_eq!((sign(-3.0), sign(2.5)), (-1.0, 1.0));
        assert!(minimum(0.0, -0.0).is_sign_negative() && minimum(-0.0, 0.0).is_sign_negative());
        assert!(maximum(-0.0, 0.0).is_sign_positive() && maximum(0.0, -0.0).is_sign_positive());
        assert!(minimum(f64::NAN, 1.0).is_nan() && minimum(1.0, f64::NAN).is_nan());
        assert!(maximum(1.0, f64::NAN).is_nan() && maximum(f64::NAN, 1.0).is_nan());
        assert_eq!((minimum(2.0, -7.5), maximum(2.0, -7.5)), (-7.5, 2.0));
        assert_eq!(minimum(f64::NEG_INFINITY, 1.0), f64::NEG_INFINITY);
        assert_eq!(minimum_of([]), f64::INFINITY);
        assert_eq!(maximum_of([]), f64::NEG_INFINITY);
        assert_eq!(
            (minimum_of([3.0, -1.0, 2.0]), maximum_of([3.0, -1.0, 2.0])),
            (-1.0, 3.0)
        );
        assert!(maximum_of([1.0, f64::NAN, 5.0]).is_nan());
    }
}
