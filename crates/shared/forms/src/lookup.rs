//! What a form reads of other layers (docs/adr/0404): a field's list of values from another layer
//! and the objects of another layer that refer to a record.
//!
//! - A list is made once, when the form is prepared: the list layer's objects that the filter takes,
//!   each key once (the first object's label), in the labels' natural order. A record narrows it
//!   by the chain (`matches`): an object is offered when each of its matched fields equals the
//!   record's; an empty record field offers nothing.
//! - `$uid` as the key writes the object's persistent id: the record refers to it.
//! - A related item lists the other layer's objects whose field holds the record's persistent id.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{FieldChoice, FormLookup, LOOKUP_UID, LayerField, js_trim};
use kentos_expression::{Unit, Variable, compile_with};

use crate::record::{Attrs, Object, holds, schema_in};

/// Another layer a form reads: its name, its fields and its objects (persistent id, attributes).
#[derive(Clone, Debug, Default)]
pub struct OtherLayer {
    pub name: String,
    pub fields: Vec<LayerField>,
    pub rows: Vec<(String, Attrs)>,
}

/// The other layers a form reads, by their ids.
pub type World = BTreeMap<String, OtherLayer>;

/// A field's list from another layer, made once.
#[derive(Clone, Debug)]
pub(crate) struct List {
    /// Each value: its code, its label and its matched fields' values.
    pub rows: Vec<(String, String, Vec<Option<String>>)>,
    /// The record's fields' names the chain reads, in the matches' order.
    pub record: Vec<String>,
}

impl List {
    /// The values the record is offered.
    pub(crate) fn offered<'a>(
        &'a self,
        attrs: &'a Attrs,
    ) -> Vec<&'a (String, String, Vec<Option<String>>)> {
        let wanted: Vec<Option<&str>> = self
            .record
            .iter()
            .map(|name| {
                attrs
                    .get(name)
                    .map(|v| js_trim(v))
                    .filter(|v| !v.is_empty())
            })
            .collect();
        if wanted.iter().any(Option::is_none) {
            return Vec::new();
        }
        self.rows
            .iter()
            .filter(|(_, _, keys)| keys.iter().zip(&wanted).all(|(k, w)| k.as_deref() == *w))
            .collect()
    }

    /// The record's offered values as choices.
    pub(crate) fn choices(&self, attrs: &Attrs) -> Vec<FieldChoice> {
        self.offered(attrs)
            .into_iter()
            .map(|(code, label, _)| FieldChoice {
                code: code.clone(),
                label: label.clone(),
                retired: false,
                description: None,
                from: None,
                until: None,
            })
            .collect()
    }

    /// The code a typed text names: the text, or the code of the one value labelled so.
    pub(crate) fn code_of<'a>(&'a self, text: &'a str) -> &'a str {
        let t = js_trim(text);
        if self.rows.iter().any(|(code, _, _)| code == t) {
            return t;
        }
        self.rows
            .iter()
            .find(|(_, label, _)| label == t)
            .map_or(t, |(code, _, _)| code.as_str())
    }
}

/// A layer field's name by its key.
fn name_of<'a>(fields: &'a [LayerField], key: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|f| f.key() == key)
        .map(|f| f.name.as_str())
}

/// The value of an object's field (`$uid`: its persistent id), trimmed; none when empty.
fn value<'a>(uid: &'a str, attrs: &'a Attrs, name: Option<&str>, by_uid: bool) -> Option<&'a str> {
    let v = if by_uid {
        uid
    } else {
        attrs.get(name?).map_or("", String::as_str)
    };
    let v = js_trim(v);
    (!v.is_empty()).then_some(v)
}

/// The list of `lookup` over the record's layer `fields` (the preflight said it holds); none when
/// its layer is not in the world.
pub(crate) fn list_of(
    lookup: &FormLookup,
    fields: &[LayerField],
    world: &World,
    variables: &[Variable],
    unit: Unit,
) -> Option<List> {
    let other = world.get(&lookup.layer)?;
    let by_uid = lookup.key == LOOKUP_UID;
    let key = name_of(&other.fields, &lookup.key);
    let label = lookup
        .label
        .as_deref()
        .map(|l| (l == LOOKUP_UID, name_of(&other.fields, l)));
    let matched: Vec<Option<&str>> = lookup
        .matches
        .iter()
        .map(|m| name_of(&other.fields, &m.field))
        .collect();
    let filter = lookup
        .filter
        .as_deref()
        .and_then(|src| compile_with(src, &schema_in(&other.fields, variables, unit)).ok());
    let none = Object::default();
    let mut seen = HashSet::new();
    let mut rows = Vec::new();
    for (uid, attrs) in &other.rows {
        let Some(code) = value(uid, attrs, key, by_uid) else {
            continue;
        };
        if filter.as_ref().is_some_and(|e| !holds(e, attrs, &none)) || !seen.insert(code) {
            continue;
        }
        let shown = match label {
            Some((l_uid, l_name)) => value(uid, attrs, l_name, l_uid).unwrap_or(code),
            None => code,
        };
        let keys = matched
            .iter()
            .map(|name| value(uid, attrs, *name, false).map(str::to_owned))
            .collect();
        rows.push((code.to_owned(), shown.to_owned(), keys));
    }
    rows.sort_by(|a, b| {
        kentos_geometry_core::text::natural::natural_cmp(&a.1, &b.1).then_with(|| a.0.cmp(&b.0))
    });
    Some(List {
        rows,
        record: lookup
            .matches
            .iter()
            .filter_map(|m| name_of(fields, &m.record).map(str::to_owned))
            .collect(),
    })
}

/// Each code of `lookup`'s layer with its label (docs/adr/0404 §2), for showing a stored value:
/// every object, filter and chain aside, the first object's label for a code; none when the layer
/// is not in the world.
pub fn labels_of(lookup: &FormLookup, world: &World) -> BTreeMap<String, String> {
    let Some(other) = world.get(&lookup.layer) else {
        return BTreeMap::new();
    };
    let by_uid = lookup.key == LOOKUP_UID;
    let key = name_of(&other.fields, &lookup.key);
    let label = lookup
        .label
        .as_deref()
        .map(|l| (l == LOOKUP_UID, name_of(&other.fields, l)));
    let mut out = BTreeMap::new();
    for (uid, attrs) in &other.rows {
        let Some(code) = value(uid, attrs, key, by_uid) else {
            continue;
        };
        if out.contains_key(code) {
            continue;
        }
        let shown = match label {
            Some((l_uid, l_name)) => value(uid, attrs, l_name, l_uid).unwrap_or(code),
            None => code,
        };
        out.insert(code.to_owned(), shown.to_owned());
    }
    out
}

/// A related item's objects, made once: each object's persistent id and the id its field holds.
#[derive(Clone, Debug)]
pub(crate) struct Related {
    pub rows: Vec<(String, String)>,
}

impl Related {
    /// The objects that refer to the record, in the layer's order.
    pub(crate) fn of(&self, object: &Object) -> Vec<String> {
        let Some(uid) = object.uid.as_deref() else {
            return Vec::new();
        };
        // A many-to-many field holds several ids (docs/adr/0409 §1); an id has no `;`.
        self.rows
            .iter()
            .filter(|(_, held)| held.split(';').any(|h| js_trim(h) == uid))
            .map(|(own, _)| own.clone())
            .collect()
    }
}

/// The related item over `layer`'s `field`; none when the layer is not in the world.
pub(crate) fn related_of(layer: &str, field: &str, world: &World) -> Option<Related> {
    let other = world.get(layer)?;
    let name = name_of(&other.fields, field);
    Some(Related {
        rows: other
            .rows
            .iter()
            .filter_map(|(uid, attrs)| {
                value(uid, attrs, name, false).map(|v| (uid.clone(), v.to_owned()))
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let row = |uid: &str, code: &str| {
            (
                uid.to_owned(),
                [("Kod".to_owned(), code.to_owned())].into_iter().collect(),
            )
        };
        let mut w = World::new();
        w.insert(
            "bina".into(),
            OtherLayer {
                name: "Bina".into(),
                fields: vec![
                    serde_json::from_value(serde_json::json!({ "name": "Kod", "kind": "text" }))
                        .expect("a field"),
                ],
                rows: vec![
                    row("u1", "B-1"),
                    row("u2", " "),
                    row("u3", "B-3"),
                    row("u1", "B-9"),
                ],
            },
        );
        w
    }

    /// docs/adr/0410 §4: a stored code shown by its label, the first object's; an empty label shows
    /// the code; a layer not in the world gives none.
    #[test]
    fn codes_are_labelled_by_the_first_object() {
        let lookup = FormLookup {
            layer: "bina".into(),
            key: LOOKUP_UID.into(),
            label: Some("Kod".into()),
            matches: Vec::new(),
            filter: None,
        };
        let labels = labels_of(&lookup, &world());
        assert_eq!(labels.get("u1").map(String::as_str), Some("B-1"));
        assert_eq!(labels.get("u2").map(String::as_str), Some("u2"));
        assert_eq!(labels.get("u3").map(String::as_str), Some("B-3"));
        let elsewhere = FormLookup {
            layer: "yok".into(),
            ..lookup
        };
        assert!(labels_of(&elsewhere, &world()).is_empty());
    }
}
