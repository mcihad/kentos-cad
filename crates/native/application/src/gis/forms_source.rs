//! A form's related records in a data layer (docs/adr/9165): a related item whose layer is a data layer, tied to the
//! record by a relation's parent key (docs/adr/9135, a code such as a neighbourhood's `M1`), lists the rows its source
//! holds with that code, not only the ones taken into the drawing. The rows are read off the interface's thread by the
//! caller ([`read_related`]); [`world_with`] puts them into the form's world beside the drawing's objects, each under a
//! source id (`kaynak:<key>`), and leaves out a row taken into the drawing (its object is there, as edited). They are
//! shown, not written: a source id is no object of the drawing.

use std::collections::BTreeMap;

use kentos_commands::{FormBody, FormItem, LayerFieldKind, LayerNode, ProjectSettings};
use kentos_data::{Cancel, Condition, Op, Request, Value};
use kentos_domain::Document;
use kentos_forms::{Attrs, World};

use crate::geopackage::attribute_text;

/// The most rows of a data layer a related item reads for one record.
pub const MOST_ROWS: u64 = 500;
/// A source row's id in the form's world: the prefix before its key.
pub const SOURCE_ID: &str = "kaynak:";

/// The rows read for a record's related items, by the data layer's id: each row's key and values.
pub type SourceRows = BTreeMap<String, Vec<(i64, Attrs)>>;

/// What a record's related item asks of a data layer's source: the layer, its field holding the code (by name) and the
/// record's code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceAsk {
    pub layer: String,
    pub field: String,
    pub value: String,
}

/// A source row's id in the form's world.
pub fn source_id(key: i64) -> String {
    format!("{SOURCE_ID}{key}")
}

/// The key of a source row's id; none for an object of the drawing.
pub fn source_key(id: &str) -> Option<i64> {
    id.strip_prefix(SOURCE_ID)?.parse().ok()
}

/// What the related items of `body` on the layer `parent` ask of data layers for the record holding `attrs`: an item
/// over a data layer whose relation ties it by a parent key the record has a value of. An item tied by persistent ids
/// asks nothing: a source's rows do not hold the drawing's ids.
pub fn source_asks(doc: &Document, body: &FormBody, parent: &str, attrs: &Attrs) -> Vec<SourceAsk> {
    let mut out: Vec<SourceAsk> = Vec::new();
    for (_, item) in body.walk() {
        let FormItem::Related { layer, field, .. } = item else {
            continue;
        };
        let Some(child) = doc.layers().get(layer).filter(|n| n.data.is_some()) else {
            continue;
        };
        let Some(relation) = doc.settings().relations.iter().find(|r| {
            r.parent == parent && r.child == *layer && r.field == *field && r.parent_key.is_some()
        }) else {
            continue;
        };
        let Some(by) = relation.parent_key.as_deref() else {
            continue;
        };
        let parent_field = doc
            .layers()
            .get(parent)
            .and_then(|n| n.fields.iter().find(|f| f.key() == by))
            .map_or_else(|| by.to_owned(), |f| f.name.clone());
        let Some(value) = attrs
            .get(&parent_field)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        else {
            continue;
        };
        let Some(child_field) = child
            .fields
            .iter()
            .find(|f| f.key() == field)
            .map(|f| f.name.clone())
        else {
            continue;
        };
        let ask = SourceAsk {
            layer: layer.clone(),
            field: child_field,
            value: value.to_owned(),
        };
        if !out.contains(&ask) {
            out.push(ask);
        }
    }
    out
}

/// The rows of the data layer `node` whose `field` holds `value`, at most [`MOST_ROWS`], by key: their keys and values
/// as the drawing's texts. Read from the source: off the interface's thread.
pub fn read_related(
    node: &LayerNode,
    _settings: &ProjectSettings,
    field: &str,
    value: &str,
) -> Result<Vec<(i64, Attrs)>, String> {
    let Some(data) = &node.data else {
        return Err(format!("“{}” bir veri katmanı değil.", node.name));
    };
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| format!("“{}” tablosu açılamadı: {e}", data.table))?;
    // The code compared as the field holds it: a number field's code as a number.
    let kind = node.fields.iter().find(|f| f.name == field).map(|f| f.kind);
    let compared = match kind {
        Some(LayerFieldKind::Integer | LayerFieldKind::BigInt) => {
            value.parse::<i64>().map(Value::Int).ok()
        }
        Some(LayerFieldKind::Decimal) => value.parse::<f64>().map(Value::Real).ok(),
        _ => Some(Value::Text(value.to_owned())),
    };
    let Some(compared) = compared else {
        return Ok(Vec::new());
    };
    let fields: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
    let request = Request {
        condition: Some(Condition::Compare {
            field: field.to_owned(),
            op: Op::Eq,
            value: compared,
        }),
        columns: Some(fields.clone()),
        geometry: false,
        limit: Some(MOST_ROWS),
        chunk: MOST_ROWS as usize,
        ..Request::default()
    };
    let mut out = Vec::new();
    source
        .features(&data.table, &request, &Cancel::new(), &mut |part| {
            for f in &part {
                let attrs: Attrs = fields
                    .iter()
                    .zip(&f.values)
                    .filter_map(|(n, v)| attribute_text(v).map(|t| (n.clone(), t)))
                    .collect();
                out.push((f.fid, attrs));
            }
            true
        })
        .map_err(|e| format!("“{}” tablosu okunamadı: {e}", data.table))?;
    out.sort_by_key(|(k, _)| *k);
    Ok(out)
}

/// The form's world for `body` with the source rows `rows` beside the drawing's objects: each under its source id,
/// a row taken into the drawing left out (its object is in the world already).
pub fn world_with(doc: &Document, body: &FormBody, rows: &SourceRows) -> World {
    let mut world = doc.form_world(body);
    for (layer, list) in rows {
        let Some(other) = world.get_mut(layer) else {
            continue;
        };
        for (key, attrs) in list {
            if doc.taken_uid(layer, *key).is_none() {
                other.rows.push((source_id(*key), attrs.clone()));
            }
        }
    }
    world
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_id_carries_its_key_and_a_drawing_id_none() {
        assert_eq!(source_id(22), "kaynak:22");
        assert_eq!(source_key("kaynak:22"), Some(22));
        assert_eq!(source_key("0193b6a8-0000-7000-8000-000000000001"), None);
        assert_eq!(source_key("kaynak:x"), None);
    }
}
