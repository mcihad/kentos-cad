//! JavaScript's reading of numbers and lists as the TypeScript read them:
//! `Number`, `parseFloat`, `parseInt(s, 16)`, `\s`-splitting, `/\d/`.

use kentos_style_core::js::text::{is_space, trim};

use crate::path::number_len;

/// `Number(s)` for text: the whole text (white space trimmed) as a decimal,
/// hexadecimal, octal or binary literal or ±Infinity; "" is 0; NaN otherwise.
pub fn js_number(s: &str) -> f64 {
    let t = trim(s);
    if t.is_empty() {
        return 0.0;
    }
    let (sign, body) = match t.as_bytes()[0] {
        b'+' => (1.0, &t[1..]),
        b'-' => (-1.0, &t[1..]),
        _ => (1.0, t),
    };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    let b = body.as_bytes();
    if b.len() > 2 && b[0] == b'0' && sign == 1.0 && t.as_bytes()[0] != b'+' {
        let radix = match b[1] {
            b'x' | b'X' => 16,
            b'o' | b'O' => 8,
            b'b' | b'B' => 2,
            _ => 0,
        };
        if radix != 0 {
            let digits: Option<Vec<u32>> = body[2..].chars().map(|c| c.to_digit(radix)).collect();
            return digits.map_or(f64::NAN, |d| whole(&d, radix));
        }
    }
    if number_len(t.as_bytes()) == t.len() {
        t.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// `parseFloat(s)`: the longest decimal (or Infinity) at the start, after white space; NaN when none.
pub fn parse_float(s: &str) -> f64 {
    let t = s.trim_start_matches(is_space);
    let b = t.as_bytes();
    let signed = !b.is_empty() && (b[0] == b'+' || b[0] == b'-');
    let rest = if signed { &t[1..] } else { t };
    if rest.starts_with("Infinity") {
        return if signed && b[0] == b'-' {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let n = number_len(b);
    if n == 0 {
        return f64::NAN;
    }
    t[..n].parse::<f64>().unwrap_or(f64::NAN)
}

/// `s.split(re).filter(Boolean)` for a separator class: runs of the class split, empty parts go.
pub fn split_on(s: &str, sep: impl Fn(char) -> bool) -> Vec<&str> {
    s.split(sep).filter(|p| !p.is_empty()).collect()
}

/// `s.split(/\s+/)` (empty parts kept, as at the ends).
pub fn split_ws(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_space = false;
    for (i, c) in s.char_indices() {
        if is_space(c) {
            if !in_space {
                out.push(&s[start..i]);
                in_space = true;
            }
        } else if in_space {
            start = i;
            in_space = false;
        }
    }
    out.push(if in_space { "" } else { &s[start..] });
    out
}

/// `/\d/`: an ASCII digit.
fn digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// Every number written in `v` (`/[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?/g`).
pub fn nums(v: Option<&str>) -> Vec<f64> {
    let Some(v) = v else { return Vec::new() };
    let s = v.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let n = number_len(&s[i..]);
        if n > 0 {
            out.push(v[i..i + n].parse::<f64>().unwrap_or(f64::NAN));
            i += n;
        } else {
            i += 1;
        }
    }
    out
}

/// `parseInt(s, 16)`: white space, a sign and a `0x` prefix skipped, then
/// the leading hex digits; NaN when there are none.
pub fn parse_hex(s: &str) -> f64 {
    let t = s.trim_start_matches(is_space);
    let (sign, t) = match t.as_bytes().first() {
        Some(b'-') => (-1.0, &t[1..]),
        Some(b'+') => (1.0, &t[1..]),
        _ => (1.0, t),
    };
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    let digits: Vec<u32> = t.chars().map_while(|c| c.to_digit(16)).collect();
    if digits.is_empty() {
        return f64::NAN;
    }
    sign * whole(&digits, 16)
}

/// The integer written by `digits` in `radix`, rounded once to the nearest
/// double (as JavaScript reads a hexadecimal literal).
fn whole(digits: &[u32], radix: u32) -> f64 {
    let mut v: u128 = 0;
    for (k, &d) in digits.iter().enumerate() {
        match v
            .checked_mul(u128::from(radix))
            .and_then(|x| x.checked_add(u128::from(d)))
        {
            Some(x) => v = x,
            // Beyond 2¹²⁸ the double is the leading digits scaled: exact enough to round once.
            None => {
                let mut f = v as f64;
                for &d in &digits[k..] {
                    f = f * f64::from(radix) + f64::from(d);
                }
                return f;
            }
        }
    }
    v as f64
}

/// `/\d/.test(first char)`.
pub fn starts_with_digit(s: &str) -> bool {
    s.chars().next().is_some_and(digit)
}
