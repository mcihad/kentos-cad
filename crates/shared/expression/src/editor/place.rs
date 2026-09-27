//! How an entry of the builder's tree (or a field's value) goes into the
//! text over the selection: a function wraps the selected text, an operator
//! stands between spaces without doubling one already there.

use super::Kind;

/// The text after placing `insert` over `start..end` (UTF-16 units from 0;
/// equal for a bare cursor) and where the cursor goes. `caret` is the
/// cursor's place in `insert`. A function (`name()`) with text selected
/// takes that text as its argument, the cursor after it; an operator or a
/// word inserted with a space either side leaves out the one before it at
/// the start of the text or after white space, and the one after it before
/// white space.
pub fn place(
    src: &str,
    start: usize,
    end: usize,
    kind: Kind,
    insert: &str,
    caret: usize,
) -> (String, usize) {
    let u: Vec<u16> = src.encode_utf16().collect();
    let start = start.min(u.len());
    let end = end.clamp(start, u.len());
    let (before, selected, after) = (&u[..start], &u[start..end], &u[end..]);
    let mut text: Vec<u16> = insert.encode_utf16().collect();
    let mut caret = caret.min(text.len());
    let paren = |c: u8| u16::from(c);
    if kind == Kind::Function && !selected.is_empty() && text.ends_with(&[paren(b'('), paren(b')')])
    {
        text.truncate(text.len() - 1);
        text.extend_from_slice(selected);
        text.push(paren(b')'));
        caret = text.len();
    }
    let space = u16::from(b' ');
    let white = |c: Option<&u16>| {
        c.and_then(|&c| char::from_u32(u32::from(c)))
            .is_some_and(crate::js::text::is_space)
    };
    if matches!(kind, Kind::Operator | Kind::Keyword) && text.len() > 2 {
        if text.last() == Some(&space) && white(after.first()) {
            text.pop();
            caret = caret.min(text.len());
        }
        if text.first() == Some(&space) && (before.is_empty() || white(before.last())) {
            text.remove(0);
            caret = caret.saturating_sub(1);
        }
    }
    let mut out = before.to_vec();
    out.extend_from_slice(&text);
    out.extend_from_slice(after);
    (String::from_utf16_lossy(&out), start + caret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn functions_wrap_the_selection_and_operators_keep_single_spaces() {
        assert_eq!(
            place("$alan", 0, 5, Kind::Function, "yuvarla()", 8),
            ("yuvarla($alan)".into(), 14)
        );
        assert_eq!(
            place("1 + ", 4, 4, Kind::Function, "yuvarla()", 8),
            ("1 + yuvarla()".into(), 12)
        );
        assert_eq!(
            place("Ada", 3, 3, Kind::Operator, " = ", 3),
            ("Ada = ".into(), 6)
        );
        assert_eq!(
            place("Ada ", 4, 4, Kind::Operator, " = ", 3),
            ("Ada = ".into(), 6)
        );
        assert_eq!(
            place("Ada 'x'", 3, 3, Kind::Operator, " = ", 3),
            ("Ada = 'x'".into(), 5)
        );
        assert_eq!(
            place("", 0, 0, Kind::Keyword, " değil ", 7),
            ("değil ".into(), 6)
        );
        assert_eq!(
            place("a", 1, 1, Kind::Field, "[Tapu alanı]", 12),
            ("a[Tapu alanı]".into(), 13)
        );
        assert_eq!(place("ab", 9, 1, Kind::Field, "x", 1), ("abx".into(), 3));
    }
}
