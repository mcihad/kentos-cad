//! Text rules the web's processing code gets from JavaScript and
//! `core/text.ts`: Turkish folding for search and layer ids, Turkish lower
//! case for sentences, JavaScript's `trim`, string length in UTF-16 units and
//! numbers written as `String(n)` writes them.

/// JavaScript's white space and line terminators (`String.prototype.trim`):
/// Unicode's white space without NEL, with the byte order mark.
fn js_space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}'
}

/// `s.trim()` as JavaScript trims.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(js_space)
}

/// `s.length`: UTF-16 code units.
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// `String(x)` for a number.
pub fn js_number(x: f64) -> String {
    kentos_style_core::js::number::to_string(x)
}

/// `x` with `digits` decimals by the display rule (docs/adr/0149), the
/// web's `fixed` (`core/displayNumber.ts`): what a tool writes into the
/// drawing is rounded as every shown value is.
pub fn to_fixed(x: f64, digits: u32) -> String {
    kentos_geometry_core::display::fixed(x, digits as usize)
}

/// `s.toLocaleUpperCase('tr-TR')`: i → İ, ı → I, the rest as Unicode says.
pub fn tr_upper(s: &str) -> String {
    s.chars()
        .flat_map(|c| -> Box<dyn Iterator<Item = char>> {
            match c {
                'i' => Box::new(std::iter::once('İ')),
                'ı' => Box::new(std::iter::once('I')),
                c => Box::new(c.to_uppercase()),
            }
        })
        .collect()
}

/// `s.toLocaleLowerCase('tr-TR')`: I → ı, İ → i, the rest as Unicode says.
pub fn tr_lower(s: &str) -> String {
    s.chars()
        .flat_map(|c| -> Box<dyn Iterator<Item = char>> {
            match c {
                'I' => Box::new(std::iter::once('ı')),
                'İ' => Box::new(std::iter::once('i')),
                c => Box::new(c.to_lowercase()),
            }
        })
        .collect()
}

/// `foldTurkish`: trimmed, upper case in Turkish, the Turkish letters folded
/// to ASCII ("köşe" and "KOSE" match).
pub fn fold_turkish(s: &str) -> String {
    tr_upper(js_trim(s))
        .chars()
        .map(|c| match c {
            'Ç' => 'C',
            'Ğ' => 'G',
            'İ' => 'I',
            'Ö' => 'O',
            'Ş' => 'S',
            'Ü' => 'U',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_as_the_web_does() {
        assert_eq!(fold_turkish("  Köşe noktaları "), "KOSE NOKTALARI");
        assert_eq!(fold_turkish("ışık"), "ISIK");
        assert_eq!(
            tr_lower("Tümü (Görünen Katmanlar)"),
            "tümü (görünen katmanlar)"
        );
        assert_eq!(tr_lower("KAPALI ALAN"), "kapalı alan");
        assert_eq!(js_trim("\u{FEFF} a \u{A0}"), "a");
        assert_eq!(js_trim("\u{85}a"), "\u{85}a");
        assert_eq!(utf16_len("köşe"), 4);
        assert_eq!(js_number(0.1), "0.1");
        assert_eq!(js_number(30.0), "30");
        assert_eq!(to_fixed(12.3456, 2), "12.35");
    }
}
