//! The builder's lexer: the language's scanning (`lexer.rs`), but it goes on
//! past what the language refuses (an unclosed quote or bracket, a lone `$`,
//! a character it has no use for), so the editor colours and completes text
//! while it is being written; and each token's class for the colours.

use crate::js::text::{fold_turkish, is_space, trim};
use crate::lexer::{OPS, code_point, number_end, starts_number, word};
use crate::library::{find_function, find_variable};
use crate::parser::{Keyword, keyword_of};

/// What a piece of the source is.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Lex {
    Num,
    /// Text in quotes; not closed when the source ends inside it.
    Str {
        closed: bool,
    },
    /// `[name]`: the name trimmed; not closed when no `]` follows.
    Field {
        name: String,
        closed: bool,
    },
    /// `$name`: the name is empty for a lone `$`.
    Var {
        name: String,
    },
    Word {
        name: String,
    },
    Op(&'static str),
    /// A character the language has no use for.
    Bad,
}

/// A piece of the source and where it is, in UTF-16 units (`end` excluded).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Piece {
    pub lex: Lex,
    pub start: usize,
    pub end: usize,
    /// The operator (a symbol of `OPERATORS`) this word is part of, where
    /// the grammar reads it so (docs/adr/0100 §4): `durum` for the words of
    /// `durum … son`, `içinde` for `içinde` and a `değil` before it, `boş`
    /// or `boş değil` for `x boş`, `x IS NOT NULL` … Elsewhere the same
    /// words are fields: a field may be called Durum, Son or Gibi.
    pub phrase: Option<&'static str>,
}

fn starts_with(u: &[u16], i: usize, op: &str) -> bool {
    op.bytes()
        .enumerate()
        .all(|(k, b)| u.get(i + k) == Some(&u16::from(b)))
}

fn text(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

/// The pieces of a source given as UTF-16 units; white space is between them.
pub(crate) fn lex(u: &[u16]) -> Vec<Piece> {
    let (quote, apostrophe, open, close, dollar) = (
        u16::from(b'"'),
        u16::from(b'\''),
        u16::from(b'['),
        u16::from(b']'),
        u16::from(b'$'),
    );
    let mut out = Vec::new();
    let mut i = 0;
    while i < u.len() {
        let c = u[i];
        if char::from_u32(u32::from(c)).is_some_and(is_space) {
            i += 1;
            continue;
        }
        let start = i;
        let lex = if starts_number(u, i) {
            i = number_end(u, i);
            Lex::Num
        } else if c == quote || c == apostrophe {
            // A doubled quote inside is one quote, as the language reads it.
            let mut j = i + 1;
            let mut closed = false;
            while j < u.len() {
                if u[j] == c {
                    if u.get(j + 1) == Some(&c) {
                        j += 2;
                        continue;
                    }
                    closed = true;
                    j += 1;
                    break;
                }
                j += 1;
            }
            i = j;
            Lex::Str { closed }
        } else if c == open {
            match u[i..].iter().position(|&x| x == close) {
                Some(k) => {
                    let name = trim(&text(&u[i + 1..i + k])).to_string();
                    i += k + 1;
                    Lex::Field { name, closed: true }
                }
                None => {
                    let name = trim(&text(&u[i + 1..])).to_string();
                    i = u.len();
                    Lex::Field {
                        name,
                        closed: false,
                    }
                }
            }
        } else if c == dollar {
            let n = word(u, i + 1);
            let name = text(&u[i + 1..i + 1 + n]);
            i += 1 + n;
            Lex::Var { name }
        } else if let n @ 1.. = word(u, i) {
            let name = text(&u[i..i + n]);
            i += n;
            Lex::Word { name }
        } else if let Some(op) = OPS.iter().find(|o| starts_with(u, i, o)) {
            i += op.len();
            Lex::Op(op)
        } else {
            i += code_point(u, i).map_or(1, |(_, n)| n);
            Lex::Bad
        };
        out.push(Piece {
            lex,
            start,
            end: i,
            phrase: None,
        });
    }
    mark(&mut out);
    out
}

/// The words that stand after a value, folded, and their operators.
const AFTER_VALUE: [(&str, &str); 8] = [
    ("ICINDE", "içinde"),
    ("IN", "içinde"),
    ("ARASINDA", "arasında"),
    ("BETWEEN", "arasında"),
    ("GIBI", "gibi"),
    ("LIKE", "gibi"),
    ("BENZER", "benzer"),
    ("ILIKE", "benzer"),
];

/// A word's folded form, if the piece is a word.
fn folded(p: &Piece) -> Option<String> {
    match &p.lex {
        Lex::Word { name } => Some(fold_turkish(name)),
        _ => None,
    }
}

/// The operator of a word that stands after a value.
fn after_value(w: Option<&str>) -> Option<&'static str> {
    let w = w?;
    AFTER_VALUE.iter().find(|(f, _)| *f == w).map(|(_, s)| *s)
}

/// Sets `Piece::phrase` in one pass that follows, as the parser does,
/// whether a value has just ended (where an operator stands) and the
/// `durum`s not yet closed; linear, so a long text colours at once.
fn mark(pieces: &mut [Piece]) {
    let mut value = false;
    let mut cases = 0usize;
    let mut i = 0;
    while i < pieces.len() {
        let Some(w) = folded(&pieces[i]) else {
            value = matches!(
                pieces[i].lex,
                Lex::Num | Lex::Str { .. } | Lex::Field { .. } | Lex::Var { .. } | Lex::Op(")")
            );
            i += 1;
            continue;
        };
        let word_at = |k: usize| pieces.get(k).and_then(folded);
        let keyword_at = |k: usize| word_at(k).as_deref().and_then(keyword_of);
        let keyword = keyword_of(&w);
        // How many pieces from `i` the operator takes, and which it is.
        let mut phrase: Option<(usize, &'static str)> = None;
        if value {
            if let Some(symbol) = after_value(Some(&w)) {
                phrase = Some((1, symbol));
                value = false;
            } else if keyword == Some(Keyword::Not)
                && let Some(symbol) = after_value(word_at(i + 1).as_deref())
            {
                phrase = Some((2, symbol));
                value = false;
            } else if keyword == Some(Keyword::Null) || w == "IS" {
                // `x boş [değil]`; `x IS [NOT] NULL`.
                let negated = keyword_at(i + 1) == Some(Keyword::Not);
                let mut n = 1 + usize::from(negated);
                value = true;
                if w == "IS" {
                    if keyword_at(i + n) == Some(Keyword::Null) {
                        n += 1;
                    } else {
                        value = false;
                    }
                }
                phrase = Some((n, if negated { "boş değil" } else { "boş" }));
            } else if cases > 0
                && matches!(
                    w.as_str(),
                    "ISE" | "THEN" | "EGER" | "WHEN" | "YOKSA" | "ELSE"
                )
            {
                phrase = Some((1, "durum"));
                value = false;
            } else if cases > 0 && matches!(w.as_str(), "SON" | "END") {
                phrase = Some((1, "durum"));
                cases -= 1;
            } else {
                value = !called(pieces, i)
                    && !matches!(keyword, Some(Keyword::And | Keyword::Or | Keyword::Not));
            }
        } else if matches!(w.as_str(), "DURUM" | "CASE")
            && matches!(word_at(i + 1).as_deref(), Some("EGER" | "WHEN"))
        {
            phrase = Some((2, "durum"));
            cases += 1;
        } else {
            value = match keyword {
                Some(Keyword::And | Keyword::Or | Keyword::Not) => false,
                Some(Keyword::True | Keyword::False | Keyword::Null) => true,
                None => !called(pieces, i),
            };
        }
        let n = phrase.map_or(1, |(n, symbol)| {
            for p in &mut pieces[i..i + n] {
                p.phrase = Some(symbol);
            }
            n
        });
        i += n;
    }
}

/// A token's class, for the editor's colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    Number,
    /// Text in quotes.
    Text,
    /// A field: a bare name or one in brackets.
    Field,
    /// A `$` value the language has.
    Variable,
    /// A function the language has, called.
    Function,
    /// ve, veya, değil (and, or, not).
    Keyword,
    /// doğru, yanlış, boş (true, false, null).
    Constant,
    Operator,
    Paren,
    Comma,
    /// What the language does not know: `$nope`, `nope(…)`, a stray character.
    Unknown,
}

impl Class {
    /// A stable name (the pages' styles and the fixture).
    pub fn id(self) -> &'static str {
        match self {
            Class::Number => "number",
            Class::Text => "text",
            Class::Field => "field",
            Class::Variable => "variable",
            Class::Function => "function",
            Class::Keyword => "keyword",
            Class::Constant => "constant",
            Class::Operator => "operator",
            Class::Paren => "paren",
            Class::Comma => "comma",
            Class::Unknown => "unknown",
        }
    }
}

/// A token's class and where it is, in UTF-16 units from 0 (`end` excluded).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub class: Class,
    pub start: usize,
    pub end: usize,
}

/// Whether the word at `i` is called: a `(` follows it, as the parser reads a call.
pub(crate) fn called(pieces: &[Piece], i: usize) -> bool {
    matches!(pieces.get(i + 1).map(|p| &p.lex), Some(Lex::Op("(")))
}

/// The class of piece `i`.
pub(crate) fn class_of(pieces: &[Piece], i: usize) -> Class {
    match &pieces[i].lex {
        Lex::Num => Class::Number,
        Lex::Str { .. } => Class::Text,
        Lex::Field { .. } => Class::Field,
        Lex::Var { name } => match find_variable(name) {
            Some(_) => Class::Variable,
            None => Class::Unknown,
        },
        Lex::Word { name } => {
            if pieces[i].phrase.is_some() {
                return Class::Keyword;
            }
            if called(pieces, i) {
                return match find_function(name) {
                    Some(_) => Class::Function,
                    None => Class::Unknown,
                };
            }
            match keyword_of(name) {
                Some(Keyword::And | Keyword::Or | Keyword::Not) => Class::Keyword,
                Some(Keyword::True | Keyword::False | Keyword::Null) => Class::Constant,
                None => Class::Field,
            }
        }
        Lex::Op("(" | ")") => Class::Paren,
        Lex::Op(",") => Class::Comma,
        Lex::Op(_) => Class::Operator,
        Lex::Bad => Class::Unknown,
    }
}

/// The source's tokens with their classes, in order.
pub fn tokens(src: &str) -> Vec<Span> {
    let u: Vec<u16> = src.encode_utf16().collect();
    let pieces = lex(&u);
    (0..pieces.len())
        .map(|i| Span {
            class: class_of(&pieces, i),
            start: pieces[i].start,
            end: pieces[i].end,
        })
        .collect()
}

/// The field a piece names, if it names one: a bracketed name, or a bare
/// word that is neither called nor a keyword.
pub(crate) fn field_name(pieces: &[Piece], i: usize) -> Option<&str> {
    match &pieces[i].lex {
        Lex::Field { name, .. } if !name.is_empty() => Some(name),
        Lex::Word { name } if class_of(pieces, i) == Class::Field => Some(name),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(src: &str) -> String {
        tokens(src)
            .iter()
            .map(|s| format!("{}:{}-{}", s.class.id(), s.start, s.end))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn classes_follow_the_grammar_and_survive_broken_text() {
        assert_eq!(
            classes("yuvarla($alan, 2) || ' m²'"),
            "function:0-7 paren:7-8 variable:8-13 comma:13-14 number:15-16 paren:16-17 operator:18-20 text:21-26"
        );
        // A word is a field unless called or a keyword; boş( is the function.
        assert_eq!(
            classes("Ada ve değil boş(Parsel) veya boş"),
            "field:0-3 keyword:4-6 keyword:7-12 function:13-16 paren:16-17 field:17-23 paren:23-24 keyword:25-29 constant:30-33"
        );
        // Unclosed text and brackets run to the end; unknown names and characters are marked.
        assert_eq!(classes("'abc"), "text:0-4");
        assert_eq!(classes("[Tapu a"), "field:0-7");
        assert_eq!(
            classes("$nope + nope(1) # $"),
            "unknown:0-5 operator:6-7 unknown:8-12 paren:12-13 number:13-14 paren:14-15 unknown:16-17 unknown:18-19"
        );
        // UTF-16 positions: 𝒜 is two units.
        assert_eq!(classes("'𝒜' || x"), "text:0-4 operator:5-7 field:8-9");
    }

    #[test]
    fn the_new_words_are_keywords_only_where_the_grammar_reads_them() {
        let keywords = |src: &str| -> Vec<String> {
            let u: Vec<u16> = src.encode_utf16().collect();
            let pieces = lex(&u);
            (0..pieces.len())
                .filter(|&i| class_of(&pieces, i) == Class::Keyword)
                .map(|i| String::from_utf16_lossy(&u[pieces[i].start..pieces[i].end]))
                .collect()
        };
        assert_eq!(
            keywords("durum eğer Kat > 3 ise 'yüksek' yoksa 'alçak' son"),
            ["durum", "eğer", "ise", "yoksa", "son"]
        );
        assert_eq!(
            keywords("Nitelik değil içinde ('Arsa') ve Kat arasında 3 ve 5"),
            ["değil", "içinde", "ve", "arasında", "ve"]
        );
        assert_eq!(
            keywords("Ada gibi '12%' veya Ada IS NOT NULL veya Ada boş"),
            ["gibi", "veya", "IS", "NOT", "NULL", "veya", "boş"]
        );
        // Nested, and a field called Gibi before gibi.
        assert_eq!(
            keywords("durum eğer Gibi gibi 'a%' ise durum eğer x ise 1 son son"),
            [
                "durum", "eğer", "gibi", "ise", "durum", "eğer", "ise", "son", "son"
            ]
        );
        // Fields called so, and eğer the function, stay what they are.
        assert!(keywords("Durum = 'Son' || Gibi || eğer(Son, 1, 2)").is_empty());
        assert_eq!(
            classes("eğer(1, 2, 3)").split(' ').next(),
            Some("function:0-4")
        );
        assert_eq!(classes("2 ^ 3"), "number:0-1 operator:2-3 number:4-5");
    }
}
