//! Veri katmanlarının düzenleme oturumu (docs/adr/9103 §2, plan/veri.md VY-05): a data layer opened for editing keeps
//! its changes in the drawing until they are saved to its table or let go.
//!
//! - A new row is an object drawn on the layer.
//! - A row of the table to be changed is taken into the drawing ([`Document::take_data_rows`]): an object of the layer made
//!   from the row, its key and the row as it came kept beside it. Taking is not an edit: no undo step, the drawing not
//!   made unsaved. What is done to the object afterwards is one: moved, a value changed, erased.
//! - A row deleted without being taken is kept by its key ([`Document::delete_data_rows`], an undo step).
//!
//! What the layer would write is worked out when asked ([`Document::data_changes`]): an object of the layer without a
//! key is a new row; a taken object that differs from its row changes the row; a taken object gone (erased, moved to
//! another layer) deletes its row, as a key deleted without being taken does.
//!
//! Saving ([`Document::data_saved`]), letting go ([`Document::discard_data_edits`]) and stopping
//! ([`Document::stop_editing`]) take the layer's objects out of the drawing as another editor's change would (no undo
//! step) and drop the undo steps that touched the layer's objects: a row written to its table is not unwritten from
//! here.
//!
//! The session belongs to the open drawing, not to its file: a saved file keeps the new rows (objects of the layer) and
//! leaves the taken ones out (their rows are in the table). Opened again, a data layer holding objects is being edited.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use kentos_contracts::{Entity, LayerNodeType};

use crate::document::Document;
use crate::edit::Refusal;
use crate::external::External;
use crate::history::{GisOp, Op};
use crate::identity::{Uuid, new_uid};

/// The undo step's name of [`Document::delete_data_rows`].
pub const DELETE_ROWS: &str = "Satırları sil";

/// A row of the table taken into the drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct Taken {
    /// The row's key in its table.
    pub key: i64,
    /// The row's object as it came (its id the slot it got): what a change is found against.
    pub row: Arc<Entity>,
}

/// One data layer's session.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LayerEdit {
    pub(crate) taken: BTreeMap<Uuid, Taken>,
    /// Keys deleted without being taken; an undo step changes them ([`GisOp::DataDeleted`]).
    pub(crate) deleted: Arc<BTreeSet<i64>>,
}

/// The sessions of the drawing's data layers, by layer id.
pub(crate) type DataEdits = BTreeMap<String, LayerEdit>;

/// A row the layer would change.
#[derive(Clone, Debug, PartialEq)]
pub struct DataChange {
    pub key: i64,
    pub uid: Uuid,
    /// The object now.
    pub now: Arc<Entity>,
    /// The row as it came.
    pub row: Arc<Entity>,
}

/// What a layer would write: the drawing as it is now against the rows taken.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataChanges {
    /// New rows: the layer's objects without a key, in the drawing's order.
    pub added: Vec<(Uuid, Arc<Entity>)>,
    pub changed: Vec<DataChange>,
    /// Keys of rows to delete, ascending.
    pub deleted: Vec<i64>,
    /// The taken rows it changes or deletes, by key: their objects' ids and the rows as they came, what a save checks the
    /// table against (docs/adr/9104 §3).
    pub originals: BTreeMap<i64, (Uuid, Arc<Entity>)>,
}

impl DataChanges {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.deleted.is_empty()
    }

    /// How many rows it writes.
    pub fn count(&self) -> usize {
        self.added.len() + self.changed.len() + self.deleted.len()
    }
}

/// What a save wrote to the table (see [`Document::data_saved`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataWritten {
    /// New rows: the object's id, the object as written, the key the table gave it.
    pub added: Vec<(Uuid, Arc<Entity>, i64)>,
    /// Changed rows: the object's id and the object as written.
    pub changed: Vec<(Uuid, Arc<Entity>)>,
    /// Keys deleted.
    pub deleted: Vec<i64>,
}

impl Document {
    /// Whether `layer` is being edited.
    pub fn editing(&self, layer: &str) -> bool {
        self.data_edits.contains_key(layer)
    }

    /// The layers being edited, in the tree's order.
    pub fn edited_layers(&self) -> Vec<String> {
        if self.data_edits.is_empty() {
            return Vec::new();
        }
        self.layers
            .leaves()
            .into_iter()
            .filter(|n| self.editing(&n.id))
            .map(|n| n.id.clone())
            .collect()
    }

    /// The refusal of a write to `layer` while it is not being edited.
    fn not_editing(&self, layer: &str) -> Refusal {
        let name = self.layers.get(layer).map_or(layer, |n| n.name.as_str());
        Refusal(format!(
            "“{name}” düzenlemeye açık değil. Katmanlar'da katmanın sağ tuşundan Düzenlemeyi başlat'ı seçin."
        ))
    }

    /// Opens a data layer for editing; not an edit (no undo step, the drawing not made unsaved). Refused: an unknown
    /// layer, a group, a layer that is not a data layer, one kept read-only. Whether it was not open already.
    pub fn start_editing(&mut self, layer: &str) -> Result<bool, Refusal> {
        let Some(node) = self.layers.get(layer) else {
            return Err(Refusal(format!(
                "“{layer}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
            )));
        };
        let name = &node.name;
        let Some(data) = node.data.as_ref().filter(|_| node.kind == LayerNodeType::Layer) else {
            return Err(Refusal(format!(
                "“{name}” bir veri katmanı değil; düzenlemeye yalnız bir tablodan gelen katman açılır. Çizim katmanlarının nesneleri her zaman düzenlenir."
            )));
        };
        if data.read_only {
            return Err(Refusal(format!(
                "“{name}” salt okunur bir veri katmanı; düzenlenemez. Kaynağı yazmaya izin veriyorsa katmanın salt okunurluğunu kaldırın."
            )));
        }
        if self.editing(layer) {
            return Ok(false);
        }
        Arc::make_mut(&mut self.data_edits).insert(layer.to_owned(), LayerEdit::default());
        Ok(true)
    }

    /// Takes rows of the table into the drawing to be changed (see the module): each `(key, object)` becomes an object
    /// of the layer. A row taken already gives its object's id; a row deleted, none. Refused while the layer is not
    /// being edited or an edit is open.
    pub fn take_data_rows(
        &mut self,
        layer: &str,
        rows: Vec<(i64, Entity)>,
    ) -> Result<Vec<Option<Uuid>>, Refusal> {
        let Some(edit) = self.data_edits.get(layer) else {
            return Err(self.not_editing(layer));
        };
        let mut by_key: HashMap<i64, Uuid> = edit.taken.iter().map(|(u, t)| (t.key, *u)).collect();
        let mut out = Vec::with_capacity(rows.len());
        let mut put = Vec::new();
        for (key, mut entity) in rows {
            if edit.deleted.contains(&key) {
                out.push(None);
                continue;
            }
            if let Some(uid) = by_key.get(&key) {
                out.push(Some(*uid));
                continue;
            }
            layer.clone_into(&mut entity.base_mut().layer_id);
            let uid = new_uid();
            by_key.insert(key, uid);
            put.push((uid, entity, key));
            out.push(Some(uid));
        }
        if put.is_empty() {
            return Ok(out);
        }
        let keys: Vec<(Uuid, i64)> = put.iter().map(|(u, _, k)| (*u, *k)).collect();
        self.apply_external(External {
            put: put.into_iter().map(|(u, e, _)| (u, e)).collect(),
            ..External::default()
        })
        .map_err(Refusal)?;
        let rows: Vec<(Uuid, Taken)> = keys
            .into_iter()
            .filter_map(|(uid, key)| {
                let stored = self.store.get(self.store.slot_of(uid)?)?;
                Some((uid, Taken { key, row: stored.entity.clone() }))
            })
            .collect();
        if let Some(edit) = Arc::make_mut(&mut self.data_edits).get_mut(layer) {
            edit.taken.extend(rows);
        }
        Ok(out)
    }

    /// The object a row of the table was taken as, by its key.
    pub fn taken_uid(&self, layer: &str, key: i64) -> Option<Uuid> {
        self.data_edits
            .get(layer)?
            .taken
            .iter()
            .find(|(_, t)| t.key == key)
            .map(|(u, _)| *u)
    }

    /// The taken rows still on the layer, by key, as their objects are now (Öznitelik tablosu shows them so).
    pub fn taken_objects(&self, layer: &str) -> Vec<(i64, Arc<Entity>)> {
        let Some(edit) = self.data_edits.get(layer) else {
            return Vec::new();
        };
        edit.taken
            .iter()
            .filter_map(|(uid, t)| {
                let stored = self.store.get(self.store.slot_of(*uid)?)?;
                (stored.entity.base().layer_id == layer).then(|| (t.key, stored.entity.clone()))
            })
            .collect()
    }

    /// The key of the row a taken object stands for.
    pub fn taken_key(&self, layer: &str, uid: Uuid) -> Option<i64> {
        Some(self.data_edits.get(layer)?.taken.get(&uid)?.key)
    }

    /// Deletes rows by key as one undo step, “Satırları sil”: a taken row's object is removed, another row's key kept.
    /// The table says unknown keys on saving. Refused while the layer is not being edited. How many rows it deleted.
    pub fn delete_data_rows(&mut self, layer: &str, keys: &[i64]) -> Result<usize, Refusal> {
        let Some(edit) = self.data_edits.get(layer) else {
            return Err(self.not_editing(layer));
        };
        let by_key: HashMap<i64, Uuid> = edit.taken.iter().map(|(u, t)| (t.key, *u)).collect();
        let mut slots = Vec::new();
        let mut next = (*edit.deleted).clone();
        let mut count = 0;
        let mut seen = HashSet::new();
        for key in keys.iter().filter(|k| seen.insert(**k)) {
            match by_key.get(key) {
                Some(uid) => {
                    if let Some(slot) = self.store.slot_of(*uid)
                        && self
                            .store
                            .get(slot)
                            .is_some_and(|s| s.entity.base().layer_id == layer)
                    {
                        slots.push(slot);
                        count += 1;
                    }
                }
                None => {
                    if next.insert(*key) {
                        count += 1;
                    }
                }
            }
        }
        if count == 0 {
            return Ok(0);
        }
        let before = edit.deleted.clone();
        self.transact(DELETE_ROWS, |doc| {
            if !slots.is_empty() {
                doc.remove(&slots);
            }
            if *before != next {
                doc.record(
                    vec![Op::Gis(GisOp::DataDeleted {
                        layer: layer.to_owned(),
                        before,
                        after: Arc::new(next),
                    })],
                    DELETE_ROWS,
                );
            }
            Ok::<_, Refusal>(())
        })?;
        Ok(count)
    }

    /// What `layer` would write now (see the module); nothing for a layer not being edited.
    pub fn data_changes(&self, layer: &str) -> DataChanges {
        let Some(edit) = self.data_edits.get(layer) else {
            return DataChanges::default();
        };
        let mut out = DataChanges::default();
        for stored in self.store.on_layer(layer) {
            match edit.taken.get(&stored.uid) {
                Some(t) => {
                    if *stored.entity != *t.row {
                        out.changed.push(DataChange {
                            key: t.key,
                            uid: stored.uid,
                            now: stored.entity.clone(),
                            row: t.row.clone(),
                        });
                        out.originals.insert(t.key, (stored.uid, t.row.clone()));
                    }
                }
                None => out.added.push((stored.uid, stored.entity.clone())),
            }
        }
        for (uid, t) in &edit.taken {
            let here = self
                .store
                .slot_of(*uid)
                .and_then(|s| self.store.get(s))
                .is_some_and(|s| s.entity.base().layer_id == layer);
            if !here {
                out.deleted.push(t.key);
                out.originals.insert(t.key, (*uid, t.row.clone()));
            }
        }
        out.deleted.extend(edit.deleted.iter().copied());
        out.deleted.sort_unstable();
        out.deleted.dedup();
        out
    }

    /// Whether `layer` has changes to save.
    pub fn data_modified(&self, layer: &str) -> bool {
        !self.data_changes(layer).is_empty()
    }

    /// The keys of the rows the view leaves out of `layer`'s table: the taken ones (their objects draw them) and the
    /// deleted ones.
    pub fn data_hidden(&self, layer: &str) -> BTreeSet<i64> {
        let Some(edit) = self.data_edits.get(layer) else {
            return BTreeSet::new();
        };
        let mut out: BTreeSet<i64> = edit.taken.values().map(|t| t.key).collect();
        out.extend(edit.deleted.iter().copied());
        out
    }

    /// Takes what a save wrote out of the drawing (see the module): an object still as it was written leaves the drawing,
    /// its row in the table now; one changed or erased since stays taken against the row as written, so the next save
    /// writes what came after. The deleted keys written are let go; a taken object whose row was deleted while it came
    /// back (an undo during the save) becomes a new row. The undo steps that touched the layer's objects are dropped.
    pub fn data_saved(&mut self, layer: &str, written: DataWritten) -> Result<(), String> {
        if !self.editing(layer) {
            return Ok(());
        }
        self.data_written += 1;
        let now = |doc: &Document, uid: Uuid| {
            doc.store
                .slot_of(uid)
                .and_then(|s| doc.store.get(s))
                .filter(|s| s.entity.base().layer_id == layer)
                .map(|s| s.entity.clone())
        };
        let mut remove = Vec::new();
        let mut taken: Vec<(Uuid, Taken)> = Vec::new();
        for (uid, as_written, key) in &written.added {
            if now(self, *uid).is_some_and(|e| *e == **as_written) {
                remove.push(*uid);
            } else {
                taken.push((*uid, Taken { key: *key, row: as_written.clone() }));
            }
        }
        let mut changed: Vec<(Uuid, Option<Arc<Entity>>)> = Vec::new();
        for (uid, as_written) in &written.changed {
            if now(self, *uid).is_some_and(|e| *e == **as_written) {
                remove.push(*uid);
                changed.push((*uid, None));
            } else {
                changed.push((*uid, Some(as_written.clone())));
            }
        }
        let gone: HashSet<i64> = written.deleted.iter().copied().collect();
        if let Some(edit) = Arc::make_mut(&mut self.data_edits).get_mut(layer) {
            for (uid, row) in changed {
                match row {
                    None => {
                        edit.taken.remove(&uid);
                    }
                    Some(row) => {
                        if let Some(t) = edit.taken.get_mut(&uid) {
                            t.row = row;
                        }
                    }
                }
            }
            edit.taken.extend(taken);
            // A deleted row's object is let go: gone, nothing of it stays; back, it is a new row.
            edit.taken.retain(|_, t| !gone.contains(&t.key));
            if edit.deleted.iter().any(|k| gone.contains(k)) {
                edit.deleted = Arc::new(
                    edit.deleted
                        .iter()
                        .copied()
                        .filter(|k| !gone.contains(k))
                        .collect(),
                );
            }
        }
        self.forget_data_history(layer);
        if !remove.is_empty() {
            self.apply_external(External {
                remove,
                ..External::default()
            })?;
        }
        Ok(())
    }

    /// Lets go of `layer`'s changes (Geri al): its objects leave the drawing, the deleted keys are forgotten and the undo
    /// steps that touched its objects dropped; the layer stays open for editing. Whether there was anything to let go.
    pub fn discard_data_edits(&mut self, layer: &str) -> Result<bool, String> {
        let Some(edit) = self.data_edits.get(layer) else {
            return Ok(false);
        };
        let had = !edit.taken.is_empty() || !edit.deleted.is_empty();
        let remove: Vec<Uuid> = self.store.on_layer(layer).map(|s| s.uid).collect();
        let had = had || !remove.is_empty();
        if had {
            self.data_written += 1;
        }
        if let Some(edit) = Arc::make_mut(&mut self.data_edits).get_mut(layer) {
            *edit = LayerEdit::default();
        }
        self.forget_data_history(layer);
        if !remove.is_empty() {
            self.apply_external(External {
                remove,
                ..External::default()
            })?;
        }
        Ok(had)
    }

    /// Closes `layer`'s session (Düzenlemeyi bitir) when it has nothing to save: its taken objects, unchanged, leave the
    /// drawing. Refused while it has changes: save them or let them go first. Whether it was open.
    pub fn stop_editing(&mut self, layer: &str) -> Result<bool, Refusal> {
        if !self.editing(layer) {
            return Ok(false);
        }
        if self.data_modified(layer) {
            let name = self.layers.get(layer).map_or(layer, |n| n.name.as_str());
            return Err(Refusal(format!(
                "“{name}” katmanında kaydedilmemiş değişiklik var; düzenlemeyi bitirmeden önce kaydedin ya da geri alın."
            )));
        }
        let remove: Vec<Uuid> = self.store.on_layer(layer).map(|s| s.uid).collect();
        Arc::make_mut(&mut self.data_edits).remove(layer);
        self.forget_data_history(layer);
        if !remove.is_empty() {
            self.apply_external(External {
                remove,
                ..External::default()
            })
            .map_err(Refusal)?;
        }
        Ok(true)
    }

    /// How many times a data layer's changes were written ([`Document::data_saved`]) or let go
    /// ([`Document::discard_data_edits`]): a table read from its source is read again when this moves, its taken rows
    /// gone back to what the table holds.
    pub fn data_written(&self) -> u64 {
        self.data_written
    }

    /// The objects a saved file leaves out: the rows taken from their tables (their rows are in the tables).
    pub(crate) fn taken_uids(&self) -> HashSet<Uuid> {
        self.data_edits
            .values()
            .flat_map(|e| e.taken.keys().copied())
            .collect()
    }

    /// The drawing with a session for every data layer holding objects (a drawing just read): its objects are its new
    /// rows.
    pub(crate) fn with_held_sessions(mut self) -> Self {
        let held: Vec<String> = self
            .layers
            .leaves()
            .into_iter()
            .filter(|n| n.data.is_some() && self.store.layer_slots(&n.id).next().is_some())
            .map(|n| n.id.clone())
            .collect();
        if !held.is_empty() {
            let edits = Arc::make_mut(&mut self.data_edits);
            for id in held {
                edits.entry(id).or_default();
            }
        }
        self
    }

    /// Drops the undo and redo steps that touched `layer`'s objects or its deleted keys: what they did is written, or
    /// let go.
    fn forget_data_history(&mut self, layer: &str) {
        self.forget_steps(|step| {
            step.ops.iter().any(|op| match op {
                Op::Add(s) | Op::Remove(s) => s.entity.base().layer_id == layer,
                Op::Update { before, after } => {
                    before.entity.base().layer_id == layer || after.entity.base().layer_id == layer
                }
                Op::Gis(GisOp::DataDeleted { layer: l, .. }) => l == layer,
                _ => false,
            })
        });
    }

    /// [`GisOp::DataDeleted`]'s write: the layer's deleted keys, while it is being edited.
    pub(crate) fn set_data_deleted(&mut self, layer: &str, deleted: &Arc<BTreeSet<i64>>) {
        if let Some(edit) = Arc::make_mut(&mut self.data_edits).get_mut(layer) {
            edit.deleted = deleted.clone();
        }
    }
}
