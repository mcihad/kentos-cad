//! The builder's diagnostics: the language's error with the span to
//! underline, and a warning for each field the objects do not have (it
//! reads as empty), with the field meant when a letter or the case slipped.

use super::lex::{Piece, field_name, lex};
use crate::js::text::fold_turkish;
use crate::lexer::code_point;
use crate::{CompileError, Schema, compile_with};

/// What is wrong or doubtful, and the span to underline, in UTF-16 units
/// from 0 (`start == end` at the end of the text: after the last character).
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub start: usize,
    pub end: usize,
    pub message: String,
}

impl Diagnostic {
    /// "8. karakterde: …", as the language's errors read.
    pub fn text(&self) -> String {
        CompileError {
            message: self.message.clone(),
            at: self.start + 1,
        }
        .text()
    }
}

/// The error the text has (None when it compiles) and its warnings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Check {
    pub error: Option<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

/// Checks a source against the fields of the objects it will run on
/// (without fields, no field is doubted).
pub fn check(src: &str, schema: &Schema) -> Check {
    let u: Vec<u16> = src.encode_utf16().collect();
    let pieces = lex(&u);
    let error = compile_with(src, schema).err().map(|e| {
        let (start, end) = error_span(e.at, &pieces, &u);
        Diagnostic {
            start,
            end,
            message: e.message,
        }
    });
    let warnings = if schema.fields.is_empty() {
        Vec::new()
    } else {
        (0..pieces.len())
            .filter_map(|i| {
                let name = field_name(&pieces, i)?;
                if schema.find(name).is_some() {
                    return None;
                }
                let message = match suggestion(name, schema) {
                    Some(s) => format!(
                        "Bu nesnelerde “{name}” alanı yok, boş okunur. “{s}” mi yazılacaktı?"
                    ),
                    None => format!("Bu nesnelerde “{name}” alanı yok, boş okunur."),
                };
                Some(Diagnostic {
                    start: pieces[i].start,
                    end: pieces[i].end,
                    message,
                })
            })
            .collect()
    };
    Check { error, warnings }
}

/// The span of an error at `at` (1-based): the piece that starts there, else
/// the character there, else the end of the text.
fn error_span(at: usize, pieces: &[Piece], u: &[u16]) -> (usize, usize) {
    let start = at.saturating_sub(1).min(u.len());
    if let Some(p) = pieces.iter().find(|p| p.start == start) {
        return (start, p.end);
    }
    match code_point(u, start) {
        Some((_, n)) => (start, start + n),
        None => (u.len(), u.len()),
    }
}

/// The field a misspelt name probably meant: the same letters in another
/// case (or without the Turkish marks), else one a letter or two away.
fn suggestion<'s>(name: &str, schema: &'s Schema) -> Option<&'s str> {
    let k: Vec<char> = fold_turkish(name).chars().collect();
    let folded: Vec<(Vec<char>, &str)> = schema
        .fields
        .iter()
        .map(|f| (fold_turkish(&f.name).chars().collect(), f.name.as_str()))
        .collect();
    if let Some((_, f)) = folded.iter().find(|(f, _)| *f == k) {
        return Some(f);
    }
    let limit = if k.len() <= 4 { 1 } else { 2 };
    folded
        .iter()
        .map(|(f, n)| (distance(&k, f), *n))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, n)| n)
}

/// Letters to insert, remove or change to turn `a` into `b` (Levenshtein).
fn distance(a: &[char], b: &[char]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if ca == cb {
                diagonal
            } else {
                1 + diagonal.min(above).min(row[j])
            };
            diagonal = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FieldDef, FieldSource, FieldType};

    fn schema(names: &[&str]) -> Schema {
        Schema {
            fields: names
                .iter()
                .map(|n| FieldDef {
                    name: (*n).into(),
                    ty: FieldType::Text,
                    source: FieldSource::Attribute,
                    description: String::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn errors_are_underlined_where_the_language_points() {
        let c = check("yuvarla($alan, 2", &Schema::default());
        let e = c.error.as_ref().map(|e| (e.start, e.end, e.text()));
        assert_eq!(
            e,
            Some((
                16,
                16,
                "17. karakterde: yuvarla(…) kapanmamış: “)” bekleniyordu.".into()
            ))
        );
        let c = check("1 + 'abc", &Schema::default());
        assert_eq!(c.error.map(|e| (e.start, e.end)), Some((4, 8)));
        let c = check("$nope * 2", &Schema::default());
        assert_eq!(
            c.error.map(|e| (e.start, e.end, e.text())),
            Some((0, 5, "Bilinmeyen değişken: $nope.".into()))
        );
        assert_eq!(check("Parsel > 5", &Schema::default()), Check::default());
    }

    #[test]
    fn fields_the_objects_lack_are_doubted_with_the_one_meant() {
        let s = schema(&["Parsel", "Ada", "Tapu alanı"]);
        let c = check("parsel || Pafta || [Tapu alani] || Adaa || Ada", &s);
        assert_eq!(c.error, None);
        let w: Vec<_> = c
            .warnings
            .iter()
            .map(|d| (d.start, d.end, d.message.as_str()))
            .collect();
        assert_eq!(
            w,
            [
                (
                    0,
                    6,
                    "Bu nesnelerde “parsel” alanı yok, boş okunur. “Parsel” mi yazılacaktı?"
                ),
                (10, 15, "Bu nesnelerde “Pafta” alanı yok, boş okunur."),
                (
                    19,
                    31,
                    "Bu nesnelerde “Tapu alani” alanı yok, boş okunur. “Tapu alanı” mi yazılacaktı?"
                ),
                (
                    35,
                    39,
                    "Bu nesnelerde “Adaa” alanı yok, boş okunur. “Ada” mi yazılacaktı?"
                ),
            ]
        );
        assert_eq!(distance(&['a', 'b'], &['b', 'a']), 2);
    }
}
