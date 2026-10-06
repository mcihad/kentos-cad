//! What Veride ara reads of a drawing (docs/adr/0178 §1, §3; the web's
//! `model/dataSearch.ts`): each object's words as a record for the core's
//! `ops::data_search` (the matching), which objects a scope takes, the
//! attribute names the drawing carries, and the place a typed coordinate
//! names. The panel is the desktop's `search`. `fixtures/search/v1` holds
//! both platforms to the same records and names.

use std::collections::{HashMap, HashSet};

use kentos_contracts::{BlockId, Entity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::data_search::Record;
use kentos_geometry_core::ops::point_editor::natural_order;
use kentos_geometry_core::tools::point_text::{PointText, js_trim, parse_point_text};

use crate::Vec2;

/// An object's kind as the interface names it (the web's `ENTITY_KIND_LABEL`).
pub fn kind_name(e: &Entity) -> &'static str {
    match e {
        Entity::Point(_) => "Nokta",
        Entity::Line(_) => "Çizgi",
        Entity::Polyline(_) => "Çoklu çizgi",
        Entity::Polygon(_) => "Kapalı alan",
        Entity::Circle(_) => "Daire",
        Entity::Arc(_) => "Yay",
        Entity::Ellipse(_) => "Elips",
        Entity::Spline(_) => "Eğri",
        Entity::Xline(_) => "Yardımcı çizgi",
        Entity::Ray(_) => "Işın",
        Entity::Text(_) => "Yazı",
        Entity::Dimension(_) => "Ölçü",
        Entity::Hatch(_) => "Tarama",
        Entity::Insert(_) => "Blok",
        Entity::Leader(_) => "Kılavuz",
        Entity::Table(_) => "Tablo",
    }
}

fn blank(v: &Option<String>) -> bool {
    v.as_deref().is_none_or(|v| js_trim(v).is_empty())
}

/// The record of an object: its kind's name, its layer's path, its label as it
/// is stored, its words (a text's text, a leader's note, a dimension's own
/// text), the name of its insert's block (none for an unknown block) and its
/// attributes as they are stored; none when it has no label, text, block name
/// or attribute value that is not blank.
pub fn record_of(
    e: &Entity,
    layer_path: &str,
    block_name: impl FnOnce(&BlockId) -> Option<String>,
) -> Option<Record> {
    let base = e.base();
    let label = base.label.clone();
    let text = match e {
        Entity::Text(t) => Some(t.text.clone()),
        Entity::Leader(l) => l.text.clone(),
        Entity::Dimension(d) => d.text.clone(),
        // A table's words, cell by cell, row by row, each on its own line (docs/adr/0184 §8).
        Entity::Table(t) => Some(
            t.cells
                .iter()
                .flatten()
                .filter(|w| !w.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        _ => None,
    };
    let block = match e {
        Entity::Insert(i) => block_name(&i.block),
        _ => None,
    };
    let attrs: Vec<[String; 2]> = base
        .attrs
        .iter()
        .map(|(k, v)| [k.clone(), v.clone()])
        .collect();
    if blank(&label)
        && blank(&text)
        && blank(&block)
        && attrs.iter().all(|[_, v]| js_trim(v).is_empty())
    {
        return None;
    }
    Some(Record {
        kind: kind_name(e).to_owned(),
        layer: layer_path.to_owned(),
        label,
        text,
        block,
        attrs,
    })
}

/// The objects of a drawing that have something to find, in the drawing's
/// order, with their records.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Index {
    /// Each object's slot.
    pub slots: Vec<Slot>,
    /// Each object's layer.
    pub layer_ids: Vec<String>,
    pub records: Vec<Record>,
}

pub fn index(doc: &Document) -> Index {
    let mut paths: HashMap<String, String> = HashMap::new();
    let layers = doc.layers();
    let names: HashMap<BlockId, String> = doc
        .blocks()
        .iter()
        .map(|b| (b.id, b.name.clone()))
        .collect();
    let mut out = Index::default();
    for e in doc.entities() {
        let base = e.base();
        let path = paths
            .entry(base.layer_id.clone())
            .or_insert_with(|| layers.path(&base.layer_id));
        let Some(record) = record_of(e, path, |id| names.get(id).cloned()) else {
            continue;
        };
        out.slots.push(Slot(base.id));
        out.layer_ids.push(base.layer_id.clone());
        out.records.push(record);
    }
    out
}

/// The index positions a scope takes, in the drawing's order: one layer's
/// objects (none: every layer's) and only the selected ones (none: all).
pub fn in_scope(index: &Index, layer: Option<&str>, selected: Option<&[Slot]>) -> Vec<usize> {
    let selected: Option<HashSet<Slot>> = selected.map(|s| s.iter().copied().collect());
    (0..index.slots.len())
        .filter(|&i| {
            layer.is_none_or(|l| index.layer_ids[i] == l)
                && selected
                    .as_ref()
                    .is_none_or(|s| s.contains(&index.slots[i]))
        })
        .collect()
}

/// The attribute names the records carry with a value that is not blank,
/// each once, in the natural order.
pub fn attribute_names(records: &[Record]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut names = Vec::new();
    for r in records {
        for [name, value] in &r.attrs {
            if !js_trim(value).is_empty() && seen.insert(name.as_str()) {
                names.push(name.clone());
            }
        }
    }
    natural_order(&names)
        .into_iter()
        .map(|i| names[i as usize].clone())
        .collect()
}

/// The place typed text names, in metres (`to_metres` turns a coordinate
/// typed in the project's unit into them): an absolute point, east then
/// north, as the Nokta tool reads one; none for anything else (a relative
/// point, a distance or a word).
pub fn typed_place(text: &str, to_metres: impl Fn(f64) -> f64) -> Option<Vec2> {
    match parse_point_text(text)? {
        PointText::Absolute(p) => Some(Vec2::new(to_metres(p.x), to_metres(p.y))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_absolute_point_is_a_place() {
        let id = |v: f64| v;
        assert_eq!(
            typed_place("487012.5,4420000", id),
            Some(Vec2::new(487012.5, 4420000.0))
        );
        assert_eq!(typed_place(" 12 ; -3.5 ", id), Some(Vec2::new(12.0, -3.5)));
        assert_eq!(
            typed_place("1000 2000", |v| v / 1000.0),
            Some(Vec2::new(1.0, 2.0))
        );
        for text in ["@12,5", "12<45", "101", "Ada 101", "12,5,3", "", "1.,2"] {
            assert_eq!(typed_place(text, id), None, "{text}");
        }
    }
}
