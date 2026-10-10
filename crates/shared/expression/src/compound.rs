//! Diziler ve eşlemeler (docs/adr/0214 §2.1, §2.5): the language's fourth
//! and fifth kinds of value. Inside, each is its JSON text behind a mark no
//! one types (U+E000 an array, U+E001 a map), so the engine's registers and
//! columns hold them as text and nothing else changes; they show (written
//! into a field, joined, made text) as that JSON. Numbers are written as
//! JavaScript's `JSON.stringify` writes them (the shortest text that reads
//! back), text with JSON's escapes; a number that is not finite is null.
//! Two arrays are equal when their texts are; a non-empty one is true.

use kentos_geometry_core::api::json::Json;

use crate::js::number;
use crate::scalar::{self, Scratch, V};

/// The marks: an array's and a map's text starts with one.
pub const ARRAY: char = '\u{E000}';
pub const MAP: char = '\u{E001}';
/// A mark's length in UTF-8.
const MARK: usize = 3;
/// The most items an array or a map holds (§2.5).
pub const MOST_ITEMS: usize = 1_000_000;

/// Which of the two a text is, when it is one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Array,
    Map,
}

pub fn kind(s: &str) -> Option<Kind> {
    match s.chars().next() {
        Some(ARRAY) => Some(Kind::Array),
        Some(MAP) => Some(Kind::Map),
        _ => None,
    }
}

/// Whether a text is an array or a map.
#[inline]
pub fn is_compound(s: &str) -> bool {
    // Both marks start with the bytes EE 80.
    s.len() >= MARK && s.as_bytes()[0] == 0xEE && s.as_bytes()[1] == 0x80 && kind(s).is_some()
}

/// What an array or a map shows: its JSON (any other text as it is).
#[inline]
pub fn shown(s: &str) -> &str {
    if is_compound(s) { &s[MARK..] } else { s }
}

/// An empty array or map: false, as an empty text is.
pub fn is_empty_compound(s: &str) -> bool {
    s.len() == MARK + 2 && is_compound(s) && matches!(&s[MARK..], "[]" | "{}")
}

/// An item of an array or a map's value: a value of the language, or JSON
/// that is an array or a map itself (kept as its text).
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Null,
    Num(f64),
    Text(String),
    Bool(bool),
    /// A nested array or map, its JSON with its mark.
    Nested(String),
}

impl Item {
    /// The item as the rules read it.
    pub fn view(&self) -> V<'_> {
        match self {
            Item::Null => V::Null,
            Item::Num(x) => V::Num(*x),
            Item::Text(s) | Item::Nested(s) => V::Text(s),
            Item::Bool(b) => V::Bool(*b),
        }
    }

    /// A value of the language as an item (an array or a map nested as its text).
    pub fn of(v: V) -> Item {
        match v {
            V::Null => Item::Null,
            V::Num(x) if x.is_finite() => Item::Num(x),
            V::Num(_) => Item::Null,
            V::Bool(b) => Item::Bool(b),
            V::Text(s) if is_compound(s) => Item::Nested(s.to_owned()),
            V::Text(s) => Item::Text(s.to_owned()),
        }
    }
}

fn item_of(j: Json) -> Item {
    match j {
        Json::Null => Item::Null,
        Json::Bool(b) => Item::Bool(b),
        Json::Num(x) => Item::Num(x),
        Json::Str(s) => Item::Text(s),
        Json::Arr(_) | Json::Obj(_) => Item::Nested(marked(&j)),
    }
}

/// An array's or a map's text: its mark, then its JSON.
fn marked(j: &Json) -> String {
    let mut out = String::new();
    out.push(if matches!(j, Json::Obj(_)) {
        MAP
    } else {
        ARRAY
    });
    push_plain_json(&mut out, j);
    out
}

/// JSON as the language writes it, without marks.
fn push_plain_json(out: &mut String, j: &Json) {
    match j {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(x) => push_number(out, *x),
        Json::Str(s) => push_string(out, s),
        Json::Arr(items) => {
            out.push('[');
            for (k, v) in items.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                push_plain_json(out, v);
            }
            out.push(']');
        }
        Json::Obj(fields) => {
            out.push('{');
            for (k, (key, v)) in fields.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                push_string(out, key);
                out.push(':');
                push_plain_json(out, v);
            }
            out.push('}');
        }
    }
}

/// A finite number as `JSON.stringify` writes it; any other is null.
fn push_number(out: &mut String, x: f64) {
    if x.is_finite() {
        number::push_string(out, x);
    } else {
        out.push_str("null");
    }
}

/// A text as JSON writes it: quotes, backslashes and control characters escaped.
pub fn push_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str("\\u00");
                let n = c as u32;
                for d in [n >> 4, n & 15] {
                    out.push(char::from_digit(d, 16).unwrap_or('0'));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// One item's JSON (inside an array or a map: no mark).
pub fn push_item(out: &mut String, it: &Item) {
    match it {
        Item::Null => out.push_str("null"),
        Item::Num(x) => push_number(out, *x),
        Item::Text(s) => push_string(out, s),
        Item::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Item::Nested(s) => out.push_str(shown(s)),
    }
}

/// A value's JSON as `json_yaz` writes it (an array or a map: its JSON).
pub fn push_value_json(out: &mut String, v: V) {
    match v {
        V::Text(s) if is_compound(s) => out.push_str(shown(s)),
        v => push_item(out, &Item::of(v)),
    }
}

/// An array of items, with its mark, at the end of `out`.
pub fn push_array<'i>(out: &mut String, items: impl IntoIterator<Item = &'i Item>) {
    out.push(ARRAY);
    out.push('[');
    for (k, it) in items.into_iter().enumerate() {
        if k > 0 {
            out.push(',');
        }
        push_item(out, it);
    }
    out.push(']');
}

/// A map of keys and items, with its mark, at the end of `out`.
pub fn push_map<'i>(out: &mut String, pairs: impl IntoIterator<Item = (&'i str, &'i Item)>) {
    out.push(MAP);
    out.push('{');
    for (k, (key, it)) in pairs.into_iter().enumerate() {
        if k > 0 {
            out.push(',');
        }
        push_string(out, key);
        out.push(':');
        push_item(out, it);
    }
    out.push('}');
}

/// An array's items; None when the value is not an array.
pub fn array_items(v: V) -> Option<Vec<Item>> {
    let V::Text(s) = v else {
        return None;
    };
    if kind(s) != Some(Kind::Array) {
        return None;
    }
    match Json::parse(shown(s)).ok()? {
        Json::Arr(items) => Some(items.into_iter().map(item_of).collect()),
        _ => None,
    }
}

/// A map's keys and items, in order; None when the value is not a map.
pub fn map_items(v: V) -> Option<Vec<(String, Item)>> {
    let V::Text(s) = v else {
        return None;
    };
    if kind(s) != Some(Kind::Map) {
        return None;
    }
    match Json::parse(shown(s)).ok()? {
        Json::Obj(fields) => {
            // A key written twice keeps its last value, in its first place.
            let mut out: Vec<(String, Item)> = Vec::with_capacity(fields.len());
            for (k, j) in fields {
                let it = item_of(j);
                match out.iter_mut().find(|(key, _)| *key == k) {
                    Some(slot) => slot.1 = it,
                    None => out.push((k, it)),
                }
            }
            Some(out)
        }
        _ => None,
    }
}

/// JSON text read as a value (`json_oku`): an array or a map with its mark,
/// or a number, a text, true/false, null; None when it is not JSON.
pub fn from_json(text: &str) -> Option<Item> {
    Json::parse(crate::js::text::trim(text)).ok().map(item_of)
}

/// Whether an item equals a value by `=`'s rule.
pub fn item_equals(it: &Item, v: V, s: &mut Scratch) -> bool {
    scalar::equals(it.view(), v, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn array(items: &[Item]) -> String {
        let mut out = String::new();
        push_array(&mut out, items);
        out
    }

    #[test]
    fn arrays_and_maps_read_and_write_their_json() {
        let a = array(&[
            Item::Num(1.0),
            Item::Text("Arsa \"A\"".into()),
            Item::Bool(true),
            Item::Null,
            Item::Num(0.1 + 0.2),
        ]);
        assert_eq!(
            shown(&a),
            r#"[1,"Arsa \"A\"",true,null,0.30000000000000004]"#
        );
        assert_eq!(kind(&a), Some(Kind::Array));
        let back = array_items(V::Text(&a)).expect("an array");
        assert_eq!(back[1], Item::Text("Arsa \"A\"".into()));
        let mut m = String::new();
        push_map(
            &mut m,
            [("ad", &Item::Text("A".into())), ("kat", &Item::Num(3.0))],
        );
        assert_eq!(shown(&m), r#"{"ad":"A","kat":3}"#);
        assert_eq!(map_items(V::Text(&m)).expect("a map").len(), 2);
        assert!(is_empty_compound(&array(&[])));
        assert!(!is_empty_compound(&a));
        // A nested array keeps its own JSON, and shows inside without a mark.
        let nested = array(&[Item::Nested(a.clone())]);
        assert_eq!(shown(&nested), format!("[{}]", shown(&a)));
        assert_eq!(
            from_json(" [1, [2, 3], {\"a\": 4}] ").map(|i| match i {
                Item::Nested(s) => shown(&s).to_owned(),
                _ => String::new(),
            }),
            Some(r#"[1,[2,3],{"a":4}]"#.to_owned())
        );
    }
}
