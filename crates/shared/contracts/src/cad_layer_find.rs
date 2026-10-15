//! `cad.layers.find` and `cad.layers.translate` (docs/adr/0311 §4, §5): the
//! layers found by a name pattern and their properties (AutoCAD's layer
//! filters), and layers merged into a drawing standard's by rules (its
//! LAYTRANS), its plan the preview. The name pattern is AutoCAD's `wcmatch`
//! (`pattern_matches`), letters folded as Turkish is written.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::LineType;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

// ── The name pattern ────────────────────────────────────────────────────────

/// A name folded for comparing: lower case as Turkish writes it (I ı, İ i).
pub fn fold_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            'I' => out.push('ı'),
            'İ' => out.push('i'),
            _ => out.extend(c.to_lowercase()),
        }
    }
    out
}

fn fold_char(c: char) -> char {
    match c {
        'I' => 'ı',
        'İ' => 'i',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Char(char),
    /// `#`: a digit.
    Digit,
    /// `@`: a letter.
    Letter,
    /// `.`: neither a letter nor a digit.
    Other,
    /// `?`: any one character.
    One,
    /// `*`: any run, empty too.
    Any,
    /// `[…]`, `[~…]`: one of (or none of) these characters and ranges.
    Class(bool, Vec<(char, char)>),
}

/// One of a pattern's comma-separated alternatives: whether it is negated
/// (`~` first) and its tokens.
fn tokens(alternative: &[char]) -> Result<(bool, Vec<Token>), String> {
    let (negated, body) = match alternative.first() {
        Some('~') => (true, &alternative[1..]),
        _ => (false, alternative),
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let c = body[i];
        i += 1;
        out.push(match c {
            '`' => {
                let Some(&next) = body.get(i) else {
                    return Err("` kaçış işaretinden sonra karakter yok".to_owned());
                };
                i += 1;
                Token::Char(fold_char(next))
            }
            '#' => Token::Digit,
            '@' => Token::Letter,
            '.' => Token::Other,
            '?' => Token::One,
            '*' => Token::Any,
            '[' => {
                let not = body.get(i) == Some(&'~');
                if not {
                    i += 1;
                }
                let mut set = Vec::new();
                let mut closed = false;
                while i < body.len() {
                    let mut a = body[i];
                    i += 1;
                    if a == ']' && !set.is_empty() {
                        closed = true;
                        break;
                    }
                    if a == '`' {
                        let Some(&next) = body.get(i) else { break };
                        a = next;
                        i += 1;
                    }
                    let a = fold_char(a);
                    if body.get(i) == Some(&'-') && body.get(i + 1).is_some_and(|&b| b != ']') {
                        let b = fold_char(body[i + 1]);
                        i += 2;
                        set.push((a.min(b), a.max(b)));
                    } else {
                        set.push((a, a));
                    }
                }
                if !closed {
                    return Err("[ ile açılan küme ] ile kapanmıyor".to_owned());
                }
                Token::Class(not, set)
            }
            _ => Token::Char(fold_char(c)),
        });
    }
    Ok((negated, out))
}

fn one(token: &Token, c: char) -> bool {
    match token {
        Token::Char(t) => *t == c,
        Token::Digit => c.is_ascii_digit(),
        Token::Letter => c.is_alphabetic(),
        Token::Other => !c.is_alphanumeric(),
        Token::One => true,
        Token::Any => true,
        Token::Class(not, set) => set.iter().any(|&(a, b)| a <= c && c <= b) != *not,
    }
}

/// Whether the tokens match the whole of `name` (folded), `*` backtracking.
fn whole(tokens: &[Token], name: &[char]) -> bool {
    let (mut t, mut n) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while n < name.len() {
        if t < tokens.len() && tokens[t] == Token::Any {
            star = Some((t, n));
            t += 1;
        } else if t < tokens.len() && one(&tokens[t], name[n]) {
            t += 1;
            n += 1;
        } else if let Some((st, sn)) = star {
            t = st + 1;
            n = sn + 1;
            star = Some((st, sn + 1));
        } else {
            return false;
        }
    }
    tokens[t..].iter().all(|k| *k == Token::Any)
}

/// What is wrong with a name pattern, in its words; none when it reads.
pub fn pattern_problem(pattern: &str) -> Option<String> {
    if pattern.trim().is_empty() {
        return Some("örüntü boş".to_owned());
    }
    let chars: Vec<char> = pattern.chars().collect();
    split(&chars).into_iter().find_map(|alt| tokens(alt).err())
}

/// The alternatives of a pattern: split at commas not escaped nor in a class.
fn split(chars: &[char]) -> Vec<&[char]> {
    let mut out = Vec::new();
    let (mut start, mut i, mut in_class) = (0, 0, false);
    while i < chars.len() {
        match chars[i] {
            '`' => i += 1,
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            ',' if !in_class => {
                out.push(&chars[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(&chars[start..]);
    out
}

/// Whether a name matches a pattern as AutoCAD's `wcmatch` reads it: `*` any
/// run, `?` one character, `#` a digit, `@` a letter, `.` neither, `[…]` one
/// of, `[~…]` none of (`a-z` ranges), `` ` `` the next character as it is,
/// `,` between alternatives, `~` first: anything but. Letters are compared
/// folded (`fold_name`). A pattern with a problem matches nothing.
pub fn pattern_matches(pattern: &str, name: &str) -> bool {
    let chars: Vec<char> = pattern.chars().collect();
    let folded: Vec<char> = fold_name(name).chars().collect();
    split(&chars).into_iter().any(|alt| match tokens(alt) {
        Ok((negated, tokens)) => whole(&tokens, &folded) != negated,
        Err(_) => false,
    })
}

// ── cad.layers.find ─────────────────────────────────────────────────────────

/// Finds layers by a name pattern and their properties; reads only.
pub const CAD_LAYERS_FIND: &str = "cad.layers.find";
pub const CAD_LAYERS_FIND_VERSION: u32 = 1;

/// Input of `cad.layers.find` v1 (docs/adr/0311 §4): what a layer must be,
/// every condition given (absent: any). Only layers are found, never groups;
/// a group's eye, freeze, lock and printing count for what it holds.
///
/// - `name`: a pattern (`pattern_matches`) over the layer's own name;
/// - `group`: only the layers under this group (or this layer itself);
/// - `color` (`#rrggbb`, compared without case, or a theme token),
///   `lineType`, `lineWeight` (mm): the layer's own style;
/// - `on`: shown (its eye and every group's above it open);
/// - `frozen`, `locked`: by itself or a group above; `plot`: printed with
///   every group above;
/// - `empty`: holding no objects (a layer drawn from a service is never empty).
///
/// Refusals (`CommandError.code`): `invalid_pattern` (`name`),
/// `invalid_color`, `invalid_line_weight`, `layer_not_found` (`group`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersFind {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<LineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub on: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frozen: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plot: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub empty: Option<bool>,
}

/// A layer found, as the Katmanlar panel shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerFound {
    pub id: String,
    pub name: String,
    /// Its names from the top of the tree, “Kadastro / Parsel”.
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    pub color: String,
    pub line_type: LineType,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: f64,
    pub on: bool,
    pub frozen: bool,
    pub locked: bool,
    pub plot: bool,
}

/// Output of `cad.layers.find` v1: the layers found, in the tree's order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersFound {
    pub layers: Vec<LayerFound>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

// ── cad.layers.translate ────────────────────────────────────────────────────

/// Merges layers into a standard's by rules (Katmanları çevir).
pub const CAD_LAYERS_TRANSLATE: &str = "cad.layers.translate";
pub const CAD_LAYERS_TRANSLATE_VERSION: u32 = 1;

/// One rule: the layers whose names match `from` go to the layer named `to`,
/// which takes the given style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TranslateRule {
    /// A name pattern (`pattern_matches`).
    pub from: String,
    /// The standard layer's name.
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<LineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
}

/// Input of `cad.layers.translate` v1 (docs/adr/0311 §5), AutoCAD's LAYTRANS:
///
/// - every layer holding objects (not a group, not drawn from a service)
///   takes the first rule whose `from` matches its name, unless its name is
///   that rule's `to` (folded): it is the standard layer itself;
/// - a rule's standard layer is the first layer holding objects named `to`
///   (folded); when there is none and a layer takes the rule, it is added
///   last in `parent` (absent: the top), named `to` as written, in the
///   rule's style over its first source's; an existing one takes the rule's
///   given style;
/// - the sources' objects go to their standard layer, the sources are
///   removed; a source that is the active layer gives that to its standard
///   layer; all of it one undo step “Katmanları çevir”;
/// - a rule no layer takes changes nothing; nothing taken writes nothing
///   (`changed` false). Its plan is the preview: what goes where, with
///   the objects' count, the layers to add and to restyle.
///
/// Refusals (`CommandError.code`), checked in this order: `no_rules`,
/// `empty_name` (`rules[i].to`), `invalid_pattern` (`rules[i].from`),
/// `invalid_color`, `invalid_line_weight` (`rules[i]…`); then
/// `invalid_revision`, `revision_conflict` (status `conflict`);
/// `layer_not_found`, `not_a_group` (`parent`); `service_layer` (a rule's
/// `to` names a layer drawn from a service); `layer_locked` (a source or a
/// standard layer taking part locked, by itself or a group above: nothing
/// is written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersTranslate {
    pub rules: Vec<TranslateRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parent: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A layer translated: where it went and how many objects with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerTranslation {
    /// The source layer's id and name.
    pub layer: String,
    pub name: String,
    /// The rule it took, by its place in `rules`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rule: u32,
    /// The standard layer's id and name.
    pub to: String,
    pub to_name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
}

/// Output of `cad.layers.translate` v1, and its plan (the preview).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersTranslated {
    pub changed: bool,
    /// The sources in the tree's order.
    pub translations: Vec<LayerTranslation>,
    /// The standard layers added and restyled, by id.
    pub created: Vec<String>,
    pub restyled: Vec<String>,
    /// The objects moved.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_read_as_wcmatch_does() {
        for (p, n, want) in [
            ("A-*", "A-DUVAR", true),
            ("a-*", "A-DUVAR", true),
            ("A-*", "B-DUVAR", false),
            ("*duvar*", "Taşıyıcı DUVAR", true),
            ("ı*", "Işık", true),
            ("i*", "İç", true),
            ("i*", "Işık", false),
            ("kat#", "Kat3", true),
            ("kat#", "KatA", false),
            ("@-*", "A-1", true),
            ("@-*", "1-1", false),
            ("a.b", "a_b", true),
            ("a.b", "a1b", false),
            ("?", "A", true),
            ("?", "AB", false),
            ("[abc]*", "Bina", true),
            ("[~abc]*", "Bina", false),
            ("[a-c]x", "bx", true),
            ("~A-*", "B-1", true),
            ("~A-*", "A-1", false),
            ("A-*,B-*", "B-1", true),
            ("`*", "*", true),
            ("`*", "a", false),
            ("*", "", true),
            ("[a,b]", ",", true),
        ] {
            assert_eq!(pattern_matches(p, n), want, "{p} {n}");
        }
        assert!(pattern_problem("[ab").is_some());
        assert!(pattern_problem(" ").is_some());
        assert!(pattern_problem("a`").is_some());
        assert!(pattern_problem("A-*,[x]").is_none());
    }
}
