//! A layer's record rules (Kayıt kuralları, docs/adr/0438 §1): the dataset's own constraints, each an
//! expression over the record that every write must keep true, whatever writes it (a form, the table,
//! a command, an import, Python or an agent). A form over the layer shows a rule on the fields it names
//! instead of repeating it; a form's own checks stay the form's. A rule whose answer is empty (a value
//! it reads is empty) holds, as an SQL CHECK does; a warning rule is said and the write goes in.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::fields::{LayerField, js_trim};

/// A layer holds at most this many rules.
pub const MOST_LAYER_RULES: usize = 64;
/// A rule's expression's and message's most characters.
pub const MOST_RULE_TEXT: usize = 2000;
pub const MOST_RULE_MESSAGE: usize = 500;

/// A record rule of a layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerRule {
    /// Its id, once in the layer: what a form or a refusal names it by (1–64 characters).
    pub id: String,
    /// Its name as the interface shows it (1–64 characters).
    pub name: String,
    /// The expression that must hold (the record's fields by name, `$alan` and the project's `@`
    /// values).
    pub rule: String,
    /// What a write breaking it is told (1–500 characters).
    pub message: String,
    /// A soft rule: broken, it warns and the write goes in; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub warning: bool,
    /// The fields' keys a form shows it on and a refusal points at, in order; none: the first field it
    /// reads.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
}

fn bad_text(s: &str, most: usize) -> bool {
    js_trim(s).is_empty()
        || js_trim(s) != s
        || s.chars().count() > most
        || s.chars().any(char::is_control)
}

/// What is wrong with `rules` on a layer of `fields` (the expressions' own words are the forms core's):
/// where (`<i>/id`, `<i>/fields/<j>` …) and the sentence.
pub fn layer_rules_problem(fields: &[LayerField], rules: &[LayerRule]) -> Option<(String, String)> {
    if rules.len() > MOST_LAYER_RULES {
        return Some((
            String::new(),
            format!(
                "Bir katmanın en çok {MOST_LAYER_RULES} kayıt kuralı olur; {} var.",
                rules.len()
            ),
        ));
    }
    for (i, r) in rules.iter().enumerate() {
        let at = i.to_string();
        if bad_text(&r.id, crate::fields::MAX_NAME) {
            return Some((
                format!("{at}/id"),
                format!(
                    "“{}” bir kural kimliği olamaz: 1–64 karakter, başında ve sonunda boşluk olmadan.",
                    r.id
                ),
            ));
        }
        if rules[..i].iter().any(|o| o.id == r.id) {
            return Some((
                format!("{at}/id"),
                format!("“{}” kuralı iki kez var.", r.id),
            ));
        }
        if bad_text(&r.name, crate::fields::MAX_NAME) {
            return Some((
                format!("{at}/name"),
                format!(
                    "“{}” kuralının adı 1–64 karakter olmalı, başında ve sonunda boşluk olmadan.",
                    r.id
                ),
            ));
        }
        if js_trim(&r.rule).is_empty() || r.rule.chars().count() > MOST_RULE_TEXT {
            return Some((
                format!("{at}/rule"),
                format!(
                    "“{}” kuralının ifadesi boş olamaz ve en çok {MOST_RULE_TEXT} karakterdir.",
                    r.name
                ),
            ));
        }
        if js_trim(&r.message).is_empty() || r.message.chars().count() > MOST_RULE_MESSAGE {
            return Some((
                format!("{at}/message"),
                format!(
                    "“{}” kuralının iletisi boş olamaz ve en çok {MOST_RULE_MESSAGE} karakterdir.",
                    r.name
                ),
            ));
        }
        for (j, k) in r.fields.iter().enumerate() {
            if !fields.iter().any(|f| f.key() == k) {
                return Some((
                    format!("{at}/fields/{j}"),
                    format!(
                        "“{}” kuralının gösterildiği “{k}” alanı katmanda yok.",
                        r.name
                    ),
                ));
            }
            if r.fields[..j].contains(k) {
                return Some((
                    format!("{at}/fields/{j}"),
                    format!("“{}” kuralında “{k}” alanı iki kez var.", r.name),
                ));
            }
        }
    }
    None
}

/// The rules as the layer's new `fields` keep them: a field key no longer there leaves the rule's
/// list (a rename keeps its stable key; the expressions follow it elsewhere).
pub fn follow_layer_rules(rules: &[LayerRule], fields: &[LayerField]) -> Vec<LayerRule> {
    rules
        .iter()
        .map(|r| LayerRule {
            fields: r
                .fields
                .iter()
                .filter(|k| fields.iter().any(|f| f.key() == *k))
                .cloned()
                .collect(),
            ..r.clone()
        })
        .collect()
}
