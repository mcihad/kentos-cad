//! Associative arrays (docs/adr/0300; AutoCAD's associative ARRAY): a
//! block's inserts laid out in rows and columns, round a centre, along a
//! displacement or along a path, kept together as one array. The source is
//! the block definition (`block`), its first place the insert at `p`; the
//! layout is the array command's (`cad_array::ArrayLayout`). Each place has
//! its item: the persistent id of the insert there, none when it was removed
//! by hand, and `kept` when it was changed by hand (a later layout change
//! leaves it where it is). Editing the block changes every item; the items
//! are ordinary inserts, drawn, snapped and selected as any.
//!
//! The array is the document's (`DocumentSnapshotV2.arrays`, `.kcad` schema
//! 53, docs/specs/kcad-v2.md §6.14), as the blocks are; its rules are
//! [`arrays_problem`], held by the file format, the documents and the server.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::cad_array::ArrayLayout;
use crate::entity::Vec2;
use crate::identity::{ArrayId, BlockId, EntityId};

/// Most places an associative array holds (the array command's largest grid).
pub const ARRAY_PLACES_MAX: u32 = 10_000;

/// One place of an associative array.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArrayItem {
    /// The insert at this place; none: removed by hand (it stays removed until the array is reset).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<EntityId>,
    /// Changed by hand (moved, turned, scaled, its block replaced): a layout change leaves it as it is.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub kept: bool,
}

/// An associative array (docs/adr/0300).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AssocArray {
    pub id: ArrayId,
    /// The source: the block definition every item inserts.
    pub block: BlockId,
    /// The items' layer.
    pub layer_id: String,
    /// The first place: where the block's base point goes, its scale (above zero) and turn (radians), as an
    /// insert's.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub p: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mirror: bool,
    /// How the places are laid out; the first is `p` itself. A path array names its path by its persistent id
    /// and follows it.
    pub layout: ArrayLayout,
    /// One per place, in the layout's order (a grid row after row).
    pub items: Vec<ArrayItem>,
}

/// How many places a layout has: rows × columns, or its count.
pub fn array_places(layout: &ArrayLayout) -> u64 {
    match *layout {
        ArrayLayout::Grid { rows, cols, .. } => u64::from(rows) * u64::from(cols),
        ArrayLayout::Polar { count, .. }
        | ArrayLayout::Line { count, .. }
        | ArrayLayout::Path { count, .. } => u64::from(count),
    }
}

/// Where an array breaks the rules: the array at this index (its id, block, placement or layout), or one of
/// its items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayFault {
    Array(usize),
    Item(usize, usize),
}

/// Why `arrays` cannot be kept, and where (docs/adr/0300 §1): an id given twice, a block the drawing does not
/// define (`blocks`), an empty layer, a placement or layout number not finite, a scale not above zero, places
/// out of 2 to [`ARRAY_PLACES_MAX`], a path not a persistent id, a spacing not above zero, items not one per
/// place, or an insert named by two items.
pub fn arrays_problem(
    arrays: &[AssocArray],
    blocks: &[BlockId],
) -> Option<(ArrayFault, &'static str)> {
    problem(arrays, Some(blocks))
}

/// [`arrays_problem`] but for the blocks: what a server holding the arrays as a project's metadata checks, the
/// blocks and the objects being the document's (docs/adr/0300 §5).
pub fn arrays_shape_problem(arrays: &[AssocArray]) -> Option<(ArrayFault, &'static str)> {
    problem(arrays, None)
}

/// The arrays as a drawing made of `blocks` and the objects `uids` can keep them: an array whose block is gone
/// dropped, an item whose insert is gone removed (a database project's arrays after another editor's changes,
/// docs/adr/0300 §5).
pub fn arrays_kept(
    arrays: &[AssocArray],
    blocks: &[BlockId],
    uids: &[EntityId],
) -> Vec<AssocArray> {
    let known: std::collections::HashSet<&EntityId> = uids.iter().collect();
    arrays
        .iter()
        .filter(|a| blocks.contains(&a.block))
        .map(|a| {
            let mut a = a.clone();
            for item in &mut a.items {
                if item.uid.is_some_and(|u| !known.contains(&u)) {
                    *item = ArrayItem::default();
                }
            }
            a
        })
        .collect()
}

fn problem(
    arrays: &[AssocArray],
    blocks: Option<&[BlockId]>,
) -> Option<(ArrayFault, &'static str)> {
    let mut ids = std::collections::BTreeSet::new();
    let mut uids = std::collections::BTreeSet::new();
    for (i, a) in arrays.iter().enumerate() {
        let at = ArrayFault::Array(i);
        if !ids.insert(a.id) {
            return Some((at, "dizinin kimliği iki kez verilmiş"));
        }
        if blocks.is_some_and(|b| !b.contains(&a.block)) {
            return Some((at, "dizinin bloğu çizimde tanımlı değil"));
        }
        if a.layer_id.is_empty() {
            return Some((at, "dizinin katmanı boş"));
        }
        if !(a.p.x.is_finite() && a.p.y.is_finite() && a.rotation.is_finite()) {
            return Some((at, "dizinin yeri ya da dönüşü sonlu değil"));
        }
        if !(a.scale.is_finite() && a.scale > 0.0) {
            return Some((at, "dizinin ölçeği sıfırdan büyük sonlu bir sayı olmalı"));
        }
        let finite = match &a.layout {
            ArrayLayout::Grid { dx, dy, .. } | ArrayLayout::Line { dx, dy, .. } => {
                dx.is_finite() && dy.is_finite()
            }
            ArrayLayout::Polar { center, fill, .. } => {
                center.x.is_finite() && center.y.is_finite() && fill.is_finite()
            }
            ArrayLayout::Path { spacing, .. } => spacing.is_none_or(f64::is_finite),
        };
        if !finite {
            return Some((at, "dizinin yerleşiminde sonlu olmayan sayı var"));
        }
        if let ArrayLayout::Path { path, spacing, .. } = &a.layout {
            if EntityId::parse(path).is_none() {
                return Some((at, "yol dizisinin yolu bir nesne kimliği olmalı"));
            }
            if spacing.is_some_and(|s| s <= 0.0) {
                return Some((at, "yol dizisinin aralığı sıfırdan büyük olmalı"));
            }
        }
        let places = array_places(&a.layout);
        if !(2..=u64::from(ARRAY_PLACES_MAX)).contains(&places) {
            return Some((at, "dizinin yeri 2 ile 10 000 arasında olmalı"));
        }
        if a.items.len() as u64 != places {
            return Some((at, "dizinin her yerinin bir öğesi olmalı"));
        }
        for (k, item) in a.items.iter().enumerate() {
            if let Some(uid) = item.uid
                && !uids.insert(uid)
            {
                return Some((ArrayFault::Item(i, k), "bir yerleştirme iki öğede"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn array() -> AssocArray {
        serde_json::from_value(json!({
            "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001", "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001",
            "layerId": "cizim", "p": { "x": 0, "y": 0 }, "scale": 1, "rotation": 0,
            "layout": { "kind": "grid", "rows": 1, "cols": 3, "dx": 2, "dy": 0 },
            "items": [{ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6001" }, {}, { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6003", "kept": true }]
        }))
        .expect("reads")
    }

    #[test]
    fn an_array_reads_with_a_removed_and_a_kept_item() {
        let a = array();
        assert_eq!(a.items[1], ArrayItem::default());
        assert!(a.items[2].kept);
        let blocks = [a.block];
        assert_eq!(arrays_problem(std::slice::from_ref(&a), &blocks), None);
    }

    #[test]
    fn the_rules_say_where() {
        let mut a = array();
        let blocks = [a.block];
        a.items.pop();
        assert_eq!(
            arrays_problem(std::slice::from_ref(&a), &blocks),
            Some((ArrayFault::Array(0), "dizinin her yerinin bir öğesi olmalı"))
        );
        let mut b = array();
        b.items[1].uid = b.items[0].uid;
        assert_eq!(
            arrays_problem(&[b], &blocks),
            Some((ArrayFault::Item(0, 1), "bir yerleştirme iki öğede"))
        );
        assert_eq!(
            arrays_problem(&[array()], &[]),
            Some((ArrayFault::Array(0), "dizinin bloğu çizimde tanımlı değil"))
        );
    }
}
