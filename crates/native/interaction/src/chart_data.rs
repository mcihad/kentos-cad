//! A chart's data (docs/adr/0227 §2; the web's `model/chartData.ts`), read
//! from a layer's objects or an İşlemler table for the shared core's
//! `ops::charts`, which also says what to chart first (`specs`). A layer's
//! columns are its fields in order (their aliases; a value list sorts by
//! what it shows), then the keys no field names in the natural order (their
//! kind the one Verilerden al gives every object's value), then the
//! geometry's Alan (m²) and Uzunluk (m) from the store's measure records. A
//! cell is its value's canonical text and what it shows, empty, or refused
//! as written.

use std::collections::BTreeMap;

use kentos_contracts::{
    Entity, LayerField, LayerFieldKind, check_value, display_value, infer_field, js_trim,
};
use kentos_geometry_core::ops::charts::data::{Cell, Column, Kind, geometry_columns};
use kentos_geometry_core::ops::point_editor::natural_order;

/// A field's kind as a column's.
pub fn kind_of(f: &LayerField) -> Kind {
    match f.kind {
        LayerFieldKind::Integer | LayerFieldKind::Decimal => Kind::Number,
        LayerFieldKind::Text => Kind::Text,
        LayerFieldKind::Date => Kind::Date,
        LayerFieldKind::Boolean => Kind::Boolean,
    }
}

/// A value of `f` as a cell: its canonical text and what it shows when that
/// differs; empty; or refused as written (a blank one is empty).
pub fn cell_of(f: &LayerField, raw: Option<&str>) -> Cell {
    match check_value(f, raw.unwrap_or_default()) {
        Err(_) => match raw.filter(|r| !js_trim(r).is_empty()) {
            Some(r) => Cell::Invalid(r.to_owned()),
            None => Cell::Empty,
        },
        Ok(v) if v.is_empty() => Cell::Empty,
        // Only a value list, a date and yes or no show otherwise than their canonical text.
        Ok(v)
            if f.values.as_ref().is_none_or(Vec::is_empty)
                && !matches!(f.kind, LayerFieldKind::Date | LayerFieldKind::Boolean) =>
        {
            Cell::Value(v, None)
        }
        Ok(v) => {
            let shown = display_value(f, &v);
            let shown = (shown != v).then_some(shown);
            Cell::Value(v, shown)
        }
    }
}

fn column(f: &LayerField, label: String, entities: &[&Entity]) -> Column {
    Column {
        key: f.name.clone(),
        label,
        kind: kind_of(f),
        labelled: f.values.as_ref().is_some_and(|v| !v.is_empty()),
        cells: entities
            .iter()
            .map(|e| cell_of(f, e.base().attrs.get(&f.name).map(String::as_str)))
            .collect(),
    }
}

/// A layer's columns: its fields, then its objects' other keys (their kind
/// inferred from every object), then the geometry's two from `records` (the
/// store's measure records of the objects, one a row; none: no geometry).
pub fn layer_columns(
    fields: &[LayerField],
    entities: &[&Entity],
    records: Option<&[f64]>,
) -> Vec<Column> {
    let named: std::collections::HashSet<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let mut others: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    for e in entities {
        for (k, v) in &e.base().attrs {
            if named.contains(k.as_str()) {
                continue;
            }
            others
                .entry(k.clone())
                .or_insert_with(|| {
                    order.push(k.clone());
                    Vec::new()
                })
                .push(v.as_str());
        }
    }
    let mut out: Vec<Column> = fields
        .iter()
        .map(|f| column(f, f.label().to_owned(), entities))
        .collect();
    for i in natural_order(&order) {
        let key = &order[i as usize];
        let f = infer_field(key, others[key].iter().copied());
        out.push(column(&f, key.clone(), entities));
    }
    if let Some(records) = records {
        out.extend(geometry_columns(records));
    }
    out
}

/// An İşlemler table's columns: its headers, their kind inferred from every row.
pub fn table_columns(columns: &[String], rows: &[Vec<String>]) -> Vec<Column> {
    columns
        .iter()
        .enumerate()
        .map(|(j, name)| {
            let f = infer_field(
                name,
                rows.iter().map(|r| r.get(j).map_or("", String::as_str)),
            );
            Column {
                key: name.clone(),
                label: name.clone(),
                kind: kind_of(&f),
                labelled: false,
                cells: rows
                    .iter()
                    .map(|r| cell_of(&f, r.get(j).map(String::as_str)))
                    .collect(),
            }
        })
        .collect()
}
