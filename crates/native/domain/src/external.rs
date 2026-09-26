//! Changes from outside this drawing: another editor's commits, the server's
//! copy chosen in a conflict (docs/adr/0040), as the web's `applyExternal`
//! takes them (apps/web/src/model/document.ts).
//!
//! They are not this user's edits: nothing goes into the undo history, the
//! drawing does not become unsaved and its revision stays. The undo and redo
//! steps that touch the objects they change are dropped, as the web drops
//! them (`forgetHistoryOf`), so an undo never puts back a state someone else
//! replaced. Readers that follow the document (`changes_since`) see them like
//! any change.
//!
//! Objects are named by their persistent ids: one already in the drawing
//! keeps its slot, a new one gets the next slot. A change that names an id
//! twice, or the nil id, is refused before anything happens, and so is any
//! change while a transaction or a group is open.

use std::collections::HashSet;
use std::sync::Arc;

use kentos_contracts::{Entity, LayerNode, ProjectSettings, ProjectStyles};

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
}

/// What came from outside.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct External {
    /// Objects as they are now, each under its persistent id (`id` is ignored).
    pub put: Vec<(Uuid, Entity)>,
    /// Objects removed; ids the drawing does not have are skipped.
    pub remove: Vec<Uuid>,
    pub meta: Option<ExternalMeta>,
}

impl External {
    pub fn is_empty(&self) -> bool {
        self.put.is_empty() && self.remove.is_empty() && self.meta.is_none()
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
        let mut slots = HashSet::new();
        for (uid, mut entity) in change.put {
            let slot = match self.store.slot_of(uid) {
                Some(slot) => slot,
                None => {
                    // Checked above: every new object has a slot left.
                    let slot = Slot(u32::try_from(self.next_slot).unwrap_or(u32::MAX));
                    self.next_slot += 1;
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
        let changed = !ops.is_empty() || change.meta.is_some();
        for op in &ops {
            self.apply_op(op);
        }
        if let Some(meta) = change.meta {
            if let Some(layers) = meta.layers {
                let active = self.layers.active().to_owned();
                self.layers = LayerTree::new(layers, &active);
            }
            if let Some(settings) = meta.settings {
                self.settings = settings;
            }
            if let Some(name) = meta.name {
                self.name = name;
            }
            if let Some(styles) = meta.styles {
                self.styles = styles;
            }
        }
        self.forget_history_of(&slots, &named);
        if changed {
            // Not an edit (the revision stays), but what the drawing shows changed.
            self.generation += 1;
        }
        Ok(())
    }
}
