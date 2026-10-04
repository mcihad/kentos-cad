//! Exact field values (docs/adr/0169 §1, CLAUDE.md §23): a number as the
//! instrument wrote it, decimal or DDD.MMSS, kept as a fraction until it is
//! rounded once to the nearest float64. The readers of Topcon GTS-7 and
//! Nikon RAW share it; their independent references compute with Python's
//! `Fraction` and its `float()`.

use std::cmp::Ordering;

/// A number's text: an optional sign, digits with an optional point, or a
/// point and digits; at most 30 digits.
fn parts(text: &str) -> Option<(bool, &str, &str)> {
    let (minus, body) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    // Digits with an optional point and digits, or a point and digits.
    let shape = if whole.is_empty() {
        body.starts_with('.') && !fraction.is_empty()
    } else {
        true
    };
    let ok = shape && digits(whole) && digits(fraction) && whole.len() + fraction.len() <= 30;
    ok.then_some((minus, whole, fraction))
}

/// Digits as an integer (empty is zero); at most 30 of them fit.
fn integer(digits: &str) -> i128 {
    digits
        .bytes()
        .fold(0i128, |v, b| v * 10 + i128::from(b - b'0'))
}

/// Why a DDD.MMSS angle is not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dms {
    /// Not a number.
    Number,
    /// Its minutes or seconds are 60 or more.
    Sexagesimal,
}

/// An exact value: a numerator over a positive denominator.
#[derive(Clone, Copy, Debug)]
pub struct Exact {
    num: i128,
    den: i128,
}

impl Exact {
    /// A decimal number's exact value; none for a text that is not one.
    pub fn decimal(text: &str) -> Option<Self> {
        let (minus, whole, fraction) = parts(text)?;
        let den = 10i128.pow(u32::try_from(fraction.len()).ok()?);
        let num = integer(whole) * den + integer(fraction);
        Some(Self {
            num: if minus { -num } else { num },
            den,
        })
    }

    /// A DDD.MMSS angle's exact value in degrees: the fraction's first two
    /// digits minutes, the next two seconds, the rest the seconds'
    /// fraction; fewer digits are zeros.
    pub fn dms(text: &str) -> Result<Self, Dms> {
        let (minus, whole, fraction) = parts(text).ok_or(Dms::Number)?;
        let padded = format!("{fraction:0<4}");
        let (minutes, seconds, rest) =
            (integer(&padded[..2]), integer(&padded[2..4]), &padded[4..]);
        if minutes >= 60 || seconds >= 60 {
            return Err(Dms::Sexagesimal);
        }
        let scale = 10i128.pow(u32::try_from(rest.len()).map_err(|_| Dms::Number)?);
        let num = ((integer(whole) * 3600 + minutes * 60 + seconds) * scale) + integer(rest);
        Ok(Self {
            num: if minus { -num } else { num },
            den: 3600 * scale,
        })
    }

    /// Below zero.
    pub fn negative(self) -> bool {
        self.num < 0
    }

    /// Not zero.
    pub fn nonzero(self) -> bool {
        self.num != 0
    }

    /// Its size against a whole number.
    pub fn size_cmp(self, n: i128) -> Ordering {
        self.num.abs().cmp(&(n * self.den))
    }

    /// Plus a whole number.
    pub fn plus(self, n: i128) -> Self {
        Self {
            num: self.num + n * self.den,
            den: self.den,
        }
    }

    /// The value's nearest float64: its decimal expansion, exact or to 120
    /// digits (far closer than any halfway point between two float64s the
    /// fraction could be near), read once (a negative zero is zero).
    pub fn value(self) -> f64 {
        let (num, den) = (self.num.unsigned_abs(), self.den.unsigned_abs());
        let mut text = String::with_capacity(160);
        if self.num < 0 {
            text.push('-');
        }
        text.push_str(&(num / den).to_string());
        let mut rest = num % den;
        if rest != 0 {
            text.push('.');
            for _ in 0..120 {
                rest *= 10;
                text.push(char::from(b'0' + (rest / den) as u8));
                rest %= den;
                if rest == 0 {
                    break;
                }
            }
        }
        text.parse::<f64>().unwrap_or(f64::NAN) + 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_rounded_once() {
        assert_eq!(Exact::decimal("0.1").map(Exact::value), Some(0.1));
        assert_eq!(Exact::decimal("-0").map(Exact::value), Some(0.0));
        assert_eq!(Exact::decimal(".5").map(Exact::value), Some(0.5));
        assert_eq!(Exact::decimal("+7").map(Exact::value), Some(7.0));
        assert_eq!(Exact::decimal("1.").map(Exact::value), Some(1.0));
        for bad in ["", "-", ".", "+", "1e3", "1,5", " 1", "--1", "1.2.3"] {
            assert!(Exact::decimal(bad).is_none(), "{bad}");
        }
        // 97°57'06.0": 352626/3600 degrees.
        let v = Exact::dms("97.57060").expect("an angle").value();
        assert_eq!(v, 352_626.0 / 3600.0);
        assert_eq!(
            Exact::dms("10.6000").map(Exact::value),
            Err(Dms::Sexagesimal)
        );
        assert_eq!(
            Exact::dms("-37.26440").map(|v| v.plus(360).value()),
            Ok((360.0 * 3600.0 - 134_804.0) / 3600.0)
        );
    }
}
