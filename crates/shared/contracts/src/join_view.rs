//! Birleşik görünüm (docs/adr/0527): a read-only view of a layer's records each beside the records of
//! another layer whose key matches its own, computed from the drawing whenever it is read. A project
//! setting (`.kcad` schema 89, `joinViews`): what joins with what, never the rows; the rows are the
//! drawing's at the moment they are read, so a view whose layer or key is gone says so instead of
//! showing a copy.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The key that is the object's persistent id (a relation's parent; docs/adr/0405).
pub const UID_KEY: &str = "$uid";

/// The most join views a project holds, and the most fields a view names.
pub const MOST_JOIN_VIEWS: usize = 50;
pub const MOST_JOIN_FIELDS: usize = 200;

/// Which records of the layer a view shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum JoinKeep {
    /// Every record, the ones without a match with empty cells (a left join).
    #[default]
    All,
    /// Only the records with a match (an inner join).
    Matched,
}

impl JoinKeep {
    pub fn is_default(&self) -> bool {
        *self == JoinKeep::All
    }
}

/// A join view: the layer's records by `key` with the `with` layer's by `withKey`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinView {
    /// Letters, digits, `-`, `_`, `.`; 1–64; unique in the project.
    pub id: String,
    /// What the table's layer list shows; trimmed, 1–64 characters.
    pub name: String,
    /// The main layer's id: one row (or more) a record of it.
    pub layer: String,
    /// The main layer's key: a field's key, or `$uid`.
    pub key: String,
    /// The joined layer's id (the main one itself for a tree).
    pub with: String,
    /// The joined layer's key: a field's key, or `$uid`.
    pub with_key: String,
    /// Written only when not `all`.
    #[serde(default, skip_serializing_if = "JoinKeep::is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<JoinKeep>", optional))]
    pub keep: JoinKeep,
    /// The joined layer's fields the view shows, in order (keys); none: all of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
}

/// What is wrong with a view by its shape (no drawing needed); none when it holds.
pub fn join_view_problem(v: &JoinView) -> Option<String> {
    if !crate::form_id_ok(&v.id) {
        return Some(format!(
            "“{}” bir görünüm kimliği olamaz: 1–64 harf, rakam, -, _ ya da . olmalı.",
            v.id
        ));
    }
    if !crate::form_name_ok(&v.name) {
        return Some(format!(
            "“{}” görünümünün adı boş, baş ya da sonda boşluklu, 64 karakterden uzun ya da denetim karakterli.",
            v.id
        ));
    }
    for (what, value) in [
        ("katmanı", &v.layer),
        ("anahtarı", &v.key),
        ("birleşen katmanı", &v.with),
        ("birleşen anahtarı", &v.with_key),
    ] {
        if value.is_empty() {
            return Some(format!("“{}” görünümünün {what} yazılmamış.", v.name));
        }
    }
    if v.fields.len() > MOST_JOIN_FIELDS {
        return Some(format!(
            "“{}” görünümü en çok {MOST_JOIN_FIELDS} alan gösterir; {} verildi.",
            v.name,
            v.fields.len()
        ));
    }
    for (i, f) in v.fields.iter().enumerate() {
        if f.is_empty() {
            return Some(format!("“{}” görünümünün alanlarından biri boş.", v.name));
        }
        if v.fields[..i].contains(f) {
            return Some(format!(
                "“{}” görünümünde “{f}” alanı iki kez yazılmış.",
                v.name
            ));
        }
    }
    None
}

/// What is wrong with a project's views (KCAD, the server): at most 50, each holding, ids unique.
pub fn join_views_problem(views: &[JoinView]) -> Option<String> {
    if views.len() > MOST_JOIN_VIEWS {
        return Some(format!(
            "Projede en çok {MOST_JOIN_VIEWS} birleşik görünüm olur; {} var.",
            views.len()
        ));
    }
    for (i, v) in views.iter().enumerate() {
        if let Some(p) = join_view_problem(v) {
            return Some(p);
        }
        if views[..i].iter().any(|w| w.id == v.id) {
            return Some(format!(
                "“{}” kimlikli iki birleşik görünüm var; kimlik bir kez olur.",
                v.id
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> JoinView {
        JoinView {
            id: "parsel-malik".into(),
            name: "Parseller ve malikleri".into(),
            layer: "parsel".into(),
            key: "no".into(),
            with: "malik".into(),
            with_key: "parsel".into(),
            keep: JoinKeep::All,
            fields: vec![],
        }
    }

    #[test]
    fn a_view_holds_by_its_shape() {
        assert_eq!(join_view_problem(&view()), None);
        let json = serde_json::to_value(view()).expect("writes");
        assert!(
            json.get("keep").is_none() && json.get("fields").is_none(),
            "{json}"
        );
        let mut v = view();
        v.id = "bir görünüm".into();
        assert!(join_view_problem(&v).is_some());
        let mut v = view();
        v.key = String::new();
        assert!(join_view_problem(&v).unwrap().contains("anahtarı"));
        let mut v = view();
        v.fields = vec!["ad".into(), "ad".into()];
        assert!(join_view_problem(&v).unwrap().contains("iki kez"));
        assert!(
            join_views_problem(&[view(), view()])
                .unwrap()
                .contains("iki")
        );
    }
}
