//! A copy of the drawing's objects for the hot paths (docs/adr/0008, S1):
//! picking, object snap and window selection ask it on every pointer move
//! instead of walking every object in TypeScript (`apps/web/src/viewport/picking.ts`
//! is its thin face). It follows the document by object: puts (new objects
//! go last, known ones keep their place) and removals, and whole reloads.
//! It keeps the document's order because results that tie (two parcels
//! sharing an edge) are decided by it, as the TypeScript's `Map` order did.
//!
//! Candidates come from a packed R-tree over the objects' boxes; objects
//! changed since the last build, and those whose box the tree cannot hold
//! (infinite lines, empty or non-finite boxes), are scanned directly. Each
//! query then applies the exact test the TypeScript applied, in the
//! document's order, so the tree only makes it faster.

pub mod draw;
pub mod labels;
mod pack;
pub mod pick;
pub mod processing;
mod rtree;
pub mod snap;
pub mod tools;

pub use pack::Packer;

use std::collections::HashMap;

use crate::api::json::{FromJson, Json};
use crate::entity::{Entity, Shape, entity_bounds_in};
use crate::text::Font;
use crate::geometry::{Bounds, empty_bounds, is_empty_bounds};
use crate::jsmath::{js_max, js_min};
use rtree::{PackedTree, overlaps};

/// What queries need from an object's layer (`LayerStore.isVisible`,
/// `isLocked`, `style.pickInterior`, `style.label`), ancestors included.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerFlags {
    pub visible: bool,
    pub locked: bool,
    pub pick_interior: bool,
    /// The layer's label style; `None` takes the kind's default.
    pub label: Option<labels::LabelRule>,
}

/// A layer the table does not list behaves as in TypeScript: visible, unlocked, interior picking on.
const UNLISTED: LayerFlags = LayerFlags {
    visible: true,
    locked: false,
    pick_interior: true,
    label: None,
};

/// One object: its id, geometry, layer and cached box.
#[derive(Clone, Debug)]
pub struct Item {
    pub id: f64,
    pub shape: Shape,
    pub layer: u32,
    pub bounds: Bounds,
    /// Whether it carries a label (`e.label`, non-empty).
    pub label: bool,
    /// Its place in the document's order.
    order: u64,
}

impl Item {
    /// Infinite lines and boxes the tree cannot hold: always candidates, decided by the exact test.
    fn special(&self) -> bool {
        let b = &self.bounds;
        matches!(self.shape, Shape::Xline { .. } | Shape::Ray { .. })
            || !(b.min_x.is_finite()
                && b.min_y.is_finite()
                && b.max_x.is_finite()
                && b.max_y.is_finite())
            || b.min_x > b.max_x
            || b.min_y > b.max_y
    }
}

/// Rebuild the tree when this many objects changed since the last build (or a sixteenth of all, if more).
const REBUILD_AFTER: usize = 256;

#[derive(Default)]
pub struct Store {
    slots: Vec<Option<Item>>,
    free: Vec<u32>,
    by_id: HashMap<u64, u32>,
    next_order: u64,
    live: usize,
    layer_ids: HashMap<String, u32>,
    flags: Vec<LayerFlags>,
    /// Label rules by kind for layers without a label style (see `labels`).
    label_defaults: [Option<labels::LabelRule>; 5],
    tree: Option<PackedTree>,
    /// Per slot: the tree holds its current object.
    in_tree: Vec<bool>,
    /// Slots outside the tree (changed since the build, or special), scanned by every query.
    loose: Vec<u32>,
    in_loose: Vec<bool>,
    /// `(order, slot)` in the document's order; entries whose slot has since
    /// been freed or reused are stale and skipped (compacted at rebuilds).
    ordered: Vec<(u64, u32)>,
    /// The drawing typeface text boxes are measured in (`ProjectSettings.drawingFont`).
    font: Font,
}

/// An object read from the document's JSON: its id, layer, label and geometry.
fn read_item(v: &Json) -> Result<(f64, String, bool, Shape), String> {
    let e = Entity::from_json(v)?;
    let mut id = None;
    let mut layer = String::new();
    let mut label = false;
    for (k, val) in &e.rest {
        match (k.as_str(), val) {
            ("id", Json::Num(n)) => id = Some(*n),
            ("layerId", Json::Str(s)) => layer = s.clone(),
            ("label", Json::Str(s)) => label = !s.is_empty(),
            _ => {}
        }
    }
    let id = id.ok_or("nesnenin kimliği yok")?;
    Ok((id, layer, label, e.shape))
}

impl Store {
    pub fn new() -> Store {
        Store::default()
    }

    /// Number of objects.
    pub fn len(&self) -> usize {
        self.live
    }

    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Adds or replaces objects given as a JSON array of entities, in order:
    /// a new id goes last, a known one keeps its place (a JavaScript `Map`).
    pub fn put_json(&mut self, text: &str) -> Result<usize, String> {
        let Json::Arr(list) = Json::parse(text)? else {
            return Err("nesne dizisi bekleniyordu".into());
        };
        for (i, v) in list.iter().enumerate() {
            let (id, layer, label, shape) = read_item(v).map_err(|e| format!("[{i}]: {e}"))?;
            self.put(id, &layer, label, shape);
        }
        self.maybe_rebuild();
        Ok(list.len())
    }

    /// Adds or replaces one object.
    pub fn put(&mut self, id: f64, layer_id: &str, label: bool, shape: Shape) {
        let layer = self.layer_index(layer_id);
        let bounds = entity_bounds_in(&shape, self.font);
        let key = id.to_bits();
        let slot = match self.by_id.get(&key) {
            Some(&s) => {
                let order = self.slots[s as usize].as_ref().map_or(0, |it| it.order);
                self.slots[s as usize] = Some(Item {
                    id,
                    shape,
                    layer,
                    bounds,
                    label,
                    order,
                });
                s
            }
            None => {
                let item = Item {
                    id,
                    shape,
                    layer,
                    bounds,
                    label,
                    order: self.next_order,
                };
                self.next_order += 1;
                self.live += 1;
                let s = match self.free.pop() {
                    Some(s) => {
                        self.slots[s as usize] = Some(item);
                        s
                    }
                    None => {
                        self.slots.push(Some(item));
                        self.in_tree.push(false);
                        self.in_loose.push(false);
                        (self.slots.len() - 1) as u32
                    }
                };
                self.by_id.insert(key, s);
                self.ordered.push((self.next_order - 1, s));
                s
            }
        };
        self.loosen(slot);
    }

    /// Removes objects; unknown ids are ignored.
    pub fn remove(&mut self, ids: &[f64]) {
        for id in ids {
            if let Some(s) = self.by_id.remove(&id.to_bits()) {
                self.slots[s as usize] = None;
                self.in_tree[s as usize] = false;
                self.free.push(s);
                self.live -= 1;
            }
        }
        if self.ordered.len() > 2 * self.live + REBUILD_AFTER {
            self.compact_order();
        }
        self.maybe_rebuild();
    }

    /// The drawing typeface: text boxes (picking, window selection, extents) follow its letters. A
    /// change measures every text again.
    pub fn set_font(&mut self, font: Font) {
        if font == self.font {
            return;
        }
        self.font = font;
        let texts: Vec<u32> = (0..self.slots.len() as u32)
            .filter(|&s| matches!(&self.slots[s as usize], Some(it) if matches!(it.shape, Shape::Text { .. })))
            .collect();
        for s in &texts {
            if let Some(it) = self.slots[*s as usize].as_mut() {
                it.bounds = entity_bounds_in(&it.shape, font);
            }
            self.loosen(*s);
        }
        self.maybe_rebuild();
    }

    pub fn font(&self) -> Font {
        self.font
    }

    /// Empties the store (the layer table and label defaults stay).
    pub fn clear(&mut self) {
        let layers = (
            std::mem::take(&mut self.layer_ids),
            std::mem::take(&mut self.flags),
        );
        let defaults = self.label_defaults;
        let font = self.font;
        *self = Store::default();
        (self.layer_ids, self.flags) = layers;
        self.label_defaults = defaults;
        self.font = font;
    }

    /// Replaces the layer table: `[{ id, visible, locked, pickInterior, label? }]`,
    /// the flags already resolved with the ancestors (every node of the
    /// tree); `label` is the layer's label style, when it has one.
    pub fn set_layers_json(&mut self, text: &str) -> Result<(), String> {
        let Json::Arr(list) = Json::parse(text)? else {
            return Err("katman dizisi bekleniyordu".into());
        };
        for f in self.flags.iter_mut() {
            *f = UNLISTED;
        }
        for (i, v) in list.iter().enumerate() {
            let field = |k: &str| -> Result<bool, String> {
                bool::from_json(v.get(k)).map_err(|e| format!("[{i}].{k}: {e}"))
            };
            let Json::Str(id) = v.get("id") else {
                return Err(format!("[{i}].id: metin bekleniyordu"));
            };
            let label = match v.get("label") {
                Json::Null => None,
                r => Some(labels::read_rule(r).map_err(|e| format!("[{i}].label: {e}"))?),
            };
            let flags = LayerFlags {
                visible: field("visible")?,
                locked: field("locked")?,
                pick_interior: field("pickInterior")?,
                label,
            };
            let l = self.layer_index(id);
            self.flags[l as usize] = flags;
        }
        Ok(())
    }

    /// Ids in the document's order.
    pub fn ids(&self) -> Vec<f64> {
        self.all_items().iter().map(|it| it.id).collect()
    }

    /// The box around these objects, or around all of them (S3b): the
    /// union of their boxes as `CadDocument.bounds` takes it, None when
    /// there is nothing. Zoom to extents and the clipboard's base point ask
    /// here instead of sending every object through JSON.
    pub fn extent(&self, ids: Option<&[f64]>) -> Option<Bounds> {
        let mut b = empty_bounds();
        let mut add = |it: &Item| {
            b.min_x = js_min(b.min_x, it.bounds.min_x);
            b.min_y = js_min(b.min_y, it.bounds.min_y);
            b.max_x = js_max(b.max_x, it.bounds.max_x);
            b.max_y = js_max(b.max_y, it.bounds.max_y);
        };
        match ids {
            Some(ids) => ids.iter().filter_map(|&id| self.get(id)).for_each(&mut add),
            None => self.slots.iter().flatten().for_each(&mut add),
        }
        if is_empty_bounds(&b) { None } else { Some(b) }
    }

    /// An object by id.
    pub fn get(&self, id: f64) -> Option<&Item> {
        let s = *self.by_id.get(&id.to_bits())?;
        self.slots[s as usize].as_ref()
    }

    /// An object as the store holds it (tests and debugging): its id, layer,
    /// label flag and geometry, as JSON.
    pub fn item_json(&self, id: f64) -> Option<String> {
        let it = self.get(id)?;
        let layer = self
            .layer_ids
            .iter()
            .find(|(_, l)| **l == it.layer)
            .map_or("", |(k, _)| k.as_str());
        let mut out = String::from("{");
        let mut first = true;
        crate::api::json::field(&mut out, &mut first, "id", &it.id);
        crate::api::json::field(&mut out, &mut first, "layerId", layer);
        crate::api::json::field(&mut out, &mut first, "label", &it.label);
        it.shape.write_fields(&mut out, &mut first);
        out.push('}');
        Some(out)
    }

    /// Layer ids by the store's layer number.
    fn layer_names(&self) -> Vec<&str> {
        let mut names = vec![""; self.flags.len()];
        for (name, &l) in &self.layer_ids {
            if let Some(n) = names.get_mut(l as usize) {
                *n = name;
            }
        }
        names
    }

    /// The flags of an object's layer.
    pub fn flags(&self, item: &Item) -> LayerFlags {
        self.flags
            .get(item.layer as usize)
            .copied()
            .unwrap_or(UNLISTED)
    }

    fn layer_index(&mut self, id: &str) -> u32 {
        if let Some(&l) = self.layer_ids.get(id) {
            return l;
        }
        let l = self.flags.len() as u32;
        self.layer_ids.insert(id.to_string(), l);
        self.flags.push(UNLISTED);
        l
    }

    /// Marks a slot as outside the tree until the next build.
    fn loosen(&mut self, s: u32) {
        self.in_tree[s as usize] = false;
        if !self.in_loose[s as usize] {
            self.in_loose[s as usize] = true;
            self.loose.push(s);
        }
    }

    fn maybe_rebuild(&mut self) {
        if self.loose.len() > REBUILD_AFTER.max(self.live / 16) {
            self.rebuild();
        }
    }

    /// Puts every ordinary object into a new tree; special ones stay loose.
    fn rebuild(&mut self) {
        let mut entries = Vec::with_capacity(self.live);
        let mut loose = Vec::new();
        for (s, slot) in self.slots.iter().enumerate() {
            self.in_tree[s] = false;
            self.in_loose[s] = false;
            let Some(it) = slot else { continue };
            if it.special() {
                loose.push(s as u32);
                self.in_loose[s] = true;
            } else {
                entries.push((s as u32, it.bounds));
                self.in_tree[s] = true;
            }
        }
        self.tree = Some(PackedTree::build(&entries));
        self.loose = loose;
        self.compact_order();
    }

    /// Drops stale `ordered` entries.
    fn compact_order(&mut self) {
        let slots = &self.slots;
        self.ordered
            .retain(|&(o, s)| slots[s as usize].as_ref().is_some_and(|it| it.order == o));
    }

    /// The object an `ordered` entry stands for, unless it is stale.
    fn ordered_item(&self, (order, s): (u64, u32)) -> Option<&Item> {
        self.slots[s as usize]
            .as_ref()
            .filter(|it| it.order == order)
    }

    /// Every object, in the document's order.
    fn all_items(&self) -> Vec<&Item> {
        self.ordered
            .iter()
            .filter_map(|&e| self.ordered_item(e))
            .collect()
    }

    /// Objects whose box may overlap `q`, and every special one, in the
    /// document's order: a superset the caller filters with its exact test.
    /// A box with NaN gives every object (the TypeScript's tests all fail open then).
    fn candidates(&self, q: &Bounds) -> Vec<&Item> {
        if q.min_x.is_nan() || q.min_y.is_nan() || q.max_x.is_nan() || q.max_y.is_nan() {
            return self.all_items();
        }
        // A box around the whole tree (an overview) takes every tree item:
        // walk the document's order once, without searching or sorting.
        if let Some(root) = self.tree.as_ref().and_then(|t| t.bounds())
            && q.min_x <= root.min_x
            && q.min_y <= root.min_y
            && q.max_x >= root.max_x
            && q.max_y >= root.max_y
        {
            return self
                .ordered
                .iter()
                .filter_map(|&(o, s)| {
                    let it = self.ordered_item((o, s))?;
                    (self.in_tree[s as usize] || it.special() || overlaps(&it.bounds, q))
                        .then_some(it)
                })
                .collect();
        }
        let mut slots = Vec::new();
        if let Some(t) = &self.tree {
            t.search(q, &mut slots);
            slots.retain(|&s| self.in_tree[s as usize]);
        }
        for &s in &self.loose {
            if let Some(it) = &self.slots[s as usize]
                && !self.in_tree[s as usize]
                && (it.special() || overlaps(&it.bounds, q))
            {
                slots.push(s);
            }
        }
        // Few candidates are sorted; many (an overview) are picked out while
        // walking the document's order, which needs no sort.
        if slots.len() * 8 < self.live {
            let mut out: Vec<&Item> = slots
                .iter()
                .filter_map(|&s| self.slots[s as usize].as_ref())
                .collect();
            out.sort_unstable_by_key(|it| it.order);
            return out;
        }
        let mut chosen = vec![false; self.slots.len()];
        for &s in &slots {
            chosen[s as usize] = true;
        }
        self.ordered
            .iter()
            .filter(|&&(_, s)| chosen[s as usize])
            .filter_map(|&e| self.ordered_item(e))
            .collect()
    }
}

/// A box widened by `extra` plus a margin for rounding: the exact tests
/// subtract and add tolerances themselves, so the candidate box must not
/// miss what they would accept.
pub(crate) fn padded(b: Bounds, extra: f64) -> Bounds {
    let scale = 1.0 + b.min_x.abs() + b.min_y.abs() + b.max_x.abs() + b.max_y.abs() + extra.abs();
    let m = extra + 1e-9 * scale;
    Bounds {
        min_x: b.min_x - m,
        min_y: b.min_y - m,
        max_x: b.max_x + m,
        max_y: b.max_y + m,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: f64, layer: &str, x: f64) -> String {
        format!(
            r#"{{"id":{id},"layerId":"{layer}","attrs":{{"Ada":"1"}},"kind":"line","a":{{"x":{x},"y":0}},"b":{{"x":{x},"y":10}}}}"#
        )
    }

    #[test]
    fn the_extent_is_the_union_of_the_boxes() {
        let mut s = Store::new();
        assert_eq!(s.extent(None), None);
        s.put_json(&format!(
            "[{},{},{}]",
            line(1.0, "a", 3.0),
            line(2.0, "a", -4.0),
            r#"{"id":3,"layerId":"a","attrs":{},"kind":"circle","c":{"x":50,"y":5},"r":2}"#
        ))
        .unwrap();
        let all = s.extent(None).unwrap();
        assert_eq!(
            (all.min_x, all.min_y, all.max_x, all.max_y),
            (-4.0, 0.0, 52.0, 10.0)
        );
        let some = s.extent(Some(&[1.0, 2.0, 99.0])).unwrap();
        assert_eq!((some.min_x, some.max_x), (-4.0, 3.0));
        assert_eq!(s.extent(Some(&[99.0])), None);
    }

    #[test]
    fn keeps_the_documents_order_through_puts_and_removes() {
        let mut s = Store::new();
        let batch: Vec<String> = (1..=5)
            .map(|i| line(f64::from(i), "a", f64::from(i)))
            .collect();
        s.put_json(&format!("[{}]", batch.join(","))).unwrap();
        assert_eq!(s.ids(), [1.0, 2.0, 3.0, 4.0, 5.0]);
        // A known id keeps its place; a removed and re-added one goes last.
        s.put_json(&format!("[{}]", line(2.0, "a", 20.0))).unwrap();
        s.remove(&[3.0]);
        s.put_json(&format!(
            "[{},{}]",
            line(3.0, "a", 3.0),
            line(6.0, "a", 6.0)
        ))
        .unwrap();
        assert_eq!(s.ids(), [1.0, 2.0, 4.0, 5.0, 3.0, 6.0]);
        assert_eq!(s.get(2.0).map(|it| it.bounds.min_x), Some(20.0));
        assert_eq!(s.len(), 6);
        s.clear();
        assert!(s.is_empty() && s.ids().is_empty());
    }

    #[test]
    fn candidates_are_the_same_before_and_after_a_rebuild() {
        let mut s = Store::new();
        let batch: Vec<String> = (0..600)
            .map(|i| line(f64::from(i), "a", f64::from(i) * 3.0))
            .collect();
        s.put_json(&format!("[{}]", batch.join(","))).unwrap();
        assert!(s.tree.is_some(), "a big batch builds the tree");
        s.put_json(&format!("[{}]", line(7.0, "a", 1000.0)))
            .unwrap();
        let q = Bounds {
            min_x: 995.0,
            min_y: -1.0,
            max_x: 1005.0,
            max_y: 1.0,
        };
        let before: Vec<f64> = s.candidates(&q).iter().map(|it| it.id).collect();
        s.rebuild();
        let after: Vec<f64> = s.candidates(&q).iter().map(|it| it.id).collect();
        assert_eq!(before, after);
        assert!(before.contains(&7.0) && before.contains(&333.0) && !before.contains(&100.0));
        let nan = Bounds {
            min_x: f64::NAN,
            ..q
        };
        assert_eq!(s.candidates(&nan).len(), 600);
    }

    #[test]
    fn reads_layers_and_rejects_bad_input() {
        let mut s = Store::new();
        s.put_json(&format!("[{}]", line(1.0, "gizli", 0.0)))
            .unwrap();
        s.set_layers_json(r#"[{"id":"gizli","visible":false,"locked":true,"pickInterior":false}]"#)
            .unwrap();
        let it = s.get(1.0).unwrap();
        assert_eq!(
            s.flags(it),
            LayerFlags {
                visible: false,
                locked: true,
                pick_interior: false,
                label: None
            }
        );
        s.set_layers_json("[]").unwrap();
        assert_eq!(s.flags(s.get(1.0).unwrap()), UNLISTED);
        assert!(
            s.put_json(r#"[{"kind":"point","p":{"x":0,"y":0}}]"#)
                .is_err()
        );
        assert!(s.set_layers_json(r#"[{"id":"a"}]"#).is_err());
    }
}
