//! Changes from outside this drawing: another editor's commits, the server's
//! copy chosen in a conflict (docs/adr/0040), as the web's `applyExternal`
//! takes them (apps/web/src/model/document.ts).
//!
//! They are not this user's edits: nothing goes into the undo history, the
//! drawing does not become unsaved and its revision stays. The undo and redo
//! steps that touch the objects they change are dropped, as the web drops
//! them (`forgetHistoryOf`), so an undo never puts back a state someone else
//! replaced; so are the steps that add or remove a layer the change puts
//! objects on. Readers that follow the document (`changes_since`) see them
//! like any change.
//!
//! Objects are named by their persistent ids: one already in the drawing
//! keeps its slot, a new one gets the next slot. A change that names an id
//! twice, or the nil id, is refused before anything happens, and so is any
//! change while a transaction or a group is open.
//!
//! Block definitions (docs/adr/0144 §5) come as the whole list the drawing
//! takes, in its order; a definition that did not change keeps its `Arc`,
//! so readers that compare them by pointer see only the changed ones. A
//! list that breaks a block rule is refused before anything happens. The
//! steps that change a definition that differs now, or that place or hold
//! one that is gone, are dropped.
//!
//! Associative arrays (docs/adr/0300 §5) come as the whole list too; the
//! drawing keeps those its blocks and objects can hold (an array whose block
//! is gone dropped, an item whose insert is gone removed, an array that
//! still breaks a rule left out), and the steps that change an array go.

use std::collections::HashSet;
use std::sync::Arc;

use kentos_contracts::{
    AssocArray, BlockDefinition, Entity, EntityId, LayerNode, ProjectSettings, ProjectSheets,
    ProjectStyles,
};

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::layers::LayerTree;
use crate::store::Stored;

/// The project's metadata as another editor left it; absent parts stay.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExternalMeta {
    pub name: Option<String>,
    pub settings: Option<ProjectSettings>,
    /// A new layer tree; this user's active layer stays when it is still a layer in it.
    pub layers: Option<Vec<LayerNode>>,
    pub styles: Option<ProjectStyles>,
    /// The sheet book and its pictures (docs/adr/0251); `Some(None)`: the project has none any more.
    pub sheets: Option<Option<ProjectSheets>>,
    /// The associative arrays (docs/adr/0300 §5), whole; none: they stay.
    pub arrays: Option<Vec<AssocArray>>,
    /// Architectural automatic dimension recipes, whole; missing hosts retain their recipe and warning.
    pub dimension_sets: Option<Vec<kentos_contracts::DimensionSet>>,
    /// The constraints and parameters (docs/adr/0336 §3), whole; none: they stay.
    pub constraints: Option<kentos_contracts::ConstraintSet>,
}

/// What came from outside.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct External {
    /// Objects as they are now, each under its persistent id (`id` is ignored).
    pub put: Vec<(Uuid, Entity)>,
    /// Objects removed; ids the drawing does not have are skipped.
    pub remove: Vec<Uuid>,
    pub meta: Option<ExternalMeta>,
    /// The block definitions the drawing takes, whole and in order; none: they stay.
    pub blocks: Option<Vec<BlockDefinition>>,
}

impl External {
    pub fn is_empty(&self) -> bool {
        self.put.is_empty()
            && self.remove.is_empty()
            && self.meta.is_none()
            && self.blocks.is_none()
    }
}

impl Document {
    /// Takes a change from outside (see the module comment). Refused, with the
    /// reason in Turkish and nothing changed, while an edit is open or when an
    /// id is nil or named twice.
    pub fn apply_external(&mut self, change: External) -> Result<(), String> {
        if self.is_busy() {
            return Err(
                "Açık bir düzenleme varken dışarıdan gelen değişiklik uygulanamaz.".to_owned(),
            );
        }
        let mut named = HashSet::new();
        for uid in change.put.iter().map(|(uid, _)| uid).chain(&change.remove) {
            if uid.is_nil() {
                return Err("Dışarıdan gelen değişiklikte boş (nil) bir kimlik var; değişiklik uygulanmadı.".to_owned());
            }
            if !named.insert(*uid) {
                return Err(format!(
                    "Kalıcı kimlik {uid} değişiklikte iki kez var; değişiklik uygulanmadı."
                ));
            }
        }
        // The definitions checked whole first: an unchanged one keeps its `Arc`.
        let blocks = match &change.blocks {
            Some(list) => {
                kentos_contracts::blocks::check(list, &[]).map_err(|fault| {
                    format!(
                        "Gelen blok tanımları kurala uymuyor ({}); değişiklik uygulanmadı.",
                        fault.message(|i| list.get(i).map_or("", |b| b.name.as_str()))
                    )
                })?;
                let mut changed = HashSet::new();
                let next: Vec<Arc<BlockDefinition>> = list
                    .iter()
                    .map(|b| match self.blocks.iter().find(|w| w.id == b.id) {
                        Some(was) if **was == *b => was.clone(),
                        _ => {
                            changed.insert(b.id);
                            Arc::new(b.clone())
                        }
                    })
                    .collect();
                let gone: HashSet<_> = self
                    .blocks
                    .iter()
                    .map(|b| b.id)
                    .filter(|id| !list.iter().any(|b| b.id == *id))
                    .collect();
                changed.extend(gone.iter().copied());
                Some((next, changed, gone))
            }
            None => None,
        };
        let added = change
            .put
            .iter()
            .filter(|(uid, _)| self.store.slot_of(*uid).is_none())
            .count() as u64;
        if self.next_slot + added > u64::from(u32::MAX) + 1 {
            return Err(
                "Çizimdeki nesne kimlikleri tükendi; dışarıdan gelen değişiklik uygulanmadı."
                    .to_owned(),
            );
        }
        let mut ops = Vec::with_capacity(change.put.len() + change.remove.len());
        let mut next_slot = self.next_slot;
        let mut slots = HashSet::new();
        // The layers the change puts objects on (the web's `onLayers`).
        let on_layers: HashSet<String> = change
            .put
            .iter()
            .map(|(_, entity)| entity.base().layer_id.clone())
            .collect();
        for (uid, mut entity) in change.put {
            let slot = match self.store.slot_of(uid) {
                Some(slot) => slot,
                None => {
                    // Checked above: every new object has a slot left.
                    let slot = Slot(u32::try_from(next_slot).unwrap_or(u32::MAX));
                    next_slot += 1;
                    slot
                }
            };
            entity.base_mut().id = slot.0;
            let after = Stored {
                uid,
                entity: Arc::new(entity),
            };
            slots.insert(slot);
            match self.store.get(slot) {
                Some(before) if before.entity == after.entity => {}
                Some(before) => ops.push(Op::Update {
                    before: before.clone(),
                    after,
                }),
                None => ops.push(Op::Add(after)),
            }
        }
        for uid in change.remove {
            if let Some(stored) = self
                .store
                .slot_of(uid)
                .and_then(|slot| self.store.get(slot))
            {
                slots.insert(stored.slot());
                ops.push(Op::Remove(stored.clone()));
            }
        }
        // Validate source recipes against the complete prospective objects and layer tree. In a cloud
        // commit a new recipe and its new members arrive together; validate before writing either.
        if !self.dimension_sets.is_empty()
            || change
                .meta
                .as_ref()
                .is_some_and(|m| m.dimension_sets.is_some())
        {
            let mut probe = self.reading_copy();
            for op in &ops {
                probe.apply_op(op);
            }
            if let Some(meta) = &change.meta {
                if let Some(layers) = &meta.layers {
                    probe.layers = LayerTree::new(layers.clone(), self.layers.active());
                }
                if let Some(sets) = &meta.dimension_sets {
                    probe.dimension_sets = sets.iter().cloned().map(Arc::new).collect();
                }
            }
            let sets: Vec<_> = probe.dimension_sets.iter().map(|s| &**s).collect();
            if let Some(why) = probe.dimension_sets_refused(&sets) {
                return Err(why);
            }
        }
        let blocks_changed = blocks.as_ref().is_some_and(|(_, c, _)| !c.is_empty());
        let changed = !ops.is_empty() || change.meta.is_some() || blocks_changed;
        // The definitions before the objects: an insert that comes is placed with them.
        let mut forget_blocks = None;
        if let Some((next, changed, gone)) = blocks
            && !changed.is_empty()
        {
            self.blocks = next;
            forget_blocks = Some((changed, gone));
        }
        self.next_slot = next_slot;
        for op in &ops {
            self.apply_op(op);
        }
        if let Some(meta) = change.meta {
            if let Some(layers) = meta.layers {
                let active = self.layers.active().to_owned();
                self.layers = LayerTree::new(layers, &active);
                // A removed layer's recorded place may no longer fit this tree.
                self.forget_tree_history();
            }
            if let Some(settings) = meta.settings {
                // The former Hibrit mode is a type not asked yet (docs/adr/0165 §1); the
                // systems and choices as a project keeps them (docs/adr/0167 §1, 0168).
                self.settings = ProjectSettings {
                    workspace: settings.project_type(),
                    ..settings
                }
                .sanitized();
            }
            if let Some(name) = meta.name {
                self.name = name;
            }
            if let Some(styles) = meta.styles {
                self.styles = styles;
            }
            if let Some(sheets) = meta.sheets {
                self.sheets = sheets.map(Arc::new);
            }
            if let Some(set) = meta.constraints {
                // As this drawing's objects hold them; outside the steps, as another editor's arrays are.
                let uids: std::collections::HashSet<EntityId> = self
                    .store
                    .iter()
                    .map(|s| EntityId(s.uid.into_bytes()))
                    .collect();
                self.constraints =
                    std::sync::Arc::new(kentos_contracts::constraints_kept(&set, &uids));
            }
            if let Some(arrays) = meta.arrays {
                self.take_arrays(&arrays);
            }
            if let Some(sets) = meta.dimension_sets {
                self.take_dimension_sets(&sets);
            }
        }
        self.forget_history_of(&slots, &named);
        self.normalize_dimension_set_sources();
        if let Some((changed, gone)) = forget_blocks {
            self.forget_block_history(&changed, &gone);
        }
        // Another editor's object on a layer a step added (or took away):
        // undoing that step would take the layer from under it, so the step goes.
        self.forget_layer_history(&on_layers);
        if changed {
            // These words also depend on settings or sources delivered by another editor.
            self.refresh_level_marks();
            self.refresh_derived_tags();
            // Not an edit (the revision stays), but what the drawing shows changed.
            self.generation += 1;
        }
        Ok(())
    }

    /// Takes another editor's arrays as this drawing's blocks and objects can hold them; when they differ, the
    /// steps that change an array are dropped.
    fn take_arrays(&mut self, arrays: &[AssocArray]) {
        let blocks: Vec<_> = self.blocks.iter().map(|b| b.id).collect();
        let uids: Vec<EntityId> = self
            .store
            .iter()
            .map(|s| EntityId(s.uid.into_bytes()))
            .collect();
        let mut kept = kentos_contracts::arrays_kept(arrays, &blocks, &uids);
        while let Some((fault, _)) = kentos_contracts::arrays_problem(&kept, &blocks) {
            let (kentos_contracts::ArrayFault::Array(i) | kentos_contracts::ArrayFault::Item(i, _)) =
                fault;
            kept.remove(i);
        }
        let now: Vec<&AssocArray> = self.arrays.iter().map(|a| &**a).collect();
        if now.len() == kept.len() && now.iter().zip(&kept).all(|(a, b)| *a == b) {
            return;
        }
        let next = kept
            .into_iter()
            .map(|a| match self.arrays.iter().find(|w| ***w == a) {
                Some(was) => was.clone(),
                None => Arc::new(a),
            })
            .collect();
        self.arrays = next;
        self.forget_array_history();
    }

    fn take_dimension_sets(&mut self, sets: &[kentos_contracts::DimensionSet]) {
        if self.dimension_sets.len() == sets.len()
            && self.dimension_sets.iter().zip(sets).all(|(a, b)| **a == *b)
        {
            return;
        }
        self.dimension_sets = sets
            .iter()
            .map(|set| {
                self.dimension_sets
                    .iter()
                    .find(|old| ***old == *set)
                    .cloned()
                    .unwrap_or_else(|| Arc::new(set.clone()))
            })
            .collect();
        self.forget_dimension_set_history();
    }
}
