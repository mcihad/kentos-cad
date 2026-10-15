//! JavaScript's `Number(text)`, as the web's style file reads numbers (docs/adr/0092).

/// `Number(text)`: blanks around ignored, empty text 0, decimal, `0x`/`0o`/`0b`
/// integers and `Infinity`; anything else NaN.
pub fn js_number(text: &str) -> f64 {
    let t = kentos_style_core::js::text::trim(text);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (prefixes, radix) in [(["0x", "0X"], 16), (["0o", "0O"], 8), (["0b", "0B"], 2)] {
        if let Some(rest) = prefixes.iter().find_map(|p| t.strip_prefix(p)) {
            if rest.is_empty() {
                return f64::NAN;
            }
            return rest
                .chars()
                .try_fold(0.0, |v, c| {
                    c.to_digit(radix)
                        .map(|d| v * f64::from(radix) + f64::from(d))
                })
                .unwrap_or(f64::NAN);
        }
    }
    // A decimal literal: [+-] digits [. digits] or . digits, then [e[+-]digits].
    let b = t.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let digits = |i: &mut usize| {
        let s = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - s
    };
    let whole = digits(&mut i);
    let mut fraction = 0;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        fraction = digits(&mut i);
    }
    if whole + fraction == 0 {
        return f64::NAN;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return f64::NAN;
        }
    }
    if i != b.len() {
        return f64::NAN;
    }
    t.parse().unwrap_or(f64::NAN)
}
