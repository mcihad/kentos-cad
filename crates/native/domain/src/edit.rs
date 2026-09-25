//! Changes to the document, as the web's `CadDocument` and `LayerStore` make
//! them (apps/web/src/model/document.ts, layers.ts).
//!
//! Object edits and layer styles are undoable: each call is one undo step, or
//! joins the open transaction. The document does not check layer locks: like
//! the web's model it accepts any edit, and the tools and commands that make
//! edits ask `LayerTree::is_locked` and refuse (CLAUDE.md §7).
//!
//! Layer visibility, lock and name are project data too: they make the drawing
//! unsaved at once but are not undoable and stay when a transaction around
//! them fails. Folding a group and choosing the active layer are not edits at
//! all, although a saved file keeps them (web).

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use kentos_contracts::{Entity, EntityBase, LayerStyle};

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, new_uid};
use crate::store::Stored;

/// Undo step names the web gives its edits; undo and redo return them.
pub mod labels {
    pub const ADD: &str = "Ekle";
    pub const REMOVE: &str = "Sil";
    pub const CHANGE: &str = "Değiştir";
    pub const LAYER_STYLE: &str = "Katman stili";
}

/// Every slot (`u32`) has been given out in this document; nothing was added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotsExhausted;

impl fmt::Display for SlotsExhausted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "Çizimdeki nesne kimlikleri tükendi (en büyük kimlik 4294967295); yeni nesne eklenemedi.",
        )
    }
}

impl std::error::Error for SlotsExhausted {}

impl Document {
    /// Adds an object (its `id` is replaced by the slot it gets) as one undo step.
    pub fn add(&mut self, entity: Entity) -> Result<Slot, SlotsExhausted> {
        let mut slots = self.add_many(vec![entity], labels::ADD)?;
        slots.pop().ok_or(SlotsExhausted)
    }

    /// Adds many objects as one change and one undo step (an import, a paste);
    /// inside a transaction they join it. Nothing is added unless every object gets a slot.
    pub fn add_many(
        &mut self,
        entities: Vec<Entity>,
        label: &str,
    ) -> Result<Vec<Slot>, SlotsExhausted> {
        let last = self.next_slot + entities.len() as u64;
        if last > u64::from(u32::MAX) + 1 {
            return Err(SlotsExhausted);
        }
        let mut slots = Vec::with_capacity(entities.len());
        let mut ops = Vec::with_capacity(entities.len());
        for mut entity in entities {
            let slot = Slot(u32::try_from(self.next_slot).map_err(|_| SlotsExhausted)?);
            self.next_slot += 1;
            base_mut(&mut entity).id = slot.0;
            slots.push(slot);
            ops.push(Op::Add(Stored {
                uid: new_uid(),
                entity: Arc::new(entity),
            }));
        }
        self.record(ops, label);
        Ok(slots)
    }

    /// Replaces an object, keeping its slot and persistent id, as one undo
    /// step. Returns false for an unknown slot, or when nothing would change,
    /// which is not an edit (docs/adr/0020); nothing happens then.
    pub fn update(&mut self, slot: Slot, entity: Entity) -> bool {
        self.update_many(vec![(slot, entity)], labels::CHANGE) == 1
    }

    /// Replaces many objects as one change and one undo step (move, stretch,
    /// a layer for the selection); inside a transaction they join it. A slot
    /// given twice changes what its first entry made, as a second `update`
    /// would. Returns how many entries changed an object; unknown slots and
    /// entries that change nothing are skipped (docs/adr/0020).
    pub fn update_many(&mut self, changes: Vec<(Slot, Entity)>, label: &str) -> usize {
        let mut latest: HashMap<Slot, Stored> = HashMap::new();
        let mut ops = Vec::with_capacity(changes.len());
        for (slot, entity) in changes {
            let Some(before) = latest.get(&slot).or_else(|| self.store.get(slot)).cloned() else {
                continue;
            };
            let after = Stored {
                uid: before.uid,
                entity: Arc::new(changed(entity, slot)),
            };
            if after.entity == before.entity {
                continue;
            }
            latest.insert(slot, after.clone());
            ops.push(Op::Update { before, after });
        }
        let applied = ops.len();
        self.record(ops, label);
        applied
    }

    /// Removes objects as one change and one undo step; unknown and repeated
    /// slots are skipped. Returns how many were removed.
    pub fn remove(&mut self, slots: &[Slot]) -> usize {
        let mut seen = HashSet::new();
        let ops: Vec<Op> = slots
            .iter()
            .filter_map(|slot| self.store.get(*slot))
            .filter(|stored| seen.insert(stored.slot()))
            .cloned()
            .map(Op::Remove)
            .collect();
        let removed = ops.len();
        self.record(ops, labels::REMOVE);
        removed
    }

    /// Gives a layer (or group) a new look as one undo step named `label`. An
    /// unknown id or an unchanged style records nothing; returns whether it changed.
    pub fn set_layer_style(&mut self, id: &str, style: LayerStyle, label: &str) -> bool {
        let Some(node) = self.layers.get(id) else {
            return false;
        };
        if node.style == style {
            return false;
        }
        let op = Op::LayerStyle {
            layer: id.to_owned(),
            before: Box::new(node.style.clone()),
            after: Box::new(style),
        };
        self.record(vec![op], label);
        true
    }

    // ── The layer tree's own changes (not undoable) ─────────────────────────

    /// Shows or hides a layer or group; an edit when it changes.
    pub fn set_layer_visible(&mut self, id: &str, visible: bool) {
        if self.layers.set_visible(id, visible) {
            self.mark_edited();
        }
    }

    pub fn toggle_layer_visible(&mut self, id: &str) {
        if let Some(visible) = self.layers.get(id).map(|node| !node.visible) {
            self.set_layer_visible(id, visible);
        }
    }

    pub fn toggle_layer_locked(&mut self, id: &str) {
        if self.layers.toggle_locked(id) {
            self.mark_edited();
        }
    }

    /// Shows only this node, the groups above it and everything below it.
    pub fn isolate_layer(&mut self, id: &str) {
        if self.layers.isolate(id) {
            self.mark_edited();
        }
    }

    /// Shows every layer and group; an edit only when one was hidden (docs/adr/0020).
    pub fn show_all_layers(&mut self) {
        if self.layers.show_all() {
            self.mark_edited();
        }
    }

    /// Opens or closes a group in the tree: kept in the file, not an edit (web).
    pub fn set_layer_expanded(&mut self, id: &str, expanded: bool) {
        self.layers.set_expanded(id, expanded);
    }

    /// Makes a layer the one new objects go to: kept in the file, not an edit
    /// (web). Groups and unknown ids are refused; returns whether it was taken.
    pub fn set_active_layer(&mut self, id: &str) -> bool {
        self.layers.set_active(id)
    }

    /// Renames a layer or group: trimmed, an empty name refused. The same name
    /// again is not an edit (docs/adr/0020).
    pub fn rename_layer(&mut self, id: &str, name: &str) {
        if self.layers.rename(id, name) {
            self.mark_edited();
        }
    }
}

/// `entity` as the object at `slot` after an update: the slot's id, and no
/// holes on a polyline (a polygon opened by an edit loses them, web `updateOp`).
/// A hatch keeps its islands, on the web too since docs/adr/0020's fix.
fn changed(mut entity: Entity, slot: Slot) -> Entity {
    base_mut(&mut entity).id = slot.0;
    if let Entity::Polyline(path) = &mut entity {
        path.holes = None;
    }
    entity
}

pub(crate) fn base_mut(entity: &mut Entity) -> &mut EntityBase {
    match entity {
        Entity::Point(e) => &mut e.base,
        Entity::Line(e) => &mut e.base,
        Entity::Polyline(e) | Entity::Polygon(e) => &mut e.base,
        Entity::Circle(e) => &mut e.base,
        Entity::Arc(e) => &mut e.base,
        Entity::Ellipse(e) => &mut e.base,
        Entity::Spline(e) => &mut e.base,
        Entity::Xline(e) | Entity::Ray(e) => &mut e.base,
        Entity::Text(e) => &mut e.base,
        Entity::Dimension(e) => &mut e.base,
        Entity::Hatch(e) => &mut e.base,
    }
}
