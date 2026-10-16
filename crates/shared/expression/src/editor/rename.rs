//! A field's name in an expression or a template, read and replaced (docs/adr/0414 §2): what the
//! fields' usage lists and what a field's rename carries to the rules that name it. Only the
//! references are touched: a quoted text, a function's name or a keyword that reads like the field
//! stays as it is, and so does every other character of the source.

use super::lex::{field_name, lex};
use super::units::to_byte;
use crate::template;

/// The fields an expression names, each once, in the order they first come.
pub fn fields_of(src: &str) -> Vec<String> {
    let u: Vec<u16> = src.encode_utf16().collect();
    let pieces = lex(&u);
    let mut out: Vec<String> = Vec::new();
    for i in 0..pieces.len() {
        if let Some(name) = field_name(&pieces, i)
            && !out.iter().any(|n| n == name)
        {
            out.push(name.to_owned());
        }
    }
    out
}

/// Whether the expression names field `name`.
pub fn names_field(src: &str, name: &str) -> bool {
    let u: Vec<u16> = src.encode_utf16().collect();
    let pieces = lex(&u);
    (0..pieces.len()).any(|i| field_name(&pieces, i) == Some(name))
}

/// `src` with every reference to field `from` naming `to`: a bracketed one stays bracketed, a bare
/// one stays bare when `to` reads there as the same field, bracketed otherwise. None when `src`
/// does not name `from` (or `from` is `to`).
pub fn rename_field(src: &str, from: &str, to: &str) -> Option<String> {
    if from == to {
        return None;
    }
    let bracketed = format!("[{to}]");
    let bare = rewrite(src, from, to, &bracketed)?;
    // A bare name that the language would read otherwise (a keyword, a call, two words) is
    // bracketed: the result must name `to` exactly where `src` named `from`.
    let want: Vec<String> = fields_of(src)
        .into_iter()
        .map(|n| if n == from { to.to_owned() } else { n })
        .fold(Vec::new(), |mut out, n| {
            if !out.contains(&n) {
                out.push(n);
            }
            out
        });
    if fields_of(&bare) == want {
        return Some(bare);
    }
    rewrite(src, from, &bracketed, &bracketed)
}

/// `src` with each bare reference to `from` as `bare` and each bracketed one as `bracketed`; none
/// when there is none.
fn rewrite(src: &str, from: &str, bare: &str, bracketed: &str) -> Option<String> {
    let u: Vec<u16> = src.encode_utf16().collect();
    let pieces = lex(&u);
    let mut out = String::with_capacity(src.len() + 8);
    let mut last = 0;
    let mut any = false;
    for (i, p) in pieces.iter().enumerate() {
        if field_name(&pieces, i) != Some(from) {
            continue;
        }
        let (start, end) = (to_byte(src, p.start), to_byte(src, p.end));
        out.push_str(&src[last..start]);
        out.push_str(if src[start..].starts_with('[') {
            bracketed
        } else {
            bare
        });
        last = end;
        any = true;
    }
    if !any {
        return None;
    }
    out.push_str(&src[last..]);
    Some(out)
}

/// Whether a template's expressions name field `name`; a template that does not read names none.
pub fn template_names_field(text: &str, name: &str) -> bool {
    template::parse(text).is_ok_and(|t| t.expressions().any(|(src, _)| names_field(src, name)))
}

/// A template (`{…}` expressions in text) with its expressions' references to `from` naming `to`;
/// none when it names none or does not read.
pub fn rename_in_template(text: &str, from: &str, to: &str) -> Option<String> {
    let parsed = template::parse(text).ok()?;
    let mut out = String::with_capacity(text.len() + 8);
    let mut last = 0;
    let mut any = false;
    for (src, at) in parsed.expressions() {
        let Some(renamed) = rename_field(src, from, to) else {
            continue;
        };
        // `at` is the source's first character, 1-based, in UTF-16 units.
        let start = to_byte(text, at - 1);
        let end = start + src.len();
        out.push_str(&text[last..start]);
        out.push_str(&renamed);
        last = end;
        any = true;
    }
    if !any {
        return None;
    }
    out.push_str(&text[last..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_is_renamed_and_nothing_else() {
        assert_eq!(
            rename_field("Kat > 3 ve [Kat] < 9 ve 'Kat' <> Ad", "Kat", "Kat sayısı").as_deref(),
            Some("[Kat sayısı] > 3 ve [Kat sayısı] < 9 ve 'Kat' <> Ad")
        );
        assert_eq!(
            rename_field("Kat > 3", "Kat", "Katlar").as_deref(),
            Some("Katlar > 3")
        );
        assert_eq!(rename_field("Ad = 'x'", "Kat", "Katlar"), None);
        // A name that would read as a keyword is bracketed.
        assert_eq!(
            rename_field("Kat > 3", "Kat", "ve").as_deref(),
            Some("[ve] > 3")
        );
        // A function of the same name is not the field.
        assert_eq!(
            rename_field("uzunluk(Ad) > uzunluk", "uzunluk", "Boy").as_deref(),
            Some("uzunluk(Ad) > Boy")
        );
        assert_eq!(fields_of("Kat > 3 ve [Ad] = Kat"), ["Kat", "Ad"]);
    }

    #[test]
    fn a_templates_expressions_are_renamed_and_its_text_kept() {
        assert_eq!(
            rename_in_template("Kat {Kat} · {{Kat}} · {[Kat] * 3}", "Kat", "Kat sayısı").as_deref(),
            Some("Kat {[Kat sayısı]} · {{Kat}} · {[Kat sayısı] * 3}")
        );
        assert_eq!(rename_in_template("Ağaç {Ad}", "Kat", "K"), None);
        assert!(template_names_field("Parsel {Ada}/{Parsel}", "Ada"));
    }
}
