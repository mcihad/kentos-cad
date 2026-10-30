//! A layer's indexes (Dizinler, docs/adr/0758): what its records are looked up by, one field or several in order, as
//! ArcGIS's attribute index and a database's CREATE INDEX are. A unique one is the layer's key: no two records hold the
//! same values in its fields, whatever writes them (a form, the table, a command, an import, Python or an agent). An
//! empty value is no value, as SQL's NULL: a set with one is not compared unless the index takes empty values as the
//! same (PostgreSQL's NULLS NOT DISTINCT). A GeoPackage the layer is written to gets its indexes, one read brings them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::fields::{LayerField, LayerFieldKind, fold, js_trim};

/// A layer holds at most this many indexes (SQLite and PostgreSQL take more; a dataset needing more is not one).
pub const MOST_LAYER_INDEXES: usize = 64;
/// An index's most fields: PostgreSQL's “Indexes can have up to 32 columns”.
pub const MOST_INDEX_FIELDS: usize = 32;

/// An index of a layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerIndex {
    /// Its name, once in the layer (the Turkish way, case aside): what a refusal and a file call it (1–64
    /// characters).
    pub name: String,
    /// The fields' keys, in order (1–32, each once; a field holding bytes is not indexed).
    pub fields: Vec<String>,
    /// The layer's key: no two records hold the same values in its fields; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub unique: bool,
    /// Empty values are the same value: a set with one is compared as the others (PostgreSQL's NULLS NOT DISTINCT);
    /// only with `unique`; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub nulls_equal: bool,
}

fn bad_name(s: &str) -> bool {
    js_trim(s).is_empty()
        || js_trim(s) != s
        || s.chars().count() > crate::fields::MAX_NAME
        || s.chars().any(char::is_control)
}

/// What is wrong with `indexes` on a layer of `fields`: where (`<i>/name`, `<i>/fields/<j>` …) and the sentence.
pub fn layer_indexes_problem(
    fields: &[LayerField],
    indexes: &[LayerIndex],
) -> Option<(String, String)> {
    if indexes.len() > MOST_LAYER_INDEXES {
        return Some((
            String::new(),
            format!(
                "Bir katmanın en çok {MOST_LAYER_INDEXES} dizini olur; {} var.",
                indexes.len()
            ),
        ));
    }
    for (i, x) in indexes.iter().enumerate() {
        let at = i.to_string();
        if bad_name(&x.name) {
            return Some((
                format!("{at}/name"),
                format!(
                    "“{}” bir dizin adı olamaz: 1–64 karakter, başında ve sonunda boşluk olmadan.",
                    x.name
                ),
            ));
        }
        if indexes[..i].iter().any(|o| fold(&o.name) == fold(&x.name)) {
            return Some((
                format!("{at}/name"),
                format!("“{}” dizini iki kez var.", x.name),
            ));
        }
        if x.fields.is_empty() || x.fields.len() > MOST_INDEX_FIELDS {
            return Some((
                format!("{at}/fields"),
                format!(
                    "“{}” dizininin 1–{MOST_INDEX_FIELDS} alanı olur; {} var.",
                    x.name,
                    x.fields.len()
                ),
            ));
        }
        for (j, k) in x.fields.iter().enumerate() {
            let Some(f) = fields.iter().find(|f| f.key() == k) else {
                return Some((
                    format!("{at}/fields/{j}"),
                    format!("“{}” dizininin “{k}” alanı katmanda yok.", x.name),
                ));
            };
            if x.fields[..j].contains(k) {
                return Some((
                    format!("{at}/fields/{j}"),
                    format!("“{}” dizininde “{}” alanı iki kez var.", x.name, f.label()),
                ));
            }
            if f.kind == LayerFieldKind::Binary {
                return Some((
                    format!("{at}/fields/{j}"),
                    format!(
                        "“{}” alanı bayt tutar; dizine girmez (ArcGIS de BLOB alanına dizin kurmaz).",
                        f.label()
                    ),
                ));
            }
        }
        if x.nulls_equal && !x.unique {
            return Some((
                format!("{at}/nullsEqual"),
                format!(
                    "“{}” dizini tekil değil; boşların aynı sayılması yalnız tekil dizinin olur.",
                    x.name
                ),
            ));
        }
        if let Some(o) = indexes[..i].iter().find(|o| o.fields == x.fields) {
            return Some((
                format!("{at}/fields"),
                format!(
                    "“{}” dizini “{}” dizininin alanlarını aynı sırayla tutar; birini bırakın.",
                    x.name, o.name
                ),
            ));
        }
    }
    None
}

/// The indexes as the layer's new `fields` keep them: one naming a field no longer there, or one now holding bytes,
/// goes whole, as PostgreSQL's DROP COLUMN drops the indexes over the column (a unique index left over fewer fields
/// would be a stricter key than the one given). A rename keeps the field's stable key.
pub fn follow_layer_indexes(indexes: &[LayerIndex], fields: &[LayerField]) -> Vec<LayerIndex> {
    indexes
        .iter()
        .filter(|x| {
            x.fields.iter().all(|k| {
                fields
                    .iter()
                    .any(|f| f.key() == k && f.kind != LayerFieldKind::Binary)
            })
        })
        .cloned()
        .collect()
}

/// The values of `index`'s fields in a record (`value` by the field's name; empty text is no value), as the index
/// compares them: none when the set holds an empty value and the index does not take empty values as the same.
pub fn index_values<'v>(
    index: &LayerIndex,
    fields: &[&LayerField],
    mut value: impl FnMut(&str) -> Option<&'v str>,
) -> Option<Vec<Option<&'v str>>> {
    let values: Vec<Option<&'v str>> = fields
        .iter()
        .map(|f| value(&f.name).filter(|v| !js_trim(v).is_empty()))
        .collect();
    (index.nulls_equal || values.iter().all(Option::is_some)).then_some(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, kind: LayerFieldKind) -> LayerField {
        serde_json::from_value(serde_json::json!({ "name": name, "kind": kind.name() }))
            .expect("a field")
    }

    fn index(name: &str, fields: &[&str], unique: bool) -> LayerIndex {
        LayerIndex {
            name: name.into(),
            fields: fields.iter().map(|f| (*f).to_owned()).collect(),
            unique,
            nulls_equal: false,
        }
    }

    #[test]
    fn an_index_naming_a_field_gone_goes_whole() {
        let fields = vec![
            field("Ada", LayerFieldKind::Integer),
            field("Parsel", LayerFieldKind::Integer),
            field("Mahalle", LayerFieldKind::Text),
        ];
        let indexes = vec![
            index("Ada ve parsel", &["Ada", "Parsel"], true),
            index("Mahalle", &["Mahalle"], false),
        ];
        assert_eq!(follow_layer_indexes(&indexes, &fields), indexes);
        // Parsel goes: the key over Ada and Parsel goes with it, not a stricter key over Ada alone.
        let fewer = vec![fields[0].clone(), fields[2].clone()];
        assert_eq!(
            follow_layer_indexes(&indexes, &fewer),
            vec![index("Mahalle", &["Mahalle"], false)]
        );
        // Mahalle comes to hold bytes: its index goes.
        let bytes = vec![
            fields[0].clone(),
            fields[1].clone(),
            field("Mahalle", LayerFieldKind::Binary),
        ];
        assert_eq!(
            follow_layer_indexes(&indexes, &bytes),
            vec![index("Ada ve parsel", &["Ada", "Parsel"], true)]
        );
    }

    #[test]
    fn values_are_compared_with_empty_ones_as_none() {
        let fields = [
            field("Ada", LayerFieldKind::Integer),
            field("Parsel", LayerFieldKind::Integer),
        ];
        let refs: Vec<&LayerField> = fields.iter().collect();
        let mut x = index("Ada ve parsel", &["Ada", "Parsel"], true);
        let attrs = |p: Option<&'static str>| {
            move |n: &str| match n {
                "Ada" => Some("101"),
                _ => p,
            }
        };
        assert_eq!(
            index_values(&x, &refs, attrs(Some("5"))),
            Some(vec![Some("101"), Some("5")])
        );
        assert_eq!(index_values(&x, &refs, attrs(Some("  "))), None);
        assert_eq!(index_values(&x, &refs, attrs(None)), None);
        x.nulls_equal = true;
        assert_eq!(
            index_values(&x, &refs, attrs(Some(" "))),
            Some(vec![Some("101"), None])
        );
    }
}
