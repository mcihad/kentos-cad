//! The expression language's text order: ICU's Turkish collation
//! (`localeCompare(b, 'tr')` in the TypeScript) as a fixed table, so the
//! order is the same in every browser and on the server. Text compares by
//! its letters first (a < b < c < ç … ı < i … z, punctuation and digits
//! before letters), then by accents, then by case (lower first); characters
//! with no weight (zero-width, controls) are skipped. The table
//! (`collation_tr.rs`) is written from ICU by
//! `apps/web/scripts/fixtures/record-collation.test.ts`; a character it
//! leaves out or does not cover orders by code point after every letter.

use std::cmp::Ordering;

use super::collation_tr::TABLE;

/// First letter weight of a character the table does not know.
const UNKNOWN: u32 = 0x10_0000;

/// A character's weights (letter, accent, case); None when it has none.
fn weights(c: char) -> Option<(u32, u16, u16)> {
    let cp = u32::from(c);
    let found = u16::try_from(cp)
        .ok()
        .and_then(|k| TABLE.binary_search_by_key(&k, |e| e.0).ok());
    match found {
        Some(i) => {
            let (_, p, s, t) = TABLE[i];
            (p != 0).then_some((u32::from(p), u16::from(s), u16::from(t)))
        }
        None => Some((UNKNOWN + cp, 0, 0)),
    }
}

/// `a.localeCompare(b, 'tr')`: letters over the whole text first, then accents, then case.
pub fn compare_tr(a: &str, b: &str) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    let ka: Vec<(u32, u16, u16)> = a.chars().filter_map(weights).collect();
    let kb: Vec<(u32, u16, u16)> = b.chars().filter_map(weights).collect();
    let level = |f: fn(&(u32, u16, u16)) -> u32| ka.iter().map(f).cmp(kb.iter().map(f));
    level(|w| w.0)
        .then_with(|| level(|w| u32::from(w.1)))
        .then_with(|| level(|w| u32::from(w.2)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ordering::*;

    #[test]
    fn turkish_letters_accents_and_case() {
        let order = |a: &str, b: &str| compare_tr(a, b);
        assert_eq!(order("Arsa", "Bahçe"), Less);
        assert_eq!(order("c", "ç"), Less);
        assert_eq!(order("ç", "d"), Less);
        assert_eq!(order("ı", "i"), Less);
        assert_eq!(order("I", "ı"), Greater);
        assert_eq!(order("i", "İ"), Less);
        assert_eq!(order("a", "A"), Less);
        assert_eq!(order("é", "e"), Greater);
        assert_eq!(order("10", "9"), Less);
        assert_eq!(order("a\u{200b}", "a"), Equal);
        assert_eq!(order(" x", "!x"), Less);
        assert_eq!(order("éa", "eb"), Less);
        // A character the table leaves out orders after every letter.
        assert_eq!(order("ß", "z"), Greater);
    }
}
