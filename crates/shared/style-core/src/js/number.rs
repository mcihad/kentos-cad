//! JavaScript's number-to-text conversions, exactly: `String(x)`,
//! `x.toPrecision(p)` and `x.toFixed(f)`. What the expression language
//! writes into attributes and labels is the same text on every target and
//! the same as the TypeScript it replaces (docs/adr/0008).
//!
//! Digits come from Rust's formatter: its shortest form (`{:e}`) is
//! JavaScript's (the shortest digits that read back to the number, the
//! closest of them), and its exact mode prints a double's decimal
//! expansion. Rounding is done here, because JavaScript rounds a tie up
//! (it "picks the larger n") where Rust rounds it to even. Digits stay on
//! the stack: a label for each of a hundred thousand objects allocates only
//! its text.

use std::cmp::Ordering;
use std::fmt::{self, Write};

/// Most digits a rounding here yields: `toFixed` keeps at most 21 integer
/// digits (larger numbers print as `String(x)`) and 100 decimals,
/// `toPrecision` at most 100 digits, and a carry may add one.
const MAX_DIGITS: usize = 128;

/// Decimal digits (each 0–9), on the stack.
#[derive(Clone, Copy)]
struct Digits {
    d: [u8; MAX_DIGITS],
    len: usize,
}

impl Digits {
    const NONE: Digits = Digits {
        d: [0; MAX_DIGITS],
        len: 0,
    };

    fn get(&self) -> &[u8] {
        &self.d[..self.len]
    }

    /// The first `k` digits.
    fn prefix(mut self, k: usize) -> Digits {
        self.len = self.len.min(k);
        self
    }

    /// Zeros after the digits, `k` digits in all.
    fn pad(mut self, k: usize) -> Digits {
        let k = k.min(MAX_DIGITS);
        if k > self.len {
            self.d[self.len..k].fill(0);
            self.len = k;
        }
        self
    }

    /// Without trailing zeros (one digit stays).
    fn trimmed(mut self) -> Digits {
        while self.len > 1 && self.d[self.len - 1] == 0 {
            self.len -= 1;
        }
        self
    }
}

/// Text on the stack: the formatter's output for one number.
struct Stack<const N: usize> {
    b: [u8; N],
    n: usize,
}

impl<const N: usize> Stack<N> {
    fn new() -> Self {
        Stack { b: [0; N], n: 0 }
    }

    fn bytes(&self) -> &[u8] {
        &self.b[..self.n]
    }
}

impl<const N: usize> Write for Stack<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.n + s.len();
        self.b
            .get_mut(self.n..end)
            .ok_or(fmt::Error)?
            .copy_from_slice(s.as_bytes());
        self.n = end;
        Ok(())
    }
}

/// Digits and exponent of Rust's scientific form "d.ddde±n":
/// x = d₁.d₂d₃… × 10^e (its first `MAX_DIGITS` digits).
fn sci(s: &[u8]) -> (Digits, i32) {
    let mut n = Digits::NONE;
    let mut rest = s;
    while let [b, tail @ ..] = rest {
        rest = tail;
        match *b {
            b'e' => break,
            b'0'..=b'9' if n.len < MAX_DIGITS => {
                n.d[n.len] = b - b'0';
                n.len += 1;
            }
            _ => {}
        }
    }
    let (negative, rest) = match rest {
        [b'-', tail @ ..] => (true, tail),
        _ => (false, rest),
    };
    let e = rest
        .iter()
        .filter(|b| b.is_ascii_digit())
        .fold(0i32, |e, &b| {
            e.saturating_mul(10).saturating_add(i32::from(b - b'0'))
        });
    (n, if negative { -e } else { e })
}

/// The shortest digits that read back to `x` (finite, > 0) and their exponent.
fn shortest(x: f64) -> (Digits, i32) {
    // At most 17 digits, a point and "e-324".
    let mut s = Stack::<32>::new();
    let _ = write!(s, "{x:e}");
    sci(s.bytes())
}

/// The digits of `x`'s exact decimal expansion (finite, > 0) up to digit
/// `k` (0-based), and its exponent. The formatter rounds its last digit;
/// the carry reaches digit `k` only through a run of nines, which then
/// print as zeros, so a nonzero digit among the twenty after `k` shows the
/// digits up to `k` are exact. Otherwise the whole expansion is read (a
/// double has at most 767 significant digits), trailing zeros dropped.
fn exact(x: f64, k: usize) -> (Digits, i32) {
    let mut s = Stack::<{ MAX_DIGITS + 48 }>::new();
    if write!(s, "{x:.*e}", k + 20).is_ok() {
        let b = s.bytes();
        // Digit j ≥ 1 of the mantissa "d.ddd…" is at j + 1.
        let mantissa = b.split(|&c| c == b'e').next().unwrap_or_default();
        if mantissa
            .get(k + 2..)
            .is_some_and(|after| after.iter().any(|&c| c != b'0'))
        {
            let (d, e) = sci(b);
            return (d.prefix(k + 1), e);
        }
    }
    let (d, e) = sci(format!("{x:.800e}").as_bytes());
    (d.trimmed(), e)
}

/// Whether a place is coarser than `x`'s precision: its unit 10^place is
/// larger than the gap to the next double (with a margin, so a near case
/// takes the exact path).
fn coarser_than_ulp(x: f64, place: i32) -> bool {
    let biased = ((x.to_bits() >> 52) & 0x7ff) as i32;
    // The gap to the next double is 2^u (subnormals: 2^-1074).
    let u = if biased == 0 { -1074 } else { biased - 1075 };
    f64::from(place) * std::f64::consts::LOG2_10 > f64::from(u) + 1.0
}

/// 5^0 … 5^27 (all below 2^64).
const POW5: [u64; 28] = {
    let mut t = [1u64; 28];
    let mut i = 1;
    while i < t.len() {
        t[i] = t[i - 1] * 5;
        i += 1;
    }
    t
};

/// 10^0 … 10^22: the powers of ten a double holds exactly.
const POW10: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

/// Most decimals the exact scaling takes: 2^53 × 5^17 is below 2^93.
const SCALE_MAX: i32 = 17;

/// A finite double's significand and exponent: x = m × 2^q.
fn parts(x: f64) -> (u64, i32) {
    let bits = x.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1 << 52) - 1);
    if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | 1 << 52, biased - 1075)
    }
}

/// x × 10^f rounded half up, exactly, when that is below 2^64 (0 ≤ x < 1e21,
/// 0 ≤ f ≤ 17): x × 10^f = m × 5^f × 2^(q+f), below 2^127, in integers.
fn scaled(x: f64, f: i32) -> Option<u64> {
    let (m, q) = parts(x);
    let v = u128::from(m) * u128::from(POW5[f.clamp(0, SCALE_MAX) as usize]);
    let s = q + f;
    let n = if s >= 0 {
        v << s
    } else {
        let k = s.unsigned_abs();
        // v < 2^93: past 2^(k−1) it rounds to zero.
        if k >= 127 {
            0
        } else {
            (v + (1u128 << (k - 1))) >> k
        }
    };
    u64::try_from(n).ok()
}

/// Whether x ≥ 10^k, exactly (x finite and > 0, −17 ≤ k ≤ 22).
fn at_least_pow10(x: f64, k: i32) -> bool {
    if k >= 0 {
        return x >= POW10[k.min(22) as usize];
    }
    // m × 2^q ≥ 10^k ⇔ m × 5^−k ≥ 2^(k−q).
    let (m, q) = parts(x);
    let v = u128::from(m) * u128::from(POW5[k.unsigned_abs().min(27) as usize]);
    let s = k - q;
    s <= 0 || (s < 128 && v >= 1u128 << s)
}

/// The digits of `v` (no leading zeros; none for zero).
fn digits_of(mut v: u64) -> Digits {
    let mut d = Digits::NONE;
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    while v > 0 && i > 0 {
        i -= 1;
        buf[i] = (v % 10) as u8;
        v /= 10;
    }
    d.len = buf.len() - i;
    d.d[..d.len].copy_from_slice(&buf[i..]);
    d
}

/// Digits one unit higher in the last place when `up`; leading zeros
/// dropped (none left: zero).
fn rounded(mut n: Digits, up: bool) -> Digits {
    if up {
        let mut i = n.len;
        loop {
            if i == 0 {
                // All nines: one digit more.
                let len = n.len.min(MAX_DIGITS - 1);
                n.d.copy_within(0..len, 1);
                n.d[0] = 1;
                n.len = len + 1;
                break;
            }
            i -= 1;
            if n.d[i] == 9 {
                n.d[i] = 0;
            } else {
                n.d[i] += 1;
                break;
            }
        }
    }
    let lead = n.get().iter().take_while(|&&b| b == 0).count();
    n.d.copy_within(lead..n.len, 0);
    n.len -= lead;
    n
}

/// `x` (finite, > 0) in units of 10^place, rounded half up: the digits of
/// that integer, without leading zeros (none for zero). `short` is `x`'s
/// shortest form.
///
/// The shortest digits decide when they can: a midpoint with no more digits
/// than the shortest form, other than the form itself, lies outside the
/// decimals that read back to `x` or farther from `x` than the form, so `x`
/// is on the same side of it as its shortest form; and a shortest form that
/// ends at or above the place is the rounded value when the place is coarser
/// than `x`'s precision. Otherwise (a midpoint equal to the shortest form, a
/// place finer than the double) the exact expansion decides, and an exact
/// tie rounds up.
fn round_at(x: f64, short: &(Digits, i32), place: i32) -> Digits {
    let (d, e) = short;
    // Digits of the shortest form at or above the place.
    let keep = i64::from(*e) - i64::from(place) + 1;
    if keep < 0 {
        return Digits::NONE;
    }
    let k = keep as usize;
    let decided = if k >= d.len {
        if coarser_than_ulp(x, place) {
            return d.pad(k);
        }
        None
    } else {
        match d.d[k].cmp(&5) {
            Ordering::Greater => Some(true),
            Ordering::Less => Some(false),
            Ordering::Equal if d.len > k + 1 => Some(true),
            Ordering::Equal => None,
        }
    };
    if let Some(up) = decided {
        return rounded(d.prefix(k), up);
    }
    // The exact expansion's first digit is the shortest form's or one place
    // lower (9.99…e22 is "1e23"): its digits up to `k` are enough.
    let (xd, xe) = exact(x, k);
    let keep = i64::from(xe) - i64::from(place) + 1;
    if keep < 0 {
        return Digits::NONE;
    }
    let k = keep as usize;
    if k >= xd.len {
        return xd.pad(k);
    }
    // The expansion is exact: 5 and anything after it (a tie included) rounds up.
    rounded(xd.prefix(k), xd.d[k] >= 5)
}

/// Powers of ten a double holds exactly.
const POW10_EXACT: i32 = 22;

/// The exponent of `x`'s first digit (finite, > 0): its shortest form's,
/// unless that form is a power of ten `x` lies just below (the double
/// nearest 1e23 is 9.99…e22).
fn first_exponent(x: f64, short: &(Digits, i32)) -> i32 {
    let (d, e) = short;
    if d.get() != [1] || (0..=POW10_EXACT).contains(e) {
        // A form other than a power of ten has x's first digit; 10^e up to
        // 10^22 is a double, so reading back to it, x is it.
        return *e;
    }
    exact(x, 0).1
}

/// `x` (finite, > 0) to `p` significant digits (1–100), rounded half up:
/// the digits and the exponent of the first.
fn precision_digits(x: f64, p: usize) -> (Digits, i32) {
    precision_by_integers(x, p).unwrap_or_else(|| precision_by_digits(x, p))
}

/// `precision_digits` in integers where they hold it: 10^(p−18) ≤ x < 10^p,
/// so x × 10^(p−1−E) (E: the exponent of x's first digit) is an exact
/// scaling. Most values an expression writes are in this range.
fn precision_by_integers(x: f64, p: usize) -> Option<(Digits, i32)> {
    let p = i32::try_from(p).ok().filter(|&p| p <= SCALE_MAX + 1)?;
    let (_, q) = parts(x);
    // x ≥ 2^(q+52): E is ⌊(q+52)·log₁₀2⌋ or one more (1233/4096 is log₁₀2 to
    // 5e-6); the exact comparisons settle it.
    let mut e = ((q + 52) * 1233) >> 12;
    if !(-16..=20).contains(&e) {
        return None;
    }
    while e > -17 && !at_least_pow10(x, e) {
        e -= 1;
    }
    while e < 21 && at_least_pow10(x, e + 1) {
        e += 1;
    }
    let f = p - 1 - e;
    if !(-16..=20).contains(&e) || !(0..=SCALE_MAX).contains(&f) {
        return None;
    }
    // p ≤ 18: below 10^18.
    let mut d = digits_of(scaled(x, f)?);
    // x < 10^(E+1), so at most 10^p: the carry of 9.99…95.
    if d.len > p as usize {
        d.len = p as usize;
        e += 1;
    }
    Some((d, e))
}

/// `precision_digits` from the shortest form and, where it cannot decide,
/// the exact expansion.
fn precision_by_digits(x: f64, p: usize) -> (Digits, i32) {
    let short = shortest(x);
    let mut e = first_exponent(x, &short);
    let mut n = round_at(x, &short, e - p as i32 + 1);
    if n.len > p {
        // 9.99…95 went up to 10.0…0: one place higher.
        n.len = p;
        e += 1;
    }
    (n.pad(p), e)
}

fn push_digits(out: &mut String, d: &[u8]) {
    out.extend(d.iter().map(|&b| char::from(b'0' + b)));
}

fn push_zeros(out: &mut String, n: usize) {
    out.extend(std::iter::repeat_n('0', n));
}

/// "d.ddde±n", as JavaScript writes an exponent.
fn push_exponential(out: &mut String, d: &[u8], e: i64) {
    let (a, b) = d.split_at(d.len().min(1));
    push_digits(out, a);
    if !b.is_empty() {
        out.push('.');
        push_digits(out, b);
    }
    out.push('e');
    out.push(if e < 0 { '-' } else { '+' });
    let _ = write!(out, "{}", e.unsigned_abs());
}

/// Number::toString of the digits `d` (the last nonzero) with the decimal
/// point after the first `n` of them.
fn push_number(out: &mut String, d: &[u8], n: i64) {
    let k = d.len() as i64;
    if k <= n && n <= 21 {
        push_digits(out, d);
        push_zeros(out, (n - k) as usize);
    } else if 0 < n && n <= 21 {
        let (a, b) = d.split_at(n as usize);
        push_digits(out, a);
        out.push('.');
        push_digits(out, b);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        push_zeros(out, (-n) as usize);
        push_digits(out, d);
    } else {
        push_exponential(out, d, n - 1);
    }
}

/// `String(x)`: JavaScript's Number::toString.
pub fn to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let mut out = String::with_capacity(24);
    let x = if x < 0.0 {
        out.push('-');
        -x
    } else {
        x
    };
    // An integer below 2^53 is its own shortest form: its digits.
    let (d, e) = if x.trunc() == x && x < 9_007_199_254_740_992.0 {
        let d = digits_of(x as u64);
        let n = d.len as i32;
        (d.trimmed(), n - 1)
    } else {
        shortest(x)
    };
    push_number(&mut out, d.get(), i64::from(e) + 1);
    out
}

/// `x.toPrecision(p)` for 1 ≤ p ≤ 100.
pub fn to_precision(x: f64, p: u32) -> String {
    if !x.is_finite() {
        return to_string(x);
    }
    let p = p.clamp(1, 100) as usize;
    let mut out = String::with_capacity(p + 8);
    // −0 prints without a sign.
    let x = if x < 0.0 {
        out.push('-');
        -x
    } else {
        x
    };
    let (digits, e) = if x == 0.0 {
        (Digits::NONE.pad(p), 0)
    } else {
        precision_digits(x, p)
    };
    let m = digits.get();
    let e = i64::from(e);
    if e < -6 || e >= p as i64 {
        push_exponential(&mut out, m, e);
    } else if e >= 0 {
        let (a, b) = m.split_at(e as usize + 1);
        push_digits(&mut out, a);
        if !b.is_empty() {
            out.push('.');
            push_digits(&mut out, b);
        }
    } else {
        out.push_str("0.");
        push_zeros(&mut out, (-(e + 1)) as usize);
        push_digits(&mut out, m);
    }
    out
}

/// `String(Number(x.toPrecision(p)))` for 1 ≤ p ≤ 15: `x` to `p`
/// significant digits, written as the number they read as. A normal double
/// keeps any 15 digits, so that number's shortest form is those digits,
/// trailing zeros dropped. Zero, and numbers whose rounding could leave the
/// normal doubles (2e-308, or 2e+308 which reads as ∞), read the text back.
pub fn to_string_precision(x: f64, p: u32) -> String {
    let p = p.clamp(1, 15);
    if !(1e-307..1e308).contains(&x.abs()) {
        let y: f64 = to_precision(x, p).parse().unwrap_or(f64::NAN);
        return to_string(y);
    }
    let mut out = String::with_capacity(24);
    let x = if x < 0.0 {
        out.push('-');
        -x
    } else {
        x
    };
    let (d, e) = precision_digits(x, p as usize);
    push_number(&mut out, d.trimmed().get(), i64::from(e) + 1);
    out
}

/// `x.toFixed(f)` for 0 ≤ f ≤ 100.
pub fn to_fixed(x: f64, f: u32) -> String {
    if !x.is_finite() || x.abs() >= 1e21 {
        return to_string(x);
    }
    let f = f.min(100) as usize;
    let mut out = String::with_capacity(f + 24);
    // −0 prints without a sign; a negative value rounded to zero keeps it ("-0.00").
    let x = if x < 0.0 {
        out.push('-');
        -x
    } else {
        x
    };
    let n = if x == 0.0 {
        Digits::NONE
    } else {
        let f = f as i32;
        match scaled(x, f).filter(|_| f <= SCALE_MAX) {
            Some(n) => digits_of(n),
            None => round_at(x, &shortest(x), -f),
        }
    };
    let d = n.get();
    let whole = d.len().saturating_sub(f);
    if whole == 0 {
        out.push('0');
    } else {
        push_digits(&mut out, &d[..whole]);
    }
    if f != 0 {
        out.push('.');
        push_zeros(&mut out, f.saturating_sub(d.len()));
        push_digits(&mut out, &d[whole..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_as_javascript_does() {
        let cases: &[(f64, &str)] = &[
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-1.5, "-1.5"),
            (0.1 + 0.2, "0.30000000000000004"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e21, "1e+21"),
            (1.5e21, "1.5e+21"),
            (1e-6, "0.000001"),
            (1e-7, "1e-7"),
            (1.2345e-7, "1.2345e-7"),
            (5e-324, "5e-324"),
            (f64::MAX, "1.7976931348623157e+308"),
            (1e23, "1e+23"),
            (598.5, "598.5"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
        ];
        for &(x, s) in cases {
            assert_eq!(to_string(x), s, "{x:e}");
        }
    }

    #[test]
    fn to_precision_rounds_ties_up_as_javascript_does() {
        let cases: &[(f64, u32, &str)] = &[
            (1234567890.125, 12, "1234567890.13"),
            (0.1 + 0.2, 12, "0.300000000000"),
            (123.456, 12, "123.456000000"),
            (1e21, 12, "1.00000000000e+21"),
            (1e-7, 12, "1.00000000000e-7"),
            (0.000001234, 12, "0.00000123400000000"),
            (999999999999.5, 12, "1.00000000000e+12"),
            (2.5, 1, "3"),
            (0.0, 3, "0.00"),
            (-1.25, 2, "-1.3"),
            (1e23, 12, "1.00000000000e+23"),
            (5e-324, 3, "4.94e-324"),
            (1152921504606846976.0, 20, "1152921504606846976.0"),
            // The double nearest 1e23 is 9.99…e22: its first digit is a place lower.
            (1e23, 20, "9.9999999999999991611e+22"),
            (1e23, 17, "9.9999999999999992e+22"),
            (1e23, 16, "9.999999999999999e+22"),
            (1e22, 20, "1.0000000000000000000e+22"),
            (1e-5, 20, "0.000010000000000000000818"),
            (5e-324, 20, "4.9406564584124654418e-324"),
            (f64::MAX, 20, "1.7976931348623157081e+308"),
            (9.5, 1, "1e+1"),
            (0.000099995, 4, "0.00009999"),
            (1e21, 21, "1.00000000000000000000e+21"),
        ];
        for &(x, p, s) in cases {
            assert_eq!(to_precision(x, p), s, "{x:e}");
        }
    }

    #[test]
    fn to_fixed_rounds_ties_up_as_javascript_does() {
        let cases: &[(f64, u32, &str)] = &[
            (0.125, 2, "0.13"),
            (0.5, 0, "1"),
            (2.5, 0, "3"),
            (1.005, 2, "1.00"),
            (452.1, 2, "452.10"),
            (-0.0001, 2, "-0.00"),
            (-0.0, 2, "0.00"),
            (0.0049, 2, "0.00"),
            (0.005, 2, "0.01"),
            (1e21, 2, "1e+21"),
            (123_456_789.987_654_33, 3, "123456789.988"),
            (0.1 + 0.2, 12, "0.300000000000"),
            (99.995, 2, "100.00"),
            (1.005, 2, "1.00"),
            (9.9999, 2, "10.00"),
            (1152921504606846976.0, 2, "1152921504606846976.00"),
            (123456.789, 12, "123456.789000000004"),
            (1e20, 0, "100000000000000000000"),
            (1e20, 2, "100000000000000000000.00"),
            (123.4565, 3, "123.457"),
            (0.0000005, 6, "0.000000"),
            (4.35, 1, "4.3"),
            (1.45, 1, "1.4"),
            (8.345, 2, "8.35"),
            (9007199254740994.0, 2, "9007199254740994.00"),
        ];
        for &(x, f, s) in cases {
            assert_eq!(to_fixed(x, f), s, "{x:e}");
        }
    }

    /// Cases per random check; `NUMBER_ROUNDS` runs a deep one (the release
    /// build takes millions in a minute).
    fn rounds() -> usize {
        std::env::var("NUMBER_ROUNDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(40_000)
    }

    /// A small seeded generator (xorshift64*), for the random checks below.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }

        /// Doubles of every kind: any bits, and short decimals (their
        /// shortest form often ends in a 5 at the rounding place).
        fn double(&mut self) -> f64 {
            match self.below(4) {
                0 => f64::from_bits(self.next() & 0x7fef_ffff_ffff_ffff),
                1 => {
                    (self.below(20_000_000) as f64)
                        / [1.0, 10.0, 100.0, 1e3, 1e4, 1e5, 1e6, 1e7][self.below(8) as usize]
                }
                2 => f64::from_bits(0x3cb0_0000_0000_0000 + self.below(0x0c00_0000_0000_0000)),
                _ => {
                    [
                        1e23,
                        1e22,
                        0.1,
                        0.5,
                        2.5,
                        1e-7,
                        5e-324,
                        9.5,
                        99.995,
                        1e21 - 1.0,
                    ][self.below(10) as usize]
                        * [1.0, 3.0, 0.1, 1e-3][self.below(4) as usize]
                }
            }
        }
    }

    /// Rounding at `place` the long way: the whole exact expansion, half up.
    fn reference(x: f64, place: i32) -> Vec<u8> {
        let s = format!("{x:.800e}");
        let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let mut d: Vec<u8> = mant
            .bytes()
            .filter(u8::is_ascii_digit)
            .map(|b| b - b'0')
            .collect();
        let keep = exp.parse::<i64>().unwrap_or(0) - i64::from(place) + 1;
        if keep < 0 {
            return Vec::new();
        }
        let k = keep as usize;
        d.resize(d.len().max(k + 1), 0);
        let mut n = d[..k].to_vec();
        if d[k] >= 5 {
            let mut i = n.len();
            loop {
                if i == 0 {
                    n.insert(0, 1);
                    break;
                }
                i -= 1;
                if n[i] == 9 {
                    n[i] = 0;
                } else {
                    n[i] += 1;
                    break;
                }
            }
        }
        let lead = n.iter().take_while(|&&b| b == 0).count();
        n.drain(..lead);
        n
    }

    #[test]
    fn rounding_matches_the_whole_expansion() {
        let mut g = Rng(0x9e37_79b9_7f4a_7c15);
        for _ in 0..rounds() {
            let x = g.double().abs();
            if x == 0.0 || !x.is_finite() {
                continue;
            }
            let (_, e) = shortest(x);
            // Places around the number's digits: the shortest form's end, its precision, beyond.
            let place = e - 20 + g.below(24) as i32;
            let got = round_at(x, &shortest(x), place);
            assert_eq!(
                got.get(),
                reference(x, place).as_slice(),
                "{x:e} at 10^{place}"
            );
        }
    }

    #[test]
    fn shortest_text_of_a_rounding_reads_back_the_long_way() {
        let mut g = Rng(0x0123_4567_89ab_cdef);
        for i in 0..rounds() {
            let x = if i % 2 == 0 { g.double() } else { -g.double() };
            let p = 1 + g.below(15) as u32;
            let slow = to_string(to_precision(x, p).parse().unwrap_or(f64::NAN));
            assert_eq!(to_string_precision(x, p), slow, "{x:e} to {p}");
        }
        assert_eq!(to_string_precision(0.1 + 0.2, 12), "0.3");
        assert_eq!(to_string_precision(5e-324, 12), "5e-324");
        assert_eq!(to_string_precision(-0.0, 12), "0");
    }

    #[test]
    fn exact_scaling_matches_the_whole_expansion() {
        let mut g = Rng(0x5151_7272_9393_b4b4);
        for _ in 0..rounds() {
            let x = g.double().abs();
            if !(x > 0.0 && x < 1e21) {
                continue;
            }
            let f = g.below(18) as i32;
            // Past 2^64 the digits take the other path (to_fixed checks it).
            if let Some(n) = scaled(x, f) {
                assert_eq!(
                    digits_of(n).get(),
                    reference(x, -f).as_slice(),
                    "{x:e} × 10^{f}"
                );
            }
            assert_eq!(
                to_fixed(x, f as u32),
                to_fixed_reference(x, f as usize),
                "{x:e} to {f}"
            );
        }
    }

    /// `toFixed` from the rounding the long way.
    fn to_fixed_reference(x: f64, f: usize) -> String {
        let d: String = reference(x, -(f as i32))
            .iter()
            .map(|&b| char::from(b'0' + b))
            .collect();
        let d = format!("{d:0>width$}", width = f + 1);
        let (whole, part) = d.split_at(d.len() - f);
        if f == 0 {
            whole.to_string()
        } else {
            format!("{whole}.{part}")
        }
    }

    #[test]
    fn precision_in_integers_matches_the_digits() {
        let mut g = Rng(0x0f0f_1e1e_2d2d_3c3c);
        let mut fast = 0;
        for _ in 0..rounds() {
            let x = g.double().abs();
            if !(x > 0.0 && x.is_finite()) {
                continue;
            }
            let p = 1 + g.below(18) as usize;
            if let Some((d, e)) = precision_by_integers(x, p) {
                fast += 1;
                let (want, we) = precision_by_digits(x, p);
                assert_eq!((d.get(), e), (want.get(), we), "{x:e} to {p}");
            }
        }
        // The random doubles reach the integer path often enough to mean something.
        assert!(fast > rounds() / 10, "{fast}");
        for i in 0..100_000u64 {
            let x = (i * 7919 % 9_007_199_254_740_991) as f64;
            assert_eq!(to_string(x), format!("{}", x as u64));
        }
    }
}
