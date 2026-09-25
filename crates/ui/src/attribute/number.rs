//! Sayıların Türkçe yazımı ve çözümlenmesi.
//!
//! Binlik ayırıcı nokta, ondalık ayırıcı virgüldür: 15.840.900 ve 1.234,5.
//! Girişte ondalık için virgül de nokta da kabul edilir.

/// Tam sayıyı binlik ayraçla yazar: 15.840.900.
pub fn integer(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3 + 1);

    if value < 0 {
        grouped.push('-');
    }

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push('.');
        }

        grouped.push(digit);
    }

    grouped
}

/// Ondalık sayıyı verilen basamakla yazar: `real(1234.5, 2)` → "1.234,50".
pub fn real(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    let fixed = format!("{:.*}", decimals, value.abs());
    let (whole, fraction) = fixed.split_once('.').unwrap_or((&fixed, ""));
    let whole: i64 = whole.parse().unwrap_or(0);
    let sign = if value < 0.0 && fixed.chars().any(|digit| digit != '0' && digit != '.') {
        "-"
    } else {
        ""
    };

    if fraction.is_empty() {
        format!("{sign}{}", integer(whole))
    } else {
        format!("{sign}{},{fraction}", integer(whole))
    }
}

/// Tam sayıyı çözümler; binlik ayraç olarak nokta ve boşluk kabul edilir.
pub fn parse_integer(text: &str) -> Option<i64> {
    let cleaned: String = text
        .trim()
        .chars()
        .filter(|character| !matches!(character, '.' | ' ' | '\u{a0}'))
        .collect();

    if cleaned.is_empty() || cleaned.contains(',') {
        return None;
    }

    cleaned.parse().ok()
}

/// Ondalık sayıyı çözümler. Virgül varsa ondalık ayırıcıdır ve noktalar
/// binlik ayraçtır ("1.234,5"); virgül yoksa nokta ondalık ayırıcıdır.
pub fn parse_real(text: &str) -> Option<f64> {
    let trimmed: String = text
        .trim()
        .chars()
        .filter(|character| !matches!(character, ' ' | '\u{a0}'))
        .collect();

    if trimmed.is_empty() {
        return None;
    }

    let normalized = if trimmed.contains(',') {
        trimmed.replace('.', "").replace(',', ".")
    } else {
        trimmed
    };

    normalized
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

/// Adım değerinin ondalık basamak sayısı: 10 → 0, 0,5 → 1, 0,25 → 2.
pub fn decimals_of(step: f64) -> usize {
    (0..6)
        .find(|&decimals| {
            let scaled = step * 10f64.powi(decimals);
            (scaled - scaled.round()).abs() < 1e-9
        })
        .unwrap_or(6) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_are_grouped() {
        assert_eq!(integer(15_840_900), "15.840.900");
        assert_eq!(integer(999), "999");
        assert_eq!(integer(-1_000), "-1.000");
        assert_eq!(integer(0), "0");
    }

    #[test]
    fn reals_use_decimal_comma() {
        assert_eq!(real(1234.5, 2), "1.234,50");
        assert_eq!(real(0.5, 1), "0,5");
        assert_eq!(real(-12.25, 1), "-12,2");
        assert_eq!(real(1234.4, 0), "1.234");
        assert_eq!(real(-0.001, 1), "0,0");
    }

    #[test]
    fn integers_are_parsed_with_optional_grouping() {
        assert_eq!(parse_integer("15.840.900"), Some(15_840_900));
        assert_eq!(parse_integer(" 42 "), Some(42));
        assert_eq!(parse_integer("-7"), Some(-7));
        assert_eq!(parse_integer("12,5"), None);
        assert_eq!(parse_integer("abc"), None);
    }

    #[test]
    fn reals_accept_comma_or_dot() {
        assert_eq!(parse_real("12,5"), Some(12.5));
        assert_eq!(parse_real("12.5"), Some(12.5));
        assert_eq!(parse_real("1.234,5"), Some(1234.5));
        assert_eq!(parse_real("-0,25"), Some(-0.25));
        assert_eq!(parse_real(""), None);
        assert_eq!(parse_real("on iki"), None);
    }

    #[test]
    fn step_decimals() {
        assert_eq!(decimals_of(10.0), 0);
        assert_eq!(decimals_of(0.5), 1);
        assert_eq!(decimals_of(0.25), 2);
    }
}
