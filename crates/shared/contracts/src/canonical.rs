//! Canonical JSON and the digests made of it (docs/adr/0253 §2, §5, §6):
//! what a state stamp, a plan's id and a request's fingerprint are computed
//! over, the same on every host and in the independent reference
//! (`scripts/fixtures/protocol_cases.py`). A fingerprint is kept in the
//! `.kcad` file (its request ledger), so the text is defined here, not left
//! to a library's version:
//!
//! - no white space; an object's keys sorted by their UTF-8 bytes;
//! - a string escaped as JSON requires (`"`, `\`, the control characters:
//!   `\b \f \n \r \t`, the others `\u00XX`), nothing else;
//! - an integer as its digits, with `-` when negative;
//! - a float by its shortest digits that read back the same number; of two
//!   such equally near it, the one whose last digit is even (ECMAScript's
//!   `Number.prototype.toString`, CPython's `repr` and serde_json choose the
//!   same; the standard library's `{:e}` rounds such a tie up). In decimal
//!   notation when 1e-5 ≤ |x| < 1e16 (`0.00001`, `12345.678`, a whole
//!   number keeping its `.0`), otherwise with an exponent signed either way
//!   (`1e-6`, `1.5e-7`, `1e+16`, `2.5e+16`); zero is `0.0` or `-0.0`. JSON
//!   has no NaN or infinity; they are written `null`.

use serde_json::{Number, Value};

/// `value` as canonical JSON.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

fn write(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => number(n, out),
        Value::String(s) => string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            out.push('{');
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(key, out);
                out.push(':');
                if let Some(v) = map.get(key) {
                    write(v, out);
                }
            }
            out.push('}');
        }
    }
}

fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn number(n: &Number, out: &mut String) {
    if let Some(u) = n.as_u64() {
        out.push_str(&u.to_string());
    } else if let Some(i) = n.as_i64() {
        out.push_str(&i.to_string());
    } else if let Some(f) = n.as_f64() {
        float(f, out);
    } else {
        out.push_str("null");
    }
}

/// A float by the rule above, from Rust's shortest round-trip digits.
fn float(x: f64, out: &mut String) {
    if !x.is_finite() {
        out.push_str("null");
        return;
    }
    if x.is_sign_negative() {
        out.push('-');
    }
    if x == 0.0 {
        out.push_str("0.0");
        return;
    }
    let (digits, exp) = shortest(x.abs());
    // Where the point goes: after `point` digits (zero or less: before them, with zeros).
    let point = exp + 1;
    if (-4..=16).contains(&point) {
        let len = i32::try_from(digits.len()).unwrap_or(i32::MAX);
        if point <= 0 {
            out.push_str("0.");
            for _ in 0..-point {
                out.push('0');
            }
            out.push_str(&digits);
        } else if point >= len {
            out.push_str(&digits);
            for _ in 0..point - len {
                out.push('0');
            }
            out.push_str(".0");
        } else {
            let (whole, frac) = digits.split_at(usize::try_from(point).unwrap_or(0));
            out.push_str(whole);
            out.push('.');
            out.push_str(frac);
        }
    } else {
        let (first, rest) = digits.split_at(1);
        out.push_str(first);
        if !rest.is_empty() {
            out.push('.');
            out.push_str(rest);
        }
        out.push('e');
        if exp > 0 {
            out.push('+');
        }
        out.push_str(&exp.to_string());
    }
}

/// The shortest digits of `x` > 0 that read back as `x`, the first before
/// the point, and the decimal exponent: `x` = d.ddd × 10^exp. Of two equally
/// near, the even one. `{:e}` gives the shortest digits but breaks such a
/// tie upwards; `{:.N$e}` rounds exactly with ties to even, and is taken
/// when it reads back as `x` (at a power of two the even neighbour may not).
/// Two candidates can be equally near only with 16 digits or more: shorter
/// decimals are further apart than two doubles.
fn shortest(x: f64) -> (String, i32) {
    let split = |text: &str| -> (String, i32) {
        let (mantissa, exp) = text.split_once('e').unwrap_or((text, "0"));
        let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
        let digits = digits.trim_end_matches('0');
        let digits = if digits.is_empty() { "0" } else { digits };
        (digits.to_owned(), exp.parse().unwrap_or(0))
    };
    let (digits, exp) = split(&format!("{x:e}"));
    if digits.len() >= 16 {
        let even = format!("{:.*e}", digits.len() - 1, x);
        if even.parse::<f64>() == Ok(x) {
            return split(&even);
        }
    }
    (digits, exp)
}

/// The SHA-256 of `value`'s canonical JSON, its first `hex` lowercase hex
/// digits (at most 64).
pub fn digest_of(value: &Value, hex: usize) -> String {
    let mut full = crate::encoding::sha256_hex(canonical_json(value).as_bytes());
    full.truncate(hex.min(64));
    full
}

/// A state stamp (docs/adr/0253 §2): 16 hex digits of the SHA-256 of the
/// canonical JSON.
pub fn stamp_of(value: &Value) -> String {
    digest_of(value, 16)
}

/// A plan's id or a request's fingerprint (docs/adr/0253 §5, §6): 32 hex digits.
pub fn id_of(value: &Value) -> String {
    digest_of(value, 32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_are_sorted_and_strings_escaped() {
        let v = json!({ "b": 1, "a": [true, -3], "ç": "ş\"\n\u{1}", "A": null });
        assert_eq!(
            canonical_json(&v),
            r#"{"A":null,"a":[true,-3],"b":1,"ç":"ş\"\n\u0001"}"#
        );
    }

    #[test]
    fn floats_are_written_by_the_rule() {
        let cases = [
            (1.0, "1.0"),
            (0.1, "0.1"),
            (-2.5, "-2.5"),
            (1e-4, "0.0001"),
            (1e-5, "0.00001"),
            (1.2345e-5, "0.000012345"),
            (1e-6, "1e-6"),
            (1.5e-7, "1.5e-7"),
            (12345.678, "12345.678"),
            (1e15, "1000000000000000.0"),
            (9_999_999_999_999_998.0, "9999999999999998.0"),
            (1e16, "1e+16"),
            (2.5e16, "2.5e+16"),
            (123_456_789_012_345_680.0, "1.2345678901234568e+17"),
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
            (f64::MAX, "1.7976931348623157e+308"),
        ];
        for (x, want) in cases {
            assert_eq!(canonical_json(&json!(x)), want, "{x:e}");
        }
    }

    /// Two shortest candidates equally near (…870.25 exactly): the even one,
    /// as ECMAScript, CPython and serde_json write it; the standard library's
    /// `{:e}` writes the odd one. At 2⁻²⁴ the even neighbour does not read
    /// back: the other stays.
    #[test]
    fn a_tie_is_written_even() {
        let tie = f64::from_bits(0x4308_af85_ed01_7672);
        assert_eq!(canonical_json(&json!(tie)), "868548647857870.2");
        assert_eq!(
            serde_json::to_string(&tie).expect("writes"),
            "868548647857870.2"
        );
        assert_eq!(format!("{tie:e}"), "8.685486478578703e14");
        assert_eq!(canonical_json(&json!(-tie)), "-868548647857870.2");
        assert_eq!(
            canonical_json(&json!(2f64.powi(-24))),
            "5.960464477539063e-8"
        );
    }

    /// Apart from ties, the rule is serde_json's own today.
    #[test]
    fn the_rule_agrees_with_serde_json_now() {
        for x in [0.1, 1e-5, 1e-6, 3.0, 1e15, 1e16, 6.02e23, -7.25e-9, 123.456] {
            assert_eq!(
                canonical_json(&json!(x)),
                serde_json::to_string(&x).expect("writes"),
                "{x:e}"
            );
        }
    }

    #[test]
    fn the_same_value_has_the_same_stamp_whatever_its_keys_order() {
        let a: Value = serde_json::from_str(r#"{"x":1,"y":{"q":2,"p":3}}"#).expect("json");
        let b: Value = serde_json::from_str(r#"{"y":{"p":3,"q":2},"x":1}"#).expect("json");
        assert_eq!(stamp_of(&a), stamp_of(&b));
        assert_eq!(stamp_of(&a).len(), 16);
        assert_eq!(id_of(&a).len(), 32);
        assert_ne!(
            stamp_of(&a),
            stamp_of(&json!({"x": 2, "y": {"p": 3, "q": 2}}))
        );
    }
}
