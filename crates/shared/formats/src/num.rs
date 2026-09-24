//! Numbers as files write them. Reading gives the float64 nearest to the
//! decimal in the file (Rust's parser is correctly rounded); writing gives
//! the shortest decimal that reads back to the same float64, so a value
//! survives any number of write-read cycles bit for bit (CLAUDE.md §23).

/// The shortest decimal that reads back to `v`, without an exponent
/// ("452345.123"). Negative zero is written as "0".
pub fn plain(v: f64) -> String {
    if v == 0.0 {
        "0".to_string()
    } else {
        format!("{v}")
    }
}

/// A DXF real: the shortest round-trip decimal, positional for ordinary
/// magnitudes and with an exponent otherwise ("6.123233995736766E-17",
/// "1.0E+20"), always with a decimal point as AutoCAD writes them.
pub fn dxf_real(v: f64) -> String {
    let a = v.abs();
    if v == 0.0 {
        return "0.0".to_string();
    }
    if (1e-6..1e15).contains(&a) {
        let s = format!("{v}");
        return if s.contains('.') { s } else { format!("{s}.0") };
    }
    let s = format!("{v:e}");
    let (mantissa, exp) = s.split_once('e').unwrap_or((s.as_str(), "0"));
    let mantissa = if mantissa.contains('.') {
        mantissa.to_string()
    } else {
        format!("{mantissa}.0")
    };
    let (sign, digits) = exp.strip_prefix('-').map_or(("+", exp), |d| ("-", d));
    format!("{mantissa}E{sign}{digits:0>2}")
}

/// A real as DXF and most tools write it (surrounding spaces allowed); None
/// when it is not a finite number ("nan" and "inf" are refused).
pub fn parse_real(s: &str) -> Option<f64> {
    let v: f64 = s.trim().parse().ok()?;
    v.is_finite().then_some(v)
}

/// An integer group value ("  70" → 70); DXF writers sometimes add spaces.
pub fn parse_int(s: &str) -> Option<i64> {
    let t = s.trim();
    t.parse::<i64>().ok().or_else(|| {
        // A few writers put integers in real notation ("1.0").
        let v: f64 = t.parse().ok()?;
        (v.is_finite() && v.fract() == 0.0 && v.abs() < 9.0e15).then_some(v as i64)
    })
}

/// A decimal written by a person or a spreadsheet: optional sign, digits,
/// one decimal mark (`comma`: the Turkish decimal comma instead of the
/// point), optional exponent. Grouping marks and anything else are refused
/// rather than guessed.
pub fn parse_decimal(s: &str, comma: bool) -> Option<f64> {
    let t = s.trim();
    let mark = if comma { b',' } else { b'.' };
    let bytes = t.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        i = 1;
    }
    let mut digits = 0;
    let mut marks = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'0'..=b'9' => digits += 1,
            b if b == mark => marks += 1,
            b'e' | b'E' => break,
            _ => return None,
        }
        i += 1;
    }
    if digits == 0 || marks > 1 {
        return None;
    }
    if i < bytes.len() {
        // Exponent: e, optional sign, at least one digit.
        let mut j = i + 1;
        if matches!(bytes.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        if j >= bytes.len() || !bytes[j..].iter().all(u8::is_ascii_digit) {
            return None;
        }
    }
    let v: f64 = if comma {
        t.replace(',', ".").parse().ok()?
    } else {
        t.parse().ok()?
    };
    v.is_finite().then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortest_decimals_read_back_bit_for_bit() {
        for v in [
            452345.123,
            4412345.678,
            0.1 + 0.2,
            1.0 / 3.0,
            -1e-300,
            6.123233995736766e-17,
            1e20,
            5.0,
            -0.5,
            4.4e6,
        ] {
            assert_eq!(
                parse_real(&plain(v)).map(f64::to_bits),
                Some(v.to_bits()),
                "{v}"
            );
            assert_eq!(
                parse_real(&dxf_real(v)).map(f64::to_bits),
                Some(v.to_bits()),
                "{v}"
            );
        }
        assert_eq!(plain(452345.123), "452345.123");
        assert_eq!(plain(-0.0), "0");
        assert_eq!(dxf_real(5.0), "5.0");
        assert_eq!(dxf_real(1e20), "1.0E+20");
        assert_eq!(dxf_real(6.123233995736766e-17), "6.123233995736766E-17");
        assert_eq!(dxf_real(-2.5e-9), "-2.5E-09");
        assert_eq!(dxf_real(0.0), "0.0");
    }

    #[test]
    fn dxf_reals_refuse_what_is_not_a_finite_number() {
        assert_eq!(parse_real("  12.5 "), Some(12.5));
        assert_eq!(parse_real("1.0E+20"), Some(1e20));
        assert_eq!(parse_real(".5"), Some(0.5));
        for bad in ["", "nan", "inf", "-infinity", "1,5", "abc", "1e999"] {
            assert_eq!(parse_real(bad), None, "{bad}");
        }
        assert_eq!(parse_int(" 70"), Some(70));
        assert_eq!(parse_int("1.0"), Some(1));
        assert_eq!(parse_int("1.5"), None);
    }

    #[test]
    fn decimals_follow_the_chosen_mark_and_nothing_else() {
        assert_eq!(parse_decimal("452345.123", false), Some(452345.123));
        assert_eq!(parse_decimal("452345,123", true), Some(452345.123));
        assert_eq!(parse_decimal("-12", false), Some(-12.0));
        assert_eq!(parse_decimal("+1.5e3", false), Some(1500.0));
        assert_eq!(parse_decimal(" 7 ", false), Some(7.0));
        for (bad, comma) in [
            ("452345,123", false),
            ("452345.123", true),
            ("1.234,5", true),
            ("1 234", false),
            ("1.2.3", false),
            ("e5", false),
            ("1e", false),
            ("abc", false),
            ("", false),
            ("-", false),
            ("NaN", false),
        ] {
            assert_eq!(parse_decimal(bad, comma), None, "{bad} comma={comma}");
        }
    }
}
