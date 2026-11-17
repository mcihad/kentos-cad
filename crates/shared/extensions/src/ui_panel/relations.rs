//! A panel's relation components (plan/eklenti.md ADIM 22, the inventory's "ilişki başvurusu" and "ilişki listesi";
//! docs/adr/8057 §67), over the project's relations (a parent layer, a child layer, the child's field holding its
//! parent's persistent id; docs/adr/0405), as the drawing holds them now:
//!
//! - **`relationPicker`:** a record of a relation's parent layer, chosen by its words (its `display` field, else its
//!   label, else its first field with a value); its value the record's persistent id, as the child's field holds it.
//! - **`relationList`:** the records a relation ties to the record the component `of` holds (a feature picker's first
//!   object, a relation picker's record): the child layer's records naming it, a row each with its `columns`. Read
//!   from the drawing, no Python: a record added, changed or deleted shows at once; a row's click shows the record.
//!
//! The relation is named by its id: one the extension's `schema.json` installs, or one of the user's project. Kapı 1
//! checks what the panel can say; a relation the open project lacks is said in the panel.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

/// The most columns a relation list shows.
pub const MOST_COLUMNS: usize = 6;

/// Kapı 1's problems with relation component `c` at `at` (`/body/<n>`), each with its JSON pointer: its relation an
/// id, a list's `of` one of the panel's feature or relation pickers (`parents`), its columns and `display` field names.
pub fn check(
    kind: &str,
    c: &Map<String, Value>,
    at: &str,
    parents: &BTreeSet<&str>,
    add: &mut impl FnMut(String, String),
) {
    if !c.get("relation").and_then(Value::as_str).is_some_and(is_id) {
        add(
            format!("{at}/relation"),
            "relation projenin bir ilişkisinin kimliği olmalı (harf, rakam, -, _, .; ör. eklentinin schema.json'unda \
             kurduğu)."
                .to_owned(),
        );
    }
    if kind == "relationPicker" {
        if c.get("display")
            .is_some_and(|d| !d.as_str().is_some_and(is_name))
        {
            add(
                format!("{at}/display"),
                "display üst katmanın, kaydı adlandıran alanının adı olmalı.".to_owned(),
            );
        }
        return;
    }
    if !c
        .get("of")
        .and_then(Value::as_str)
        .is_some_and(|of| parents.contains(of))
    {
        add(
            format!("{at}/of"),
            "of bu paneldeki bir nesne seçicinin (featurePicker) ya da ilişki seçicinin (relationPicker) kimliği \
             olmalı: liste onun tuttuğu kaydın ilişkili kayıtlarıdır."
                .to_owned(),
        );
    }
    match c.get("columns") {
        None => {}
        Some(Value::Array(columns)) if columns.len() <= MOST_COLUMNS => {
            for (i, column) in columns.iter().enumerate() {
                if !column.as_str().is_some_and(is_name) {
                    add(
                        format!("{at}/columns/{i}"),
                        "Sütun alt katmanın bir alanının adı olmalı.".to_owned(),
                    );
                }
            }
        }
        Some(_) => add(
            format!("{at}/columns"),
            format!(
                "columns alt katmanın gösterilecek alanlarının adları olmalı, en çok {MOST_COLUMNS} tane."
            ),
        ),
    }
}

/// A relation's id as the project writes it: letters, digits, `-`, `_`, `.`; 1–64 (docs/adr/0405).
fn is_id(id: &str) -> bool {
    (1..=64).contains(&id.chars().count())
        && id
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// A field's name: not empty, one line.
fn is_name(name: &str) -> bool {
    !name.trim().is_empty() && !name.contains(['\n', '\r'])
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A picker names a relation and may name its display field; a list names its relation, the picker its parent is
    /// in, and at most six columns; each mistake is said where it is written.
    #[test]
    fn relation_components_name_their_relation_and_parent() {
        let parents: BTreeSet<&str> = ["tesis"].into();
        let said = |kind: &str, c: Value| -> Vec<String> {
            let mut out = Vec::new();
            check(
                kind,
                c.as_object().expect("a component"),
                "/body/1",
                &parents,
                &mut |at, _| out.push(at),
            );
            out
        };
        assert!(
            said(
                "relationPicker",
                json!({ "relation": "bakim-tesis", "display": "ad" })
            )
            .is_empty()
        );
        assert!(
            said(
                "relationList",
                json!({ "relation": "bakim-tesis", "of": "tesis", "columns": ["tarih"] })
            )
            .is_empty()
        );
        assert_eq!(
            said("relationPicker", json!({ "relation": "bakım tesis" })),
            ["/body/1/relation"]
        );
        assert_eq!(
            said("relationPicker", json!({ "relation": "r", "display": "" })),
            ["/body/1/display"]
        );
        assert_eq!(
            said("relationList", json!({ "relation": "r", "of": "yok" })),
            ["/body/1/of"]
        );
        assert_eq!(
            said(
                "relationList",
                json!({ "relation": "r", "of": "tesis", "columns": "tarih" })
            ),
            ["/body/1/columns"]
        );
        assert_eq!(
            said(
                "relationList",
                json!({ "relation": "r", "of": "tesis", "columns": ["tarih", 3] })
            ),
            ["/body/1/columns/1"]
        );
        assert_eq!(
            said(
                "relationList",
                json!({ "relation": "r", "of": "tesis", "columns": ["a", "b", "c", "d", "e", "f", "g"] })
            ),
            ["/body/1/columns"]
        );
    }
}
