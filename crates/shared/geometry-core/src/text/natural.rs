//! Doğal sıra (docs/adr/0153 §6): how the point editor sorts names and codes
//! on both platforms, so that P2 comes before P10 and 101/2 before 101/10.
//! The independent reference is `scripts/fixtures/point_editor_cases.py`.
//!
//! A text is read as tokens: each run of ASCII digits is a number, every
//! other character a token of its own. Two texts compare token by token:
//!
//! 1. two numbers by their value (leading zeros do not count);
//! 2. a number before any other character;
//! 3. two characters, folded the Turkish way (I is ı's, İ is i's): first
//!    what is not a letter (by code point), then letters in the Turkish
//!    alphabet's order (q, w and x in their Latin places), then other
//!    letters by code point;
//! 4. the text with fewer tokens first;
//! 5. last, the texts as they are, by code point (`A` before `a`, `01`
//!    before `1`): no two different texts are equal.

use std::cmp::Ordering;

use super::edit::fold;

/// The letters in their order: the Turkish alphabet with q, w and x where Latin has them.
const ALPHABET: [char; 32] = [
    'a', 'b', 'c', 'ç', 'd', 'e', 'f', 'g', 'ğ', 'h', 'ı', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'ö',
    'p', 'q', 'r', 's', 'ş', 't', 'u', 'ü', 'v', 'w', 'x', 'y', 'z',
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token<'a> {
    Number(&'a str),
    Char(char),
}

fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c.is_ascii_digit() {
            let end = rest
                .find(|d: char| !d.is_ascii_digit())
                .unwrap_or(rest.len());
            out.push(Token::Number(&rest[..end]));
            rest = &rest[end..];
        } else {
            out.push(Token::Char(c));
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

/// Two runs of digits by their value.
fn by_value(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.trim_start_matches('0'), b.trim_start_matches('0'));
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

/// Where a character sorts: what is not a letter, then the alphabet, then other letters.
fn rank(c: char) -> (u8, u32) {
    let f = fold(c);
    if !f.is_alphabetic() {
        return (0, f as u32);
    }
    match ALPHABET.iter().position(|&a| a == f) {
        Some(i) => (1, i as u32),
        None => (2, f as u32),
    }
}

/// `a` against `b` in the natural order.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (ta, tb) = (tokens(a), tokens(b));
    for (x, y) in ta.iter().zip(&tb) {
        let order = match (x, y) {
            (Token::Number(p), Token::Number(q)) => by_value(p, q),
            (Token::Number(_), Token::Char(_)) => Ordering::Less,
            (Token::Char(_), Token::Number(_)) => Ordering::Greater,
            (Token::Char(c), Token::Char(d)) => rank(*c).cmp(&rank(*d)),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    ta.len().cmp(&tb.len()).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(list: &[&str]) -> Vec<String> {
        let mut v: Vec<&str> = list.to_vec();
        v.sort_by(|a, b| natural_cmp(a, b));
        v.into_iter().map(str::to_owned).collect()
    }

    #[test]
    fn numbers_by_their_value() {
        assert_eq!(sorted(&["P10", "P2", "P1"]), ["P1", "P2", "P10"]);
        assert_eq!(
            sorted(&["101/10", "101/2", "101", "101/1"]),
            ["101", "101/1", "101/2", "101/10"]
        );
        assert_eq!(sorted(&["A-10", "A-009", "A-9"]), ["A-009", "A-9", "A-10"]);
        // Equal values: the texts as they are.
        assert_eq!(sorted(&["1", "001", "01"]), ["001", "01", "1"]);
    }

    #[test]
    fn letters_in_the_turkish_order_caseless() {
        assert_eq!(
            sorted(&["Kuzey", "köşe", "Kale", "Köşe"]),
            ["Kale", "Köşe", "köşe", "Kuzey"]
        );
        // I and ı fold to ı, İ and i to i; then the texts by code point.
        assert_eq!(sorted(&["i", "I", "ı", "İ"]), ["I", "ı", "i", "İ"]);
        assert_eq!(sorted(&["çam", "dal", "can"]), ["can", "çam", "dal"]);
    }

    #[test]
    fn numbers_before_marks_before_letters() {
        assert_eq!(sorted(&["PA", "P-1", "P1"]), ["P1", "P-1", "PA"]);
        assert_eq!(sorted(&["a", "", "1"]), ["", "1", "a"]);
    }
}
