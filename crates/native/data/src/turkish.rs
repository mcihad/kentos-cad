//! Texts in the Turkish alphabet's order (docs/adr/9102 §3): the order a text column is sorted by in every driver,
//! whatever the source's collation. `a b c ç d e f g ğ h ı i j k l m n o ö p q r s ş t u ü v w x y z`; digits before
//! letters; a circumflexed vowel with its plain one (`â` with `a`); a word and its capitalised self together, the small
//! one first (`ağaç`, `Ağaç`). Any other character comes after the letters, by its code. A database whose own collation
//! is the Turkish one (PostgreSQL's `tr-x-icu`) is asked for it; this is the rule it is checked against.

use std::cmp::Ordering;

/// The Turkish alphabet, small letters, in order; `q`, `w` and `x` where the Latin alphabet has them.
const ALPHABET: &str = "abcçdefgğhıijklmnoöpqrsştuüvwxyz";

/// The circumflexed vowels and their plain ones.
const CIRCUMFLEX: [(char, char); 6] = [
    ('â', 'a'),
    ('Â', 'a'),
    ('î', 'i'),
    ('Î', 'i'),
    ('û', 'u'),
    ('Û', 'u'),
];

/// A letter's small plain form: `İ` is `i`, `I` is `ı`, `Â` is `a`.
fn plain(c: char) -> char {
    if let Some((_, p)) = CIRCUMFLEX.iter().find(|(x, _)| *x == c) {
        return *p;
    }
    match c {
        'İ' => 'i',
        'I' => 'ı',
        c => c.to_lowercase().next().unwrap_or(c),
    }
}

/// A character's place at the first level: digits, then the letters, then the rest by code.
fn primary(c: char) -> (u8, u32) {
    if c.is_ascii_digit() {
        return (0, c as u32);
    }
    let p = plain(c);
    match ALPHABET.chars().position(|a| a == p) {
        Some(at) => (1, at as u32),
        None => (2, p as u32),
    }
}

/// `a` against `b` in the Turkish alphabet's order: letter by letter at the first level; then a plain vowel before
/// its circumflexed one; then a small letter before its capital; then by code.
pub fn compare(a: &str, b: &str) -> Ordering {
    let circumflexed = |c: char| CIRCUMFLEX.iter().any(|(x, _)| *x == c);
    a.chars()
        .map(primary)
        .cmp(b.chars().map(primary))
        .then_with(|| a.chars().map(circumflexed).cmp(b.chars().map(circumflexed)))
        .then_with(|| {
            a.chars()
                .map(char::is_uppercase)
                .cmp(b.chars().map(char::is_uppercase))
        })
        .then_with(|| a.cmp(b))
}

/// SQL's `translate` arguments that order a text as [`compare`]'s first level does where no Turkish collation is at
/// hand: each letter (small, capital or circumflexed) written as one byte of its place from `A` on, compared byte by
/// byte (`COLLATE "C"`). Digits stay below the letters, any other character above them.
pub fn sort_key_translation() -> (String, String) {
    let mut from = String::new();
    let mut to = String::new();
    for (at, small) in ALPHABET.chars().enumerate() {
        let place = char::from(b'A' + at as u8);
        let capital = match small {
            'i' => 'İ',
            'ı' => 'I',
            c => c.to_uppercase().next().unwrap_or(c),
        };
        from.push(small);
        to.push(place);
        from.push(capital);
        to.push(place);
        for (c, p) in CIRCUMFLEX {
            if p == small {
                from.push(c);
                to.push(place);
            }
        }
    }
    (from, to)
}

/// The bytes a source without a Turkish collation (SQLite) orders a text by, byte by byte, as [`compare`] orders it:
/// the first level's places (a digit, a letter, any other character as a mark and its UTF-8), then the circumflexes,
/// then the capitals, then the text itself; each level ends with a zero byte, so a word comes before its longer self.
pub fn sort_key(text: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(text.len() * 4 + 3);
    for c in text.chars() {
        match primary(c) {
            (0, d) => key.push(0x10 + (d - u32::from('0')) as u8),
            (1, at) => key.push(0x30 + at as u8),
            (_, code) => {
                key.push(0x80);
                let mut b = [0u8; 4];
                key.extend_from_slice(
                    char::from_u32(code)
                        .unwrap_or(c)
                        .encode_utf8(&mut b)
                        .as_bytes(),
                );
            }
        }
    }
    key.push(0);
    key.extend(text.chars().map(|c| {
        if CIRCUMFLEX.iter().any(|(x, _)| *x == c) {
            2
        } else {
            1
        }
    }));
    key.push(0);
    key.extend(text.chars().map(|c| if c.is_uppercase() { 2 } else { 1 }));
    key.push(0);
    key.extend_from_slice(text.as_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_sort_in_the_turkish_alphabet_s_order() {
        let mut w = vec![
            "Zafer",
            "Çınar",
            "cami",
            "Ceylan",
            "ırmak",
            "İnci",
            "ilçe",
            "Şehir",
            "Su",
            "Öğretmen",
            "Okul",
            "Ünal",
            "Uzun",
            "Ağaç",
            "ağaç",
            "2. Cadde",
            "10. Sokak",
            "âlem",
            "alan",
        ];
        w.sort_by(|a, b| compare(a, b));
        assert_eq!(
            w,
            [
                "10. Sokak",
                "2. Cadde",
                "ağaç",
                "Ağaç",
                "alan",
                "âlem",
                "cami",
                "Ceylan",
                "Çınar",
                "ırmak",
                "ilçe",
                "İnci",
                "Okul",
                "Öğretmen",
                "Su",
                "Şehir",
                "Uzun",
                "Ünal",
                "Zafer",
            ]
        );
    }

    #[test]
    fn the_sort_key_orders_as_compare_does() {
        let words = [
            "Zafer",
            "Çınar",
            "cami",
            "Ceylan",
            "ırmak",
            "İnci",
            "ilçe",
            "Şehir",
            "Su",
            "Öğretmen",
            "Okul",
            "Ünal",
            "Uzun",
            "Ağaç",
            "ağaç",
            "2. Cadde",
            "10. Sokak",
            "âlem",
            "alan",
            "alem",
            "Su yolu",
            "su",
            "émile",
            "",
        ];
        for a in words {
            for b in words {
                assert_eq!(sort_key(a).cmp(&sort_key(b)), compare(a, b), "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn the_translation_keeps_the_first_level() {
        let (from, to) = sort_key_translation();
        let from: Vec<char> = from.chars().collect();
        let to: Vec<char> = to.chars().collect();
        assert_eq!(from.len(), to.len());
        let key = |s: &str| -> String {
            s.chars()
                .map(|c| from.iter().position(|f| *f == c).map_or(c, |i| to[i]))
                .collect()
        };
        for (a, b) in [
            ("cami", "Çınar"),
            ("ırmak", "ilçe"),
            ("Okul", "Öğretmen"),
            ("Su", "Şehir"),
            ("Uzun", "Ünal"),
            ("2. Cadde", "ağaç"),
            ("Zafer", "émile"),
        ] {
            assert!(key(a) < key(b), "{a} < {b}");
        }
        assert_eq!(key("İNCİ"), key("inci"));
        assert_eq!(key("IRMAK"), key("ırmak"));
        assert_eq!(key("âlem"), key("alem"));
    }
}
