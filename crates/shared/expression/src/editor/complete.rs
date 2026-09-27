//! What the builder offers at the cursor: the names that can stand there
//! (`complete`), the call the cursor is in with its current argument
//! (`signature`), and the parenthesis matching the one at the cursor
//! (`bracket`). Names match without regard to case or Turkish letters
//! (`cev` finds `$çevre`'s `$uzunluk`, `round` finds yuvarla).

use std::cmp::Ordering;

use super::catalog::{field_item, function_item, operator_item, variable_item};
use super::lex::{Lex, Piece, called, lex};
use super::{Item, Kind};
use crate::Schema;
use crate::js::collate::compare_tr;
use crate::js::text::{fold_turkish, utf16_len};
use crate::library::{FUNCTIONS, FuncDef, OPERATORS, VARIABLES, find_function};

/// The entries that can stand at the cursor and the span they replace.
#[derive(Clone, Debug, PartialEq)]
pub struct Completion {
    pub start: usize,
    pub end: usize,
    pub items: Vec<Item>,
}

/// What the text at the cursor asks for.
enum Want {
    /// Any name (a word being written, or nothing yet).
    Any,
    /// A field, inside `[…`.
    Field,
    /// A variable, after `$`.
    Variable,
}

fn text(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

/// The piece the cursor is in or just after.
fn piece_at(pieces: &[Piece], cursor: usize) -> Option<usize> {
    pieces
        .iter()
        .position(|p| p.start < cursor && cursor <= p.end)
}

/// The names that can stand at `cursor` (UTF-16 units from 0). Without
/// `explicit` (the list opening as one types) only where a name is being
/// written: a word, a `$`, a `[`; with it (Ctrl+Space) also where nothing
/// is, which lists everything.
pub fn complete(src: &str, cursor: usize, schema: &Schema, explicit: bool) -> Option<Completion> {
    let u: Vec<u16> = src.encode_utf16().collect();
    let cursor = cursor.min(u.len());
    let pieces = lex(&u);
    let everything = || (Want::Any, String::new(), cursor, cursor, false);
    let (want, prefix, start, end, before_call) = match piece_at(&pieces, cursor) {
        Some(i) => {
            let p = &pieces[i];
            match &p.lex {
                Lex::Word { .. } => (
                    Want::Any,
                    text(&u[p.start..cursor]),
                    p.start,
                    p.end,
                    called(&pieces, i),
                ),
                Lex::Var { .. } => (
                    Want::Variable,
                    text(&u[p.start + 1..cursor]),
                    p.start,
                    p.end,
                    false,
                ),
                // Inside the brackets (not after the closing one).
                Lex::Field { closed, .. } if !(*closed && cursor == p.end) => (
                    Want::Field,
                    text(&u[p.start + 1..cursor]).trim_start().to_string(),
                    p.start,
                    p.end,
                    false,
                ),
                // Inside text or a number nothing is offered, even on request.
                Lex::Str { closed } if !(*closed && cursor == p.end) => return None,
                Lex::Num if cursor < p.end || !explicit => return None,
                _ if explicit => everything(),
                _ => return None,
            }
        }
        None if explicit => everything(),
        None => return None,
    };
    let key = fold_turkish(&prefix);
    let mut found: Vec<(Rank, Item)> = Vec::new();
    let mut add = |names: &[&str], item: Item| {
        if let Some((rank, alias)) = rank(&key, names) {
            found.push((
                rank,
                Item {
                    alias: alias.map(str::to_string),
                    ..item
                },
            ));
        }
    };
    if matches!(want, Want::Any | Want::Field) {
        for f in &schema.fields {
            let mut item = field_item(f);
            if matches!(want, Want::Field) {
                // Inside brackets the name stays bracketed.
                item.insert = format!("[{}]", f.name);
                item.caret = utf16_len(&item.insert);
            }
            add(&[&f.name], item);
        }
    }
    if matches!(want, Want::Any | Want::Variable) {
        for v in VARIABLES {
            let names: Vec<&str> = std::iter::once(v.name)
                .chain(v.aliases.iter().copied())
                .collect();
            add(&names, variable_item(v));
        }
    }
    if matches!(want, Want::Any) {
        for f in FUNCTIONS {
            let names: Vec<&str> = std::iter::once(f.name)
                .chain(f.aliases.iter().copied())
                .collect();
            add(&names, function_item(f, !before_call));
        }
        for o in OPERATORS
            .iter()
            .filter(|o| o.symbol.chars().all(char::is_alphabetic))
        {
            let names: Vec<&str> = std::iter::once(o.symbol)
                .chain(o.aliases.iter().copied())
                .collect();
            add(&names, operator_item(o, false));
        }
    }
    if found.is_empty() {
        return None;
    }
    found.sort_by(|(ra, a), (rb, b)| {
        ra.cmp(rb)
            .then(order(a.kind).cmp(&order(b.kind)))
            .then_with(|| compare_tr(&a.label, &b.label))
    });
    Some(Completion {
        start,
        end,
        items: found.into_iter().map(|(_, item)| item).collect(),
    })
}

/// How well a name matched what was typed, best first.
type Rank = u8;

/// The rank of the best of an entry's names (its own first, then its other
/// names) for a folded prefix, and the other name when that one matched.
fn rank<'n>(key: &str, names: &[&'n str]) -> Option<(Rank, Option<&'n str>)> {
    if key.is_empty() {
        return Some((1, None));
    }
    let mut best: Option<(Rank, Option<&'n str>)> = None;
    for (i, name) in names.iter().enumerate() {
        let folded = fold_turkish(name);
        let own = i == 0;
        let r = if folded == key {
            Some(if own { 0 } else { 2 })
        } else if folded.starts_with(key) {
            Some(if own { 1 } else { 3 })
        } else if folded
            .split(['_', ' '])
            .skip(1)
            .any(|part| part.starts_with(key))
        {
            Some(4)
        } else if own && key.chars().count() >= 3 && folded.contains(key) {
            Some(5)
        } else {
            None
        };
        if let Some(r) = r
            && best.is_none_or(|(b, _)| r < b)
        {
            best = Some((r, (!own).then_some(*name)));
        }
    }
    best
}

/// The list's order of kinds at equal rank.
fn order(kind: Kind) -> u8 {
    match kind {
        Kind::Field => 0,
        Kind::Function => 1,
        Kind::Variable => 2,
        Kind::Keyword | Kind::Operator => 3,
    }
}

/// An argument of a call's signature and its span in the signature's text.
#[derive(Clone, Debug, PartialEq)]
pub struct SignatureArg {
    pub name: &'static str,
    pub description: &'static str,
    pub optional: bool,
    /// Where it is in `Signature::signature`, in UTF-16 units.
    pub start: usize,
    pub end: usize,
}

/// The call the cursor is in: the function, its arguments and the one being written.
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    pub key: String,
    pub name: &'static str,
    pub signature: &'static str,
    pub description: &'static str,
    pub args: Vec<SignatureArg>,
    /// The argument the cursor is in; None past the last one the function takes.
    pub active: Option<usize>,
    /// Where the call's `(` is.
    pub open: usize,
}

/// The spans of the arguments in a signature's text: one for the whole
/// list when the function takes any number of them.
fn arg_spans(f: &FuncDef) -> Vec<(usize, usize)> {
    let u: Vec<u16> = f.signature.encode_utf16().collect();
    let (Some(open), Some(close)) = (
        u.iter().position(|&c| c == u16::from(b'(')),
        u.iter().rposition(|&c| c == u16::from(b')')),
    ) else {
        return Vec::new();
    };
    if f.arity.1.is_none() {
        return vec![(open + 1, close)];
    }
    let mut spans = Vec::new();
    let mut start = open + 1;
    for i in open + 1..=close {
        if i == close || u[i] == u16::from(b',') {
            let mut a = start;
            while a < i && u[a] == u16::from(b' ') {
                a += 1;
            }
            spans.push((a, i));
            start = i + 1;
        }
    }
    spans
}

/// The innermost call around `cursor` and the argument the cursor is in.
pub fn signature(src: &str, cursor: usize) -> Option<Signature> {
    let u: Vec<u16> = src.encode_utf16().collect();
    let pieces = lex(&u);
    // Open parentheses before the cursor: the function called (None for a
    // group), the commas seen at its level, where it opened.
    let mut open: Vec<(Option<&'static FuncDef>, usize, usize)> = Vec::new();
    for (i, p) in pieces.iter().enumerate() {
        if p.end > cursor {
            break;
        }
        match p.lex {
            Lex::Op("(") => {
                let f = match i.checked_sub(1).map(|j| &pieces[j].lex) {
                    Some(Lex::Word { name }) => find_function(name),
                    _ => None,
                };
                open.push((f, 0, p.start));
            }
            Lex::Op(")") => {
                open.pop();
            }
            Lex::Op(",") => {
                if let Some(top) = open.last_mut() {
                    top.1 += 1;
                }
            }
            _ => {}
        }
    }
    let &(f, commas, at) = open.iter().rev().find(|(f, _, _)| f.is_some())?;
    let f = f?;
    let spans = arg_spans(f);
    let args: Vec<SignatureArg> = f
        .args
        .iter()
        .enumerate()
        .map(|(i, (name, description))| {
            let (start, end) = spans.get(i).copied().unwrap_or((0, 0));
            SignatureArg {
                name,
                description,
                optional: i >= f.arity.0,
                start,
                end,
            }
        })
        .collect();
    let active = match f.arity.1 {
        None => Some(commas.min(args.len().saturating_sub(1))),
        Some(max) if commas < max => Some(commas.min(args.len().saturating_sub(1))),
        Some(_) => None,
    };
    Some(Signature {
        key: format!("func:{}", f.name),
        name: f.name,
        signature: f.signature,
        description: f.description,
        args,
        active,
        open: at,
    })
}

/// A parenthesis beside the cursor and the one it pairs with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bracket {
    /// Where the parenthesis beside the cursor is (the one before it first).
    pub at: usize,
    /// Where its pair is; None when it has none.
    pub partner: Option<usize>,
}

/// The parenthesis just before the cursor, else the one just after it, and its pair.
pub fn bracket(src: &str, cursor: usize) -> Option<Bracket> {
    let u: Vec<u16> = src.encode_utf16().collect();
    let pieces = lex(&u);
    let paren = |i: &usize| matches!(pieces[*i].lex, Lex::Op("(" | ")"));
    let i = (0..pieces.len())
        .find(|i| pieces[*i].end == cursor && paren(i))
        .or_else(|| (0..pieces.len()).find(|i| pieces[*i].start == cursor && paren(i)))?;
    let partner = if pieces[i].lex == Lex::Op("(") {
        let mut depth = 0usize;
        pieces[i..].iter().find_map(|p| {
            match p.lex {
                Lex::Op("(") => depth += 1,
                Lex::Op(")") => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(p.start);
                    }
                }
                _ => {}
            }
            None
        })
    } else {
        let mut depth = 0usize;
        pieces[..=i].iter().rev().find_map(|p| {
            match p.lex {
                Lex::Op(")") => depth += 1,
                Lex::Op("(") => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(p.start);
                    }
                }
                _ => {}
            }
            None
        })
    };
    Some(Bracket {
        at: pieces[i].start,
        partner,
    })
}

/// Sorts items as the tree lists a group: by the Turkish order of their labels.
pub(crate) fn by_label(a: &Item, b: &Item) -> Ordering {
    compare_tr(&a.label, &b.label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FieldDef, FieldSource, FieldType};

    fn schema() -> Schema {
        let f = |name: &str, ty, source| FieldDef {
            name: name.into(),
            ty,
            source,
            description: String::new(),
        };
        Schema {
            fields: vec![
                f("Ada", FieldType::Text, FieldSource::Attribute),
                f("Parsel", FieldType::Text, FieldSource::Attribute),
                f("Tapu alanı", FieldType::Number, FieldSource::User),
                f("Kat", FieldType::Number, FieldSource::User),
            ],
        }
    }

    fn labels(src: &str, cursor: usize, explicit: bool) -> Vec<String> {
        complete(src, cursor, &schema(), explicit)
            .map(|c| c.items.into_iter().map(|i| i.label).collect())
            .unwrap_or_default()
    }

    #[test]
    fn a_word_finds_fields_functions_variables_and_words() {
        assert_eq!(labels("ta", 2, false), ["Tapu alanı", "tamsayı"]);
        let c = complete("yuv", 3, &schema(), false).expect("yuvarla");
        assert_eq!((c.start, c.end), (0, 3));
        assert_eq!(c.items[0].insert, "yuvarla()");
        assert_eq!(c.items[0].caret, 8);
        // English names find the Turkish function, and say which name matched.
        let c = complete("rou", 3, &schema(), false).expect("round");
        assert_eq!(
            (c.items[0].label.as_str(), c.items[0].alias.as_deref()),
            ("yuvarla", Some("round"))
        );
        // Before an existing "(" only the name is replaced.
        let c = complete("yuv(1)", 2, &schema(), false).expect("before a call");
        assert_eq!(
            (c.start, c.end, c.items[0].insert.as_str()),
            (0, 3, "yuvarla")
        );
        // A field that is no plain word goes in brackets.
        let c = complete("1 + tap", 7, &schema(), false).expect("field");
        assert_eq!(c.items[0].insert, "[Tapu alanı]");
    }

    #[test]
    fn a_dollar_lists_variables_and_a_bracket_fields() {
        let all = labels("$", 1, false);
        assert_eq!(all.len(), VARIABLES.len());
        assert_eq!(labels("$cev", 4, false), ["$uzunluk"]);
        assert_eq!(labels("$y", 2, false)[..2], ["$y", "$yükseklik"]);
        assert_eq!(labels("[t", 2, false), ["Tapu alanı"]);
        let c = complete("[Tapu a] + 1", 4, &schema(), false).expect("inside brackets");
        assert_eq!(
            (c.start, c.end, c.items[0].insert.as_str()),
            (0, 8, "[Tapu alanı]")
        );
    }

    #[test]
    fn nothing_is_offered_in_text_numbers_or_between_tokens_unasked() {
        assert!(complete("'ab", 2, &schema(), true).is_none());
        assert!(complete("12", 1, &schema(), true).is_none());
        assert!(complete("Ada + ", 6, &schema(), false).is_none());
        let c = complete("Ada + ", 6, &schema(), true).expect("everything");
        assert_eq!((c.start, c.end), (6, 6));
        assert_eq!(c.items[0].label, "Ada");
        assert!(c.items.len() > FUNCTIONS.len() + VARIABLES.len());
    }

    #[test]
    fn the_signature_follows_the_argument_being_written() {
        let s = signature("yuvarla($alan, ", 15).expect("in the call");
        assert_eq!((s.name, s.active, s.open), ("yuvarla", Some(1), 7));
        let arg = &s.args[1];
        let sig: Vec<u16> = s.signature.encode_utf16().collect();
        assert_eq!(
            String::from_utf16_lossy(&sig[arg.start..arg.end]),
            "basamak"
        );
        assert!(arg.optional && !s.args[0].optional);
        // The innermost call; a group's parentheses are not a call.
        let s = signature("eğer(metin((1", 13).expect("inner");
        assert_eq!((s.name, s.active), ("metin", Some(0)));
        assert!(signature("yuvarla(1)", 10).is_none());
        let s = signature("min(1, 2, 3", 11).expect("any number");
        assert_eq!(s.active, Some(0));
        assert!(signature("mutlak(1, 2", 11).is_some_and(|s| s.active.is_none()));
    }

    #[test]
    fn parentheses_pair_up_around_text_and_brackets() {
        let src = "eğer(')' = [a)], (1), 2)";
        assert_eq!(
            bracket(src, 5),
            Some(Bracket {
                at: 4,
                partner: Some(23)
            })
        );
        assert_eq!(
            bracket(src, 24),
            Some(Bracket {
                at: 23,
                partner: Some(4)
            })
        );
        assert_eq!(
            bracket(src, 18),
            Some(Bracket {
                at: 17,
                partner: Some(19)
            })
        );
        assert_eq!(
            bracket("(1", 0),
            Some(Bracket {
                at: 0,
                partner: None
            })
        );
        assert_eq!(bracket("a + b", 2), None);
    }
}
