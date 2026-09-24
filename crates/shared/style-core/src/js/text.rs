//! JavaScript's text semantics where the expression language shows them:
//! what `\s` and `trim()` take for white space, lengths and positions in
//! UTF-16 code units, and the Turkish locale's upper and lower case
//! (`toLocaleUpperCase('tr-TR')`). A slice that would split a surrogate
//! pair (an emoji cut in half) cannot be Rust text: its halves become
//! U+FFFD, where JavaScript keeps a lone surrogate.

/// JavaScript's `\s` and `trim()`: WhiteSpace and LineTerminator.
pub fn is_space(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{a}' | '\u{b}' | '\u{c}' | '\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `s.trim()`.
pub fn trim(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// `s.toLocaleUpperCase('tr-TR')`: i → İ, then the default mapping (ı → I, ß → SS).
pub fn upper_tr(s: &str) -> String {
    if s.is_ascii() && !s.contains('i') {
        return s.to_ascii_uppercase();
    }
    s.chars()
        .map(|c| if c == 'i' { 'İ' } else { c })
        .collect::<String>()
        .to_uppercase()
}

/// `s.toLocaleLowerCase('tr-TR')`: I → ı, İ → i, I with a combining dot above
/// → i; then the default mapping (with the final-sigma rule).
pub fn lower_tr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'I' if chars.peek() == Some(&'\u{307}') => {
                chars.next();
                out.push('i');
            }
            'I' => out.push('ı'),
            'İ' => out.push('i'),
            c => out.push(c),
        }
    }
    out.to_lowercase()
}

/// Turkish-insensitive form (`core/text.ts` `foldTurkish`): trimmed, upper
/// case in the Turkish locale, the dotted and cedilla letters as plain ones.
pub fn fold_turkish(s: &str) -> String {
    upper_tr(trim(s))
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

/// `s.length`: UTF-16 code units. Each character is one unit, and one
/// outside the Basic Multilingual Plane (four bytes in UTF-8) two: counted
/// from the lead bytes.
pub fn utf16_len(s: &str) -> usize {
    s.bytes()
        .map(|b| usize::from(b & 0xc0 != 0x80) + usize::from(b >= 0xf0))
        .sum()
}

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn text(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

/// `s.slice(start, end)` with `start` and `end` already clamped to 0…length
/// by the caller's arithmetic (a start past the end is empty).
pub fn slice(s: &str, start: usize, end: Option<usize>) -> String {
    let u = units(s);
    let end = end.unwrap_or(u.len()).min(u.len());
    if start >= end {
        return String::new();
    }
    text(&u[start..end])
}

/// The first code unit of `s`, as `s.slice(0, 1)` gives it (empty for "").
pub fn first_unit(s: &str) -> Option<u16> {
    s.encode_utf16().next()
}

/// V8's longest string, in code units: longer results throw (RangeError).
pub const MAX_STRING_UNITS: usize = (1 << 29) - 24;

/// `s.padStart(target, fill)` with a one-unit fill; None where JavaScript
/// throws (the result would pass V8's longest string).
pub fn pad_start(s: &str, target: usize, fill: u16) -> Option<String> {
    let n = utf16_len(s);
    if target <= n {
        return Some(s.to_string());
    }
    if target > MAX_STRING_UNITS {
        return None;
    }
    if let Some(c) = char::from_u32(u32::from(fill)) {
        // A fill that is a whole character: no detour through UTF-16.
        let mut out = String::with_capacity((target - n) * c.len_utf8() + s.len());
        out.extend(std::iter::repeat_n(c, target - n));
        out.push_str(s);
        return Some(out);
    }
    let mut u = vec![fill; target - n];
    u.extend(s.encode_utf16());
    Some(text(&u))
}

/// `pad_start` on text the caller owns: a short fill of whole characters
/// goes in front of it in place (a number's text has room for it).
pub fn pad_start_owned(mut s: String, target: usize, fill: u16) -> Option<String> {
    let n = utf16_len(&s);
    let Some(c) = char::from_u32(u32::from(fill)) else {
        return pad_start(&s, target, fill);
    };
    let mut buf = [0u8; 64];
    let width = c.len_utf8();
    let count = target.saturating_sub(n);
    if count == 0 || count * width > buf.len() {
        return pad_start(&s, target, fill);
    }
    for i in 0..count {
        c.encode_utf8(&mut buf[i * width..]);
    }
    s.insert_str(
        0,
        std::str::from_utf8(&buf[..count * width]).unwrap_or_default(),
    );
    Some(s)
}

/// `s.split(a).join(b)`: every occurrence of `a` replaced, from the left;
/// an empty `a` splits between code units.
pub fn split_join(s: &str, a: &str, b: &str) -> Option<String> {
    if !a.is_empty() {
        return Some(s.replace(a, b));
    }
    let u = units(s);
    let bu = units(b);
    let len = u.len() + u.len().saturating_sub(1) * bu.len();
    if len > MAX_STRING_UNITS {
        return None;
    }
    let mut out = Vec::with_capacity(len);
    for (i, &c) in u.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(&bu);
        }
        out.push(c);
    }
    Some(text(&out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_case_and_folding() {
        assert_eq!(upper_tr("kadıköy"), "KADIKÖY");
        assert_eq!(upper_tr("istanbul ß"), "İSTANBUL SS");
        assert_eq!(lower_tr("IŞIK"), "ışık");
        assert_eq!(lower_tr("İZMİR"), "izmir");
        assert_eq!(lower_tr("I\u{307}"), "i");
        assert_eq!(lower_tr("ΟΔΟΣ"), "οδος");
        assert_eq!(fold_turkish("  Çizgi\u{feff}"), "CIZGI");
        assert_eq!(fold_turkish("ÇİZGİ"), fold_turkish("cizgi"));
        assert_eq!(fold_turkish("değil"), "DEGIL");
    }

    #[test]
    fn javascript_white_space() {
        assert_eq!(trim("\u{feff}\u{a0} a b \u{3000}\n"), "a b");
        // NEL (U+0085) is not white space to JavaScript.
        assert_eq!(trim("\u{85}a"), "\u{85}a");
    }

    #[test]
    fn utf16_positions() {
        assert_eq!(utf16_len("ağaç"), 4);
        assert_eq!(utf16_len("a😀"), 3);
        assert_eq!(slice("P00012", 1, Some(4)), "000");
        assert_eq!(slice("abc", 5, None), "");
        // Half an emoji is not text: it becomes U+FFFD.
        assert_eq!(slice("😀", 0, Some(1)), "\u{fffd}");
        assert_eq!(
            pad_start("12", 5, u16::from(b'0')).as_deref(),
            Some("00012")
        );
        assert_eq!(
            pad_start("12345", 3, u16::from(b'0')).as_deref(),
            Some("12345")
        );
        assert_eq!(pad_start("", MAX_STRING_UNITS + 1, 48), None);
        // In place or not, the same text: whole characters, several bytes,
        // half an emoji (U+FFFD), and more fill than fits the stack.
        for (s, target, fill) in [
            ("12", 5, u16::from(b'0')),
            ("ağ", 6, 0x011f),
            ("x", 3, 0xd83d),
            ("7", 100, u16::from(b'_')),
            ("12345", 3, u16::from(b'0')),
            ("😀", 4, u16::from(b'0')),
        ] {
            assert_eq!(
                pad_start_owned(s.to_string(), target, fill),
                pad_start(s, target, fill),
                "{s} {target}"
            );
        }
        assert_eq!(utf16_len("İşık 😀 ǅ"), 9);
        assert_eq!(split_join("1245/12", "/", "-").as_deref(), Some("1245-12"));
        assert_eq!(split_join("abc", "", "-").as_deref(), Some("a-b-c"));
        assert_eq!(split_join("", "", "-").as_deref(), Some(""));
    }
}
