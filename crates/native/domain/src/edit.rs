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

use kentos_contracts::{
    Entity, EntityBase, LayerNodeType, LayerSnap, LayerStyle, ProjectSettings, ProjectStyles,
};

use crate::document::Document;
use crate::history::{LayerPlace, Op};
use crate::identity::{Slot, new_uid};
use crate::layers::NewLayer;
use crate::store::Stored;

/// Undo step names the web gives its edits; undo and redo return them.
pub mod labels {
    pub const ADD: &str = "Ekle";
    pub const REMOVE: &str = "Sil";
    pub const CHANGE: &str = "Değiştir";
    pub const LAYER_STYLE: &str = "Katman stili";
    pub const LAYER_REMOVE: &str = "Katman sil";
    pub const LAYER_ADD: &str = "Katman ekle";
    pub const GROUP_ADD: &str = "Grup ekle";
    pub const BLOCK_ADD: &str = "Blok tanımla";
    pub const BLOCK_CHANGE: &str = "Blok değiştir";
    pub const BLOCK_REMOVE: &str = "Blok sil";
    pub const BLOCK_PURGE: &str = "Blokları temizle";
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

/// An edit the document refuses, with the reason for people; nothing was
/// changed (the web's `Refusal`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal(pub String);

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refusal {}

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
        self.store.reserve(ops.len());
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

    /// Deletes a layer, or a group with everything under it, and the objects
    /// on them, as one undo step “Katman sil” (into the open transaction or
    /// group, if one is). Undo puts the node back in its place with its
    /// children, flags and style, then the objects in their slots with their
    /// persistent ids. Refused, with nothing changed, as
    /// [`Document::layer_removal_refused`] says. Returns how many objects
    /// went; an unknown id changes nothing and returns 0 (web `removeLayer`).
    pub fn remove_layer(&mut self, id: &str) -> Result<usize, Refusal> {
        let (Some(node), Some((parent, index))) =
            (self.layers.get(id).cloned(), self.layers.place_of(id))
        else {
            return Ok(0);
        };
        if let Some(reason) = self.layer_removal_refused(id) {
            return Err(Refusal(reason));
        }
        let kept: HashSet<&str> = self
            .layers
            .leaves_of(id)
            .into_iter()
            .map(|layer| layer.id.as_str())
            .collect();
        let gone: Vec<Slot> = self
            .entities()
            .filter(|e| kept.contains(e.base().layer_id.as_str()))
            .map(|e| Slot(e.base().id))
            .collect();
        let place = LayerPlace {
            node,
            parent,
            index,
        };
        let _ = self.transact(labels::LAYER_REMOVE, |doc| {
            doc.remove(&gone);
            doc.record(vec![Op::LayerRemove(Box::new(place))], labels::LAYER_REMOVE);
            Ok::<(), Refusal>(())
        });
        Ok(gone.len())
    }

    /// Why [`Document::remove_layer`] would refuse, in its words, or none
    /// when it would not: the interface asks this before its own question
    /// (the web's `layerRemovalRefused`). In this order: the last layer (or a
    /// group holding every layer), the active layer (or a group holding it),
    /// a locked node (by itself or a group above it), a group with a locked
    /// layer under it.
    pub fn layer_removal_refused(&self, id: &str) -> Option<String> {
        let layers = &self.layers;
        let node = layers.get(id)?;
        let name = &node.name;
        let group = node.kind == LayerNodeType::Group;
        let leaves = layers.leaves_of(id);
        let kept: HashSet<&str> = leaves.iter().map(|l| l.id.as_str()).collect();
        if layers.leaves().iter().all(|l| kept.contains(l.id.as_str())) {
            return Some(if group {
                format!(
                    "“{name}” grubu çizimin bütün katmanlarını içeriyor; silinemez. Çizimde en az bir katman olmalı."
                )
            } else {
                format!("“{name}” çizimin son katmanı; silinemez. Çizimde en az bir katman olmalı.")
            });
        }
        let active = layers.active();
        if id == active {
            return Some(format!(
                "“{name}” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin."
            ));
        }
        if kept.contains(active) {
            let held = layers.get(active).map_or(active, |l| l.name.as_str());
            return Some(format!(
                "“{name}” grubu etkin katmanı (“{held}”) içeriyor; silinemez. Önce grubun dışındaki bir katmanı etkinleştirin."
            ));
        }
        if layers.is_locked(id) {
            let what = if group { "grubu" } else { "katmanı" };
            return Some(format!(
                "“{name}” {what} kilitli; silinemez. Kilidi Katmanlar panelinden açın."
            ));
        }
        leaves
            .iter()
            .find(|l| layers.is_locked(&l.id))
            .map(|locked| {
                format!(
                    "“{name}” grubu kilitli bir katman (“{}”) içeriyor; silinemez. Kilidi Katmanlar panelinden açın.",
                    locked.name
                )
            })
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

    /// A layer's own snapping, on a group's every layer (docs/adr/0163 §4):
    /// an edit, not an undo step, as a lock is.
    pub fn set_layer_snap(&mut self, id: &str, snap: Option<LayerSnap>) {
        if self.layers.set_snap(id, snap) {
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

    /// Adds a layer or a group as one undo step, “Katman ekle” or “Grup
    /// ekle” (into the open transaction or group, if one is, under its name;
    /// the web's `CadDocument.addLayer`). It goes last into `parent` when
    /// that is a group, last into the group of `parent` when that is a
    /// layer, last at the top otherwise (an unknown `parent` too); the group
    /// it enters opens, and undo leaves it open (a view state). With
    /// `activate` a new layer becomes the active one in the same step: undo
    /// makes the one before active again, unless another was made active by
    /// hand meanwhile; an active layer that goes with an undone node gives
    /// way to the first layer of the tree. Refused, with nothing changed,
    /// when the tree already has the id. Returns the new node's id.
    pub fn add_layer(
        &mut self,
        new: NewLayer,
        parent: Option<&str>,
        activate: bool,
    ) -> Result<String, Refusal> {
        if let Some(id) = new.id.as_deref().filter(|id| self.layers.get(id).is_some()) {
            return Err(Refusal(format!(
                "“{id}” kimlikli katman zaten var; katman eklenmedi."
            )));
        }
        let node = self.layers.make(new);
        let id = node.id.clone();
        let container = self.layers.container_for(parent);
        let index = self.layers.len_of(container.as_deref());
        let label = match node.kind {
            LayerNodeType::Group => labels::GROUP_ADD,
            LayerNodeType::Layer => labels::LAYER_ADD,
        };
        let before = self.layers.active().to_owned();
        let active = (activate && node.kind == LayerNodeType::Layer && before != id).then(|| {
            Op::LayerActive {
                before,
                after: id.clone(),
            }
        });
        if let Some(group) = &container {
            self.layers.set_expanded(group, true);
        }
        let place = LayerPlace {
            node,
            parent: container,
            index,
        };
        let _ = self.transact(label, |doc| {
            doc.record(vec![Op::LayerAdd(Box::new(place))], label);
            if let Some(op) = active {
                doc.record(vec![op], label);
            }
            Ok::<(), Refusal>(())
        });
        Ok(id)
    }

    /// Whether any object is on a layer of this node or under it now.
    pub(crate) fn holds_objects(&self, id: &str) -> bool {
        let kept: HashSet<&str> = self
            .layers
            .leaves_of(id)
            .into_iter()
            .map(|layer| layer.id.as_str())
            .collect();
        !kept.is_empty()
            && self
                .entities()
                .any(|e| kept.contains(e.base().layer_id.as_str()))
    }

    /// Names the drawing (web `doc.name.set`): an edit when the name
    /// differs, never an undo step. The window that asks trims it.
    pub fn set_name(&mut self, name: &str) {
        if self.name != name {
            name.clone_into(&mut self.name);
            self.mark_edited();
        }
    }

    /// Replaces the project settings (web `settings.assign`): an edit when
    /// any differs, never an undo step. A new coordinate system is assigned,
    /// never a transformation of the coordinates (CLAUDE.md §5). A second
    /// system that is not another system than the project's own, or is a
    /// local project's, goes (docs/adr/0167 §1); so do definitions and datum
    /// choices the project may not keep (`ProjectSettings::sanitized`,
    /// docs/adr/0168).
    pub fn set_settings(&mut self, settings: ProjectSettings) {
        let settings = settings.sanitized();
        if self.settings != settings {
            self.settings = settings;
            self.mark_edited();
        }
    }

    /// Replaces the project's style library, its symbols, drawings and
    /// categories (web `doc.styles.set`, the style manager's Proje): an edit
    /// when it differs, never an undo step (the library keeps no undo).
    pub fn set_styles(&mut self, styles: ProjectStyles) {
        if self.styles != styles {
            self.styles = styles;
            self.mark_edited();
        }
    }
}

/// `entity` as the object at `slot` after an update: the slot's id, and no
/// holes or parts on a polyline (a polygon opened by an edit loses them, web
/// `updateOp`; docs/adr/0143). A hatch keeps its islands, on the web too
/// since docs/adr/0020's fix.
fn changed(mut entity: Entity, slot: Slot) -> Entity {
    base_mut(&mut entity).id = slot.0;
    if let Entity::Polyline(path) = &mut entity {
        path.holes = None;
        path.parts = None;
    }
    entity
}

pub(crate) fn base_mut(entity: &mut Entity) -> &mut EntityBase {
    entity.base_mut()
}
