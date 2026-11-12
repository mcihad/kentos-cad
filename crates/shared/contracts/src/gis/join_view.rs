//! Birleşik görünüm (docs/adr/0527): a read-only view of a layer's records each beside the records of
//! another layer whose key matches its own, computed from the drawing whenever it is read. A project
//! setting (`.kcad` schema 89, `joinViews`): what joins with what, never the rows; the rows are the
//! drawing's at the moment they are read, so a view whose layer or key is gone says so instead of
//! showing a copy. Its joined records may be a source's table instead of a layer (a GeoPackage's or a PostGIS
//! database's, read whole through the provider whenever the view is), and it may sum each record's joined records in
//! columns of its own (`.kcad` schema 105, docs/adr/0632).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The key that is the object's persistent id (a relation's parent; docs/adr/0405).
pub const UID_KEY: &str = "$uid";

/// The most join views a project holds, and the most fields a view names.
pub const MOST_JOIN_VIEWS: usize = 50;
pub const MOST_JOIN_FIELDS: usize = 200;
/// The most summing columns a view has (docs/adr/0632 §2).
pub const MOST_JOIN_AGGREGATES: usize = 20;

/// A table the joined records are read from instead of a layer (docs/adr/0632 §1): a GeoPackage file's (its path) or
/// a PostgreSQL database's (its address, without a password) table, view or read-only query, read whole through its
/// provider whenever the view is read; never a copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinSource {
    pub path: String,
    pub table: String,
}

/// What a summing column does with a record's joined records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum JoinHow {
    /// The joined records (with a field: those with a value in it).
    Count,
    /// The numbers' exact sum, least, greatest and mean (`kentos.statistics/1`).
    Sum,
    Min,
    Max,
    Mean,
}

/// A column summing each record's joined records (docs/adr/0632 §2): the owners' count, their shares' sum.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JoinAggregate {
    /// The heading; trimmed, 1–64 characters, unique among the view's.
    pub name: String,
    pub how: JoinHow,
    /// The joined records' field (a field's key; a source's column): none only for `count`, which then counts the
    /// matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
}

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
    /// The joined layer's id (the main one itself for a tree); empty when the records are a source's table.
    pub with: String,
    /// The joined layer's key: a field's key, or `$uid`; a source's column, or `$uid` (its records' lasting ids).
    pub with_key: String,
    /// Written only when not `all`.
    #[serde(default, skip_serializing_if = "JoinKeep::is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<JoinKeep>", optional))]
    pub keep: JoinKeep,
    /// The joined layer's fields the view shows, in order (keys); none: all of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
    /// The joined records' table, read through its provider (docs/adr/0632 §1); `with` is then empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<JoinSource>,
    /// Columns summing each record's joined records, after the joined fields (docs/adr/0632 §2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<JoinAggregate>>", optional))]
    pub aggregates: Vec<JoinAggregate>,
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
        ("birleşen anahtarı", &v.with_key),
    ] {
        if value.is_empty() {
            return Some(format!("“{}” görünümünün {what} yazılmamış.", v.name));
        }
    }
    match &v.source {
        None if v.with.is_empty() => {
            return Some(format!(
                "“{}” görünümünün birleşen katmanı yazılmamış (ya da kaynak tablosu: source).",
                v.name
            ));
        }
        Some(_) if !v.with.is_empty() => {
            return Some(format!(
                "“{}” görünümü ya bir katmanla (with) ya bir kaynak tablosuyla (source) birleşir, ikisiyle değil.",
                v.name
            ));
        }
        Some(src) => {
            if src.path.trim().is_empty() || src.table.trim().is_empty() {
                return Some(format!(
                    "“{}” görünümünün kaynak tablosunun yolu ya da adı yazılmamış.",
                    v.name
                ));
            }
            if address_has_password(&src.path) {
                return Some(format!(
                    "“{}” görünümünün kaynak adresinde parola var: proje parola taşımaz; parola bağlantının kaydındadır.",
                    v.name
                ));
            }
        }
        None => {}
    }
    if v.aggregates.len() > MOST_JOIN_AGGREGATES {
        return Some(format!(
            "“{}” görünümünde en çok {MOST_JOIN_AGGREGATES} toplama sütunu olur; {} verildi.",
            v.name,
            v.aggregates.len()
        ));
    }
    for (i, a) in v.aggregates.iter().enumerate() {
        if !crate::form_name_ok(&a.name) {
            return Some(format!(
                "“{}” görünümünün toplama sütunlarından birinin adı boş, baş ya da sonda boşluklu ya da 64 karakterden uzun.",
                v.name
            ));
        }
        if v.aggregates[..i].iter().any(|b| b.name == a.name) {
            return Some(format!(
                "“{}” görünümünde “{}” toplama sütunu iki kez var.",
                v.name, a.name
            ));
        }
        match (&a.field, a.how) {
            (None, JoinHow::Count) => {}
            (None, _) => {
                return Some(format!(
                    "“{}” toplama sütunu bir alan ister (field): toplam, en küçük, en büyük ve ortalama bir alanın sayılarıdır.",
                    a.name
                ));
            }
            (Some(f), _) if f.is_empty() => {
                return Some(format!("“{}” toplama sütununun alanı boş.", a.name));
            }
            _ => {}
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

/// Whether a PostgreSQL address carries a password (`postgresql://kullanıcı:parola@…`).
fn address_has_password(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("postgresql://")
        .or_else(|| lower.strip_prefix("postgres://"))
    else {
        return false;
    };
    rest.split_once('@')
        .is_some_and(|(user, _)| user.contains(':'))
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
            source: None,
            aggregates: vec![],
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
