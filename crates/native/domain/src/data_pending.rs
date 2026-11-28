//! The rows of a data layer's session a saved file leaves out (docs/adr/9111 §3). A file keeps the new rows as the
//! layer's objects (docs/adr/9103 §2) but not the rows taken from the table, nor the keys deleted without being taken:
//! they are the table's data, personal data among it (6698 md. 12), and a file goes from hand to hand. A frontend keeps
//! them on the device beside the file ([`Document::pending_rows`]) and gives them back when the file is opened again
//! ([`Document::restore_pending`]); a save checks them against the table as it checks any taken row (docs/adr/9104 §3).
//!
//! What left the drawing because it was written to its table or let go is kept in order ([`Document::data_left`]): a
//! file saved before holds those objects as new rows, and opened again they would be written a second time. A frontend
//! takes them out of the drawing it opens ([`Document::drop_left`]).

use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use kentos_contracts::Entity;

use crate::data_edits::Taken;
use crate::document::Document;
use crate::external::External;
use crate::identity::Uuid;

/// A taken row a file leaves out.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingRow {
    /// Its object's persistent id.
    pub uid: Uuid,
    /// The row's key in its table.
    pub key: i64,
    /// The row as it came: what a save checks the table against.
    pub row: Entity,
    /// Its object now; none: erased, the row to be deleted.
    pub now: Option<Entity>,
}

/// A data layer's rows a file leaves out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PendingLayer {
    /// The layer's id.
    pub layer: String,
    /// The taken rows it changes or deletes.
    pub taken: Vec<PendingRow>,
    /// Keys deleted without being taken, ascending.
    pub deleted: Vec<i64>,
}

impl PendingLayer {
    pub fn is_empty(&self) -> bool {
        self.taken.is_empty() && self.deleted.is_empty()
    }
}

impl Document {
    /// Each edited layer's rows a saved file leaves out (see the module), in the tree's order. A taken row still as it
    /// came is not among them (nothing of it waits), nor is a layer with none.
    pub fn pending_rows(&self) -> Vec<PendingLayer> {
        self.edited_layers()
            .into_iter()
            .filter_map(|layer| {
                let edit = self.data_edits.get(&layer)?;
                let taken = edit
                    .taken
                    .iter()
                    .filter_map(|(uid, t)| {
                        let now = self
                            .store
                            .slot_of(*uid)
                            .and_then(|s| self.store.get(s))
                            .filter(|s| s.entity.base().layer_id == layer)
                            .map(|s| s.entity.clone());
                        if now.as_ref().is_some_and(|e| **e == *t.row) {
                            return None;
                        }
                        Some(PendingRow {
                            uid: *uid,
                            key: t.key,
                            row: (*t.row).clone(),
                            now: now.map(|e| (*e).clone()),
                        })
                    })
                    .collect();
                let pending = PendingLayer {
                    layer,
                    taken,
                    deleted: edit.deleted.iter().copied().collect(),
                };
                (!pending.is_empty()).then_some(pending)
            })
            .collect()
    }

    /// Gives a drawing just opened the rows its file left out (see the module): each layer opened for editing, its taken
    /// rows put back as its objects beside their rows as they came, its deleted keys kept. Not an edit: no undo step, the
    /// drawing stays clean. What could not be given back is said, a sentence each: a layer the drawing has not, or not a
    /// data layer one may edit; a row the drawing holds already.
    pub fn restore_pending(&mut self, pending: Vec<PendingLayer>) -> Vec<String> {
        let mut said = Vec::new();
        for p in pending {
            if let Err(why) = self.start_editing(&p.layer) {
                said.push(why.0);
                continue;
            }
            let held: HashSet<i64> = self
                .data_edits
                .get(&p.layer)
                .map(|e| e.taken.values().map(|t| t.key).collect())
                .unwrap_or_default();
            let mut put = Vec::new();
            let mut erased = Vec::new();
            for r in p.taken {
                if self.store.slot_of(r.uid).is_some() || held.contains(&r.key) {
                    said.push(format!(
                        "{} anahtarlı satır çizimde zaten var; bekleyen hâli geri getirilmedi.",
                        r.key
                    ));
                    continue;
                }
                match r.now {
                    Some(mut now) => {
                        p.layer.clone_into(&mut now.base_mut().layer_id);
                        put.push((r.uid, now, r.key, r.row));
                    }
                    None => erased.push((
                        r.uid,
                        Taken {
                            key: r.key,
                            row: Arc::new(r.row),
                        },
                    )),
                }
            }
            let rows: Vec<(Uuid, i64, Entity)> = put
                .iter()
                .map(|(u, _, k, row)| (*u, *k, row.clone()))
                .collect();
            if !put.is_empty()
                && let Err(why) = self.apply_external(External {
                    put: put.into_iter().map(|(u, e, _, _)| (u, e)).collect(),
                    ..External::default()
                })
            {
                said.push(why);
                continue;
            }
            let mut taken: Vec<(Uuid, Taken)> = rows
                .into_iter()
                .filter_map(|(uid, key, mut row)| {
                    let stored = self.store.get(self.store.slot_of(uid)?)?;
                    // The row as it came takes the id its object got now, as a row taken from the table does: a change is
                    // what differs from it.
                    row.base_mut().id = stored.entity.base().id;
                    Some((
                        uid,
                        Taken {
                            key,
                            row: Arc::new(row),
                        },
                    ))
                })
                .collect();
            taken.extend(erased);
            if let Some(edit) = Arc::make_mut(&mut self.data_edits).get_mut(&p.layer) {
                edit.taken.extend(taken);
                if !p.deleted.is_empty() {
                    let mut next = (*edit.deleted).clone();
                    next.extend(p.deleted);
                    edit.deleted = Arc::new(next);
                }
            }
        }
        said
    }

    /// The objects that left the drawing because they were written to their tables or let go, in the order they left,
    /// since the drawing was made or read.
    pub fn data_left(&self) -> &[Uuid] {
        &self.data_left
    }

    /// Takes out of a drawing just opened the data layers' objects its file held but that were written to their tables
    /// or let go after it was saved (see the module): no undo step, the drawing stays clean. A layer left with nothing to
    /// save stops being edited (its file's objects had opened it). How many objects went.
    pub fn drop_left(&mut self, uids: &[Uuid]) -> usize {
        let mut seen = HashSet::new();
        let mut layers = BTreeSet::new();
        let remove: Vec<Uuid> = uids
            .iter()
            .filter(|u| seen.insert(**u))
            .filter(|u| {
                let Some(stored) = self.store.slot_of(**u).and_then(|s| self.store.get(s)) else {
                    return false;
                };
                let layer = &stored.entity.base().layer_id;
                let data = self.layers.get(layer).is_some_and(|n| n.data.is_some());
                if data {
                    layers.insert(layer.clone());
                }
                data
            })
            .copied()
            .collect();
        if remove.is_empty()
            || self
                .apply_external(External {
                    remove: remove.clone(),
                    ..External::default()
                })
                .is_err()
        {
            return 0;
        }
        for layer in layers {
            let idle = self
                .data_edits
                .get(&layer)
                .is_some_and(|e| e.taken.is_empty() && e.deleted.is_empty());
            if idle && self.store.layer_slots(&layer).next().is_none() {
                Arc::make_mut(&mut self.data_edits).remove(&layer);
            }
        }
        remove.len()
    }

    /// The row a taken object stands for, as it came (or as a save last wrote it).
    pub fn taken_row(&self, layer: &str, uid: Uuid) -> Option<Arc<Entity>> {
        Some(self.data_edits.get(layer)?.taken.get(&uid)?.row.clone())
    }

    /// The keys of `layer` deleted without being taken, ascending.
    pub fn deleted_keys(&self, layer: &str) -> Vec<i64> {
        self.data_edits
            .get(layer)
            .map(|e| e.deleted.iter().copied().collect())
            .unwrap_or_default()
    }
}
