//! Numbers as the language reads them out of text and writes them as text
//! (docs/adr/0100): attribute values are text, so arithmetic reads a number
//! per object, and a number written into an attribute or a label is text.
//! Both are JavaScript's (docs/adr/0008): what `Number(text)` accepts under
//! the language's grammar, read to the same double as Rust's correctly
//! rounded parser; and `String(x)` with float noise dropped.

use crate::js::number;

/// Powers of ten a double holds exactly.
const EXACT_POW10: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

/// Text that is a number by the language's grammar
/// (`^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$`, ASCII digits), read as
/// Rust's correctly rounded parser reads it; None when it is not one. Attribute values are
/// text, so arithmetic reads them per object: with at most 15 significant
/// digits and a power of ten a double holds, the value is one correctly
/// rounded multiplication or division of two exact doubles (Clinger's fast
/// path), the same double the parser gives; the rest goes to the parser.
pub fn number(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0;
    let negative = match b.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    // Significant digits (leading zeros skipped) and the power of ten they are scaled by.
    let (mut m, mut digits, mut e) = (0u64, 0u32, 0i32);
    let mut add = |d: u8, fraction: bool, e: &mut i32| {
        if m == 0 && d == 0 {
            if fraction {
                *e -= 1;
            }
            return;
        }
        if digits < 19 {
            m = m * 10 + u64::from(d);
            digits += 1;
            if fraction {
                *e -= 1;
            }
        } else {
            // Past what a u64 holds: counted, left to the parser.
            digits += 1;
            if !fraction {
                *e += 1;
            }
        }
    };
    let whole = i;
    while i < b.len() && b[i].is_ascii_digit() {
        add(b[i] - b'0', false, &mut e);
        i += 1;
    }
    let whole = i - whole;
    let mut fraction = 0;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let from = i;
        while i < b.len() && b[i].is_ascii_digit() {
            add(b[i] - b'0', true, &mut e);
            i += 1;
        }
        fraction = i - from;
    }
    // "\d+(\.\d*)?" or "\.\d+".
    if whole == 0 && fraction == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        let sign = match b.get(i) {
            Some(b'-') => {
                i += 1;
                -1
            }
            Some(b'+') => {
                i += 1;
                1
            }
            _ => 1,
        };
        let from = i;
        let mut x: i32 = 0;
        while i < b.len() && b[i].is_ascii_digit() {
            x = x.saturating_mul(10).saturating_add(i32::from(b[i] - b'0'));
            i += 1;
        }
        if i == from {
            return None;
        }
        e = e.saturating_add(sign * x);
    }
    if i != b.len() {
        return None;
    }
    if digits <= 15 && (-22..=22).contains(&e) {
        let v = m as f64;
        let v = if e < 0 {
            v / EXACT_POW10[e.unsigned_abs() as usize]
        } else {
            v * EXACT_POW10[e as usize]
        };
        return Some(if negative { -v } else { v });
    }
    s.parse().ok()
}

/// Number to text for attributes: integers as they are, others to 12
/// significant digits, so float noise is dropped (0.1 + 0.2 → "0.3").
pub fn push_number_text(out: &mut String, x: f64) {
    if x.trunc() == x && x.abs() < 1e15 {
        // An integer prints as its digits (−0 as 0), as JavaScript's String does.
        push_integer(out, x as i64);
    } else if !x.is_finite() || x.trunc() == x {
        number::push_string(out, x);
    } else {
        number::push_string_precision(out, x, 12);
    }
}

/// An integer's decimal digits, with a minus sign when it is negative.
fn push_integer(out: &mut String, v: i64) {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    let mut u = v.unsigned_abs();
    loop {
        at -= 1;
        digits[at] = b'0' + (u % 10) as u8;
        u /= 10;
        if u == 0 {
            break;
        }
    }
    if v < 0 {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(&digits[at..]).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::number;

    /// The text of a number as `number_text` wrote it before the integer shortcut.
    fn written(x: f64) -> String {
        if !x.is_finite() || x.trunc() == x {
            number::to_string(x)
        } else {
            number::to_string_precision(x, 12)
        }
    }

    /// The grammar as it was checked before `read_number` (the old `numeric`).
    fn numeric(s: &str) -> bool {
        let b = s.as_bytes();
        let mut i = 0;
        let digits = |i: &mut usize| {
            let start = *i;
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
            *i - start
        };
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let whole = digits(&mut i);
        if whole > 0 {
            if i < b.len() && b[i] == b'.' {
                i += 1;
                digits(&mut i);
            }
        } else {
            if !(i < b.len() && b[i] == b'.') {
                return false;
            }
            i += 1;
            if digits(&mut i) == 0 {
                return false;
            }
        }
        if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
            i += 1;
            if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
                i += 1;
            }
            if digits(&mut i) == 0 {
                return false;
            }
        }
        i == b.len()
    }

    #[test]
    fn numbers_read_as_the_parser_reads_them() {
        let mut seed = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let digits = |n: u64, next: &mut dyn FnMut() -> u64| -> String {
            (0..n)
                .map(|_| char::from(b'0' + (next() % 10) as u8))
                .collect()
        };
        let mut texts: Vec<String> = [
            "0",
            "-0",
            "+0",
            "5.",
            ".5",
            "+.5",
            "-.5e-3",
            "1e400",
            "-1e400",
            "1e-400",
            "0x10",
            "1,5",
            ".",
            "-",
            "+",
            "e5",
            "1e",
            "1e+",
            "12abc",
            "0.000001234",
            "99.995",
            "1.005",
            "598.50",
            "1234567890.125",
            "9007199254740993",
            "123456789012345678901234567890",
            "0.1e1",
            "1E21",
            "4.9e-324",
            "2.2250738585072014e-308",
            "1.7976931348623157e308",
            "inf",
            "NaN",
        ]
        .map(String::from)
        .to_vec();
        for _ in 0..100_000 {
            let r = next();
            let mut t = String::new();
            match r % 3 {
                0 => t.push('-'),
                1 => t.push('+'),
                _ => {}
            }
            t += &digits(next() % 21, &mut next);
            if next() % 2 == 0 {
                t.push('.');
                t += &digits(next() % 21, &mut next);
            }
            if next() % 3 == 0 {
                t.push(if next() % 2 == 0 { 'e' } else { 'E' });
                match next() % 3 {
                    0 => t.push('-'),
                    1 => t.push('+'),
                    _ => {}
                }
                t += &digits(next() % 4, &mut next);
            }
            texts.push(t);
        }
        for t in texts {
            let old: Option<f64> = if numeric(&t) { t.parse().ok() } else { None };
            assert_eq!(
                super::number(&t).map(f64::to_bits),
                old.map(f64::to_bits),
                "{t:?}"
            );
        }
    }

    #[test]
    fn numbers_write_as_javascript_does() {
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut xs = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            9.0,
            10.0,
            99.0,
            1e15 - 1.0,
            1e15,
            -1e15 + 1.0,
            1e21,
            2e53,
            0.1 + 0.2,
            1.0 / 3.0,
            -2.5,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            5e-324,
        ];
        for _ in 0..20_000 {
            let r = next();
            xs.push(match r % 4 {
                0 => (r >> 11) as f64 % 1e16 * if r & 1 == 0 { 1.0 } else { -1.0 },
                1 => f64::from_bits(r),
                2 => (r % 2_000_001) as f64 - 1e6,
                _ => (r >> 11) as f64 / (1u64 << 53) as f64 * 1e4,
            });
        }
        for x in xs {
            let mut out = String::from(">");
            push_number_text(&mut out, x);
            assert_eq!(out, format!(">{}", written(x)), "{x:e}");
        }
    }
}
