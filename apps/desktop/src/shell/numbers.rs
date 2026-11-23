//! Numbers as the windows read and write them (the web's `readNumber`, `smallAngleText`, `mmText`): a typed number with
//! its first comma a decimal point, a small angle with its fine unit and a small length in millimetres. The core's:
//! CBS's calculators and Harita's windows both use them (docs/adr/9199 §4.3).

use kentos_interaction::{Format, fixed, js_trim};

/// A number as typed (the web's `readNumber`): the first comma is a decimal
/// point; empty is none; anything else not a number is NaN.
pub fn read_number(text: &str) -> Option<f64> {
    let t = js_trim(text).replacen(',', ".", 1);
    if t.is_empty() {
        return None;
    }
    Some(if is_number(&t) {
        t.parse().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    })
}

/// `^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$`, case-insensitive.
fn is_number(t: &str) -> bool {
    let b = t.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'-' | b'+')));
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let whole = digits(i);
    i += whole;
    let mut fraction = 0;
    if b.get(i) == Some(&b'.') {
        fraction = digits(i + 1);
        i += 1 + fraction;
    }
    if whole == 0 && fraction == 0 {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(b.get(i + 1), Some(b'-' | b'+')));
        let exponent = digits(i + 1 + sign);
        if exponent == 0 {
            return false;
        }
        i += 1 + sign + exponent;
    }
    i == b.len()
}

/// A small angle (a misclosure) with its fine unit: grad with cc (10⁻⁴ g),
/// degrees with seconds (the web's `smallAngleText`).
pub fn small_angle_text(format: &Format, v: f64) -> String {
    if format.angle_unit_label() == "°" {
        format!(
            "{}° ({}″)",
            unsigned(fixed(v, 5)),
            unsigned(fixed(v * 3600.0, 1))
        )
    } else {
        format!(
            "{} g ({} cc)",
            unsigned(fixed(v, 5)),
            unsigned(fixed(v * 10_000.0, 1))
        )
    }
}

/// A small length (a misclosure) in millimetres (the web's `mmText`).
pub fn mm_text(m: f64) -> String {
    format!("{} mm", unsigned(fixed(m * 1000.0, 1)))
}

/// A rounded value that came out as −0 reads as 0 (the web's `unsigned`,
/// `/^-0(\.0*)?$/`).
fn unsigned(text: String) -> String {
    let zero = text.strip_prefix("-0").is_some_and(|rest| {
        rest.is_empty()
            || rest
                .strip_prefix('.')
                .is_some_and(|d| d.bytes().all(|b| b == b'0'))
    });
    if zero { text[1..].to_owned() } else { text }
}
