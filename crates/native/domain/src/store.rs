//! The document's objects: by slot, in document order, per layer in document
//! order, and by persistent id.
//!
//! Document order is the order objects entered the document, as in the web's
//! `Map` (apps/web/src/model/document.ts): changing an object keeps its place,
//! even on another layer; an object removed and put back (undo, a rolled back
//! transaction) goes to the end. It is the drawing order and the order a saved
//! file lists the objects in.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use kentos_contracts::Entity;

use crate::identity::{Slot, Uuid};

/// An object as the document keeps it: its persistent id and its data, whose
/// `id` is the slot. Never changed in place: an edit stores a new value, so the
/// store and the undo records share the unchanged ones.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Stored {
    pub uid: Uuid,
    pub entity: Arc<Entity>,
}

impl Stored {
    pub fn slot(&self) -> Slot {
        Slot(self.entity.base().id)
    }

    fn layer(&self) -> &str {
        &self.entity.base().layer_id
    }
}

#[derive(Clone, Debug)]
struct Item {
    /// Place in the document: larger is later.
    seq: u64,
    stored: Stored,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Store {
    items: HashMap<Slot, Item>,
    order: BTreeMap<u64, Slot>,
    layers: HashMap<String, BTreeMap<u64, Slot>>,
    uids: HashMap<Uuid, Slot>,
    next_seq: u64,
}

impl Store {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, slot: Slot) -> Option<&Stored> {
        self.items.get(&slot).map(|item| &item.stored)
    }

    pub fn slot_of(&self, uid: Uuid) -> Option<Slot> {
        self.uids.get(&uid).copied()
    }

    /// Every object in document order.
    pub fn iter(&self) -> impl Iterator<Item = &Stored> {
        self.order.values().filter_map(|slot| self.get(*slot))
    }

    /// The objects of one layer in document order.
    pub fn on_layer(&self, layer: &str) -> impl Iterator<Item = &Stored> {
        self.layers
            .get(layer)
            .into_iter()
            .flat_map(|list| list.values())
            .filter_map(|slot| self.get(*slot))
    }

    pub fn count(&self, layer: &str) -> usize {
        self.layers.get(layer).map_or(0, BTreeMap::len)
    }

    /// Sets an object. A known slot keeps its place in the document (and in
    /// its new layer's list when the layer changed); a new one goes last.
    pub fn put(&mut self, stored: Stored) {
        let slot = stored.slot();
        let Some(item) = self.items.get_mut(&slot) else {
            let seq = self.next_seq;
            self.next_seq += 1;
            self.order.insert(seq, slot);
            self.layers
                .entry(stored.layer().to_owned())
                .or_default()
                .insert(seq, slot);
            self.uids.insert(stored.uid, slot);
            self.items.insert(slot, Item { seq, stored });
            return;
        };
        if item.stored.layer() != stored.layer() {
            if let Some(list) = self.layers.get_mut(item.stored.layer()) {
                list.remove(&item.seq);
            }
            self.layers
                .entry(stored.layer().to_owned())
                .or_default()
                .insert(item.seq, slot);
        }
        if item.stored.uid != stored.uid {
            self.uids.remove(&item.stored.uid);
            self.uids.insert(stored.uid, slot);
        }
        item.stored = stored;
    }

    pub fn remove(&mut self, slot: Slot) -> Option<Stored> {
        let item = self.items.remove(&slot)?;
        self.order.remove(&item.seq);
        if let Some(list) = self.layers.get_mut(item.stored.layer()) {
            list.remove(&item.seq);
        }
        self.uids.remove(&item.stored.uid);
        Some(item.stored)
    }
}
