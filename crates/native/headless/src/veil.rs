//! What an agent is given of a drawing (docs/adr/0738 §1). A field a form keeps out of examples (`redact`,
//! docs/adr/0443 §2) or a layer marks `withheld` (docs/adr/0638 §5) is read on screen and written by the
//! drawing's owner; no answer to an agent (MCP, the desktop's agents' link) carries its values. The owner's own
//! Python reads the drawing whole: the drawing is theirs (docs/adr/0550 §3).
//!
//! An answer is cut where its shapes hold a field's value: an attribute map (`attrs`, `values`) under the field's
//! name; a join view's cells and sums under a column of the field; a reconciliation's two sides of the field; a
//! statistic of the field. A value of a hidden field quoted in a text (“…”) is written as hidden too: a rule's
//! words name the value they refuse. The command history's rule writes the journal's inputs (docs/adr/0443 §2).
//! What is cut is written [`HIDDEN`]; the summary names the hidden fields.

use std::collections::BTreeSet;

use kentos_domain::Document;
use kentos_native_application::python_call::{HIDDEN, redacted, redacted_names};
use serde_json::{Map, Value};

/// The keys under which a call's texts are what it writes (an object's attribute values, a text's words, a
/// note, a file's path), not what it reads (docs/adr/0743 §1).
pub const WRITTEN: [&str; 9] = [
    "attrs", "values", "value", "text", "content", "note", "notes", "comment", "path",
];

/// The drawing's hidden fields and the values they hold.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Veil {
    names: Vec<String>,
    values: BTreeSet<String>,
}

impl Veil {
    /// The fields `doc` hides from an agent and the values they hold; none when it hides none.
    pub fn of(doc: &Document) -> Option<Veil> {
        let names = redacted_names(doc);
        if names.is_empty() {
            return None;
        }
        let mut values = BTreeSet::new();
        for e in doc.entities() {
            for (k, v) in &e.base().attrs {
                if !v.is_empty() && names.contains(k) {
                    values.insert(v.clone());
                }
            }
        }
        Some(Veil { names, values })
    }

    /// A veil of `names` over `values` (tests and other hosts).
    pub fn new(names: Vec<String>, values: impl IntoIterator<Item = String>) -> Veil {
        Veil {
            names,
            values: values.into_iter().filter(|v| !v.is_empty()).collect(),
        }
    }

    /// The hidden fields' names (and keys).
    pub fn names(&self) -> &[String] {
        &self.names
    }

    fn hides(&self, name: &str) -> bool {
        self.names.iter().any(|n| n == name)
    }

    /// Where an agent's call input names a hidden field (docs/adr/0743 §1): the first place in the input's own
    /// order (keys sorted, `a.b[2]`) and the field. A key that is the field's name names it, wherever it is; a
    /// text names it when it is the name, or an expression or a template (`{…}`) that reads the field. The texts
    /// a call writes ([`WRITTEN`]) are content, not read: only their keys are looked at.
    pub fn named_in(&self, input: &Value) -> Option<(String, String)> {
        let mut path = String::new();
        self.named_at(input, &mut path, false)
    }

    fn named_at(&self, v: &Value, path: &mut String, content: bool) -> Option<(String, String)> {
        match v {
            Value::String(s) if !content => self.named_by(s).map(|f| (path.clone(), f)),
            Value::Array(items) => items.iter().enumerate().find_map(|(i, item)| {
                let len = path.len();
                path.push_str(&format!("[{i}]"));
                let found = self.named_at(item, path, content);
                path.truncate(len);
                found
            }),
            Value::Object(map) => map.iter().find_map(|(k, item)| {
                let len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(k);
                let found = if self.hides(k) {
                    Some((path.clone(), k.clone()))
                } else {
                    self.named_at(item, path, content || WRITTEN.contains(&k.as_str()))
                };
                path.truncate(len);
                found
            }),
            _ => None,
        }
    }

    fn named_by(&self, s: &str) -> Option<String> {
        if let Some(n) = self.names.iter().find(|n| *n == s) {
            return Some(n.clone());
        }
        if s.contains('{')
            && let Some(n) = self
                .names
                .iter()
                .find(|n| kentos_expression::editor::template_names_field(s, n))
        {
            return Some(n.clone());
        }
        let fields = kentos_expression::editor::fields_of(s);
        self.names.iter().find(|n| fields.contains(n)).cloned()
    }

    /// An agent's call refused because it names a hidden field ([`Veil::named_in`]), as the protocol answers.
    pub fn refusal(&self, input: &Value) -> Option<kentos_commands::CommandError> {
        let (at, field) = self.named_in(input)?;
        Some(kentos_commands::CommandError {
            code: kentos_commands::codes::HIDDEN_FIELD.into(),
            message: format!(
                "“{field}” alanı çizimde ajandan gizli ({at}): o alanla süzme, sıralama, hesap ve yazma çizimin \
                 sahibinindir. Çağrıyı bu alanı anmadan yeniden yapın."
            ),
            path: Some(if at.is_empty() {
                "input".into()
            } else {
                format!("input.{at}")
            }),
            ..kentos_commands::CommandError::default()
        })
    }

    /// `answer` as an agent may read it.
    pub fn cut(&self, answer: Value) -> Value {
        match answer {
            Value::Object(map) => Value::Object(self.cut_object(map)),
            Value::Array(items) => Value::Array(items.into_iter().map(|v| self.cut(v)).collect()),
            Value::String(text) => Value::String(self.text(&text)),
            other => other,
        }
    }

    /// A drawing's summary as an agent reads it: cut, with the hidden fields named (`hidden`).
    pub fn summary(&self, summary: Value) -> Value {
        let mut summary = self.cut(summary);
        if let Some(fields) = summary.as_object_mut() {
            fields.insert("hidden".into(), Value::from(self.names.clone()));
        }
        summary
    }

    /// A drawing's journal as an agent reads it: its calls' inputs as the command history writes them
    /// (docs/adr/0443 §2), then cut.
    pub fn journal(&self, journal: Value) -> Value {
        self.cut(redacted(&journal, &self.names))
    }

    /// A text with every hidden value it quotes (“…”) written as hidden.
    pub fn text(&self, text: &str) -> String {
        if self.values.is_empty() || !text.contains('“') {
            return text.to_owned();
        }
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(open) = rest.find('“') {
            let (before, after) = rest.split_at(open + '“'.len_utf8());
            out.push_str(before);
            match after.find('”') {
                Some(close) => {
                    let inner = &after[..close];
                    out.push_str(if self.values.contains(inner) {
                        HIDDEN
                    } else {
                        inner
                    });
                    rest = &after[close..];
                }
                None => {
                    rest = after;
                    break;
                }
            }
        }
        out.push_str(rest);
        out
    }

    fn cut_object(&self, mut map: Map<String, Value>) -> Map<String, Value> {
        self.cut_join(&mut map);
        // A reconciliation's field: its name and the two sides' values (`ConflictField`, docs/adr/0538).
        if map
            .keys()
            .all(|k| matches!(k.as_str(), "name" | "layer" | "table"))
            && map
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| self.hides(n))
        {
            for side in ["layer", "table"] {
                hide(&mut map, side);
            }
        }
        // A statistic of the field (`QueriedStatistic`, docs/adr/0581).
        if map.contains_key("read")
            && map
                .get("field")
                .and_then(Value::as_str)
                .is_some_and(|n| self.hides(n))
        {
            for figure in ["read", "skipped", "sum", "mean", "min", "max", "std"] {
                hide(&mut map, figure);
            }
        }
        map.into_iter()
            .map(|(k, v)| {
                let v = match v {
                    Value::Object(attrs) if k == "attrs" || k == "values" => {
                        Value::Object(self.cut_attrs(attrs))
                    }
                    other => self.cut(other),
                };
                (k, v)
            })
            .collect()
    }

    /// An attribute map with the hidden fields' values hidden.
    fn cut_attrs(&self, attrs: Map<String, Value>) -> Map<String, Value> {
        attrs
            .into_iter()
            .map(|(k, v)| {
                let v = if self.hides(&k) && !v.is_null() {
                    Value::String(HIDDEN.into())
                } else {
                    self.cut(v)
                };
                (k, v)
            })
            .collect()
    }

    /// A join view's answer (`JoinsAnswer`, docs/adr/0527): the cells and sums under a hidden field's column.
    fn cut_join(&self, map: &mut Map<String, Value>) {
        let Some(columns) = map.get("columns").and_then(Value::as_array) else {
            return;
        };
        let hidden: Vec<usize> = columns
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.get("field")
                    .and_then(Value::as_str)
                    .is_some_and(|f| self.hides(f))
            })
            .map(|(i, _)| i)
            .collect();
        if hidden.is_empty() {
            return;
        }
        if let Some(Value::Array(rows)) = map.get_mut("rows") {
            for row in rows {
                if let Some(Value::Array(cells)) = row.get_mut("cells") {
                    for &i in &hidden {
                        if let Some(cell) = cells.get_mut(i).filter(|c| !c.is_null()) {
                            *cell = Value::String(HIDDEN.into());
                        }
                    }
                }
            }
        }
        if let Some(Value::Array(totals)) = map.get_mut("totals") {
            for total in totals {
                let column = total.get("column").and_then(Value::as_u64);
                if column.is_some_and(|c| hidden.contains(&(c as usize)))
                    && let Some(total) = total.as_object_mut()
                {
                    for figure in ["sum", "count", "skipped"] {
                        hide(total, figure);
                    }
                }
            }
        }
    }
}

/// `map`'s `key` written as hidden, when it holds a value.
fn hide(map: &mut Map<String, Value>, key: &str) {
    if let Some(v) = map.get_mut(key).filter(|v| !v.is_null()) {
        *v = Value::String(HIDDEN.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn veil() -> Veil {
        Veil::new(
            vec!["Malik".into(), "TC".into()],
            ["Ali Yılmaz".to_owned(), "12345678901".to_owned()],
        )
    }

    #[test]
    fn an_attribute_map_hides_the_fields_and_keeps_the_rest() {
        let got = veil().cut(
            json!({ "entities": [{ "uid": "u", "entity": { "kind": "point", "id": 1,
            "attrs": { "Malik": "Ali Yılmaz", "Ada": "101", "TC": "12345678901" } } }] }),
        );
        assert_eq!(
            got["entities"][0]["entity"]["attrs"],
            json!({ "Malik": HIDDEN, "Ada": "101", "TC": HIDDEN })
        );
        // A structural key of the same name elsewhere is not an attribute.
        let named = Veil::new(vec!["name".into()], ["x".to_owned()]);
        assert_eq!(
            named.cut(json!({ "name": "Parseller" })),
            json!({ "name": "Parseller" })
        );
    }

    #[test]
    fn a_quoted_hidden_value_is_written_hidden() {
        assert_eq!(
            veil().text("“TC” katmanda tekil olmalı; “12345678901” başka bir nesnede var."),
            format!("“TC” katmanda tekil olmalı; “{HIDDEN}” başka bir nesnede var.")
        );
        // An unclosed quote and a value not hidden stay.
        assert_eq!(veil().text("“Ada” “101"), "“Ada” “101");
    }

    /// fixtures/agent-veil/v1/cases.json, written by scripts/fixtures/agent_veil_cases.py from the ADR without
    /// KentOS code.
    #[test]
    fn the_shared_cases_are_cut_alike() {
        let data: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/agent-veil/v1/cases.json"
        ))
        .expect("the cases");
        assert_eq!(data["hidden"], HIDDEN);
        let cases = data["cases"].as_array().expect("cases");
        assert!(cases.len() >= 20);
        for case in cases {
            let strings = |k: &str| -> Vec<String> {
                serde_json::from_value(case[k].clone()).expect("a list of texts")
            };
            let veil = Veil::new(strings("names"), strings("values"));
            assert_eq!(
                veil.cut(case["answer"].clone()),
                case["cut"],
                "{}",
                case["name"]
            );
        }
    }

    /// fixtures/agent-veil/v1/oracle.json, written by scripts/fixtures/agent_oracle_cases.py from docs/adr/0743 §1
    /// without KentOS code.
    #[test]
    fn the_shared_cases_name_a_hidden_field_alike() {
        let data: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/agent-veil/v1/oracle.json"
        ))
        .expect("the cases");
        let cases = data["cases"].as_array().expect("cases");
        assert!(cases.len() >= 20);
        for case in cases {
            let names: Vec<String> =
                serde_json::from_value(case["names"].clone()).expect("the names");
            let got = Veil::new(names, Vec::new())
                .named_in(&case["input"])
                .map(|(path, field)| json!({ "path": path, "field": field }))
                .unwrap_or(Value::Null);
            assert_eq!(got, case["named"], "{}", case["name"]);
        }
    }

    #[test]
    fn a_join_a_conflict_and_a_statistic_hide_the_field() {
        let got = veil().cut(json!({
            "columns": [{ "name": "Ada", "field": "Ada" }, { "name": "Malik", "field": "Malik" }],
            "rows": [{ "id": "a/b", "left": "a", "cells": ["101", "Ali Yılmaz"] }, { "id": "c/", "left": "c", "cells": ["102", null] }],
            "totals": [{ "column": 1, "sum": "3", "count": 1 }]
        }));
        assert_eq!(got["rows"][0]["cells"], json!(["101", HIDDEN]));
        assert_eq!(got["rows"][1]["cells"], json!(["102", null]));
        assert_eq!(
            got["totals"][0],
            json!({ "column": 1, "sum": HIDDEN, "count": HIDDEN })
        );
        let conflict = veil().cut(json!({ "name": "Malik", "layer": "Ali", "table": "Veli" }));
        assert_eq!(
            conflict,
            json!({ "name": "Malik", "layer": HIDDEN, "table": HIDDEN })
        );
        let stat = veil().cut(json!({ "field": "TC", "read": 3, "skipped": 0, "sum": "6" }));
        assert_eq!(
            stat,
            json!({ "field": "TC", "read": HIDDEN, "skipped": HIDDEN, "sum": HIDDEN })
        );
    }
}
