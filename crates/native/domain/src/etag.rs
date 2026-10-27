//! State stamps (docs/adr/0253 §2): 16 hex digits of the SHA-256 of a
//! state's canonical JSON (`kentos_contracts::canonical`). A stamp comes from
//! the content alone, so it is the same in every host, across a save and an
//! opening, and again when undo brings the state back.
//!
//! - an object: its persistent id and its data without its slot;
//! - a block definition: the definition;
//! - the layer tree: its nodes and the active layer;
//! - the settings: the drawing's name and the project's settings;
//! - the styles: the project's style library;
//! - the sheets: the book and its pictures' SHA-256s (`null` without a book).

use kentos_contracts::canonical::stamp_of;
use kentos_contracts::{BlockDefinition, BlockId, Entity};
use serde_json::{Value, json};

use crate::document::Document;
use crate::identity::{Slot, Uuid};

/// An object's data as its stamp reads it: its JSON without its slot.
pub fn entity_value(entity: &Entity) -> Value {
    let mut v = serde_json::to_value(entity).unwrap_or(Value::Null);
    if let Some(map) = v.as_object_mut() {
        map.remove("id");
    }
    v
}

/// The stamp of the object `uid` holding `entity`.
pub fn entity_stamp(uid: Uuid, entity: &Entity) -> String {
    stamp_of(&json!({ "uid": uid.to_string(), "entity": entity_value(entity) }))
}

/// The stamp of a block definition.
pub fn block_stamp(block: &BlockDefinition) -> String {
    stamp_of(&serde_json::to_value(block).unwrap_or(Value::Null))
}

impl Document {
    /// The stamp of the object in `slot`.
    pub fn stamp(&self, slot: Slot) -> Option<String> {
        self.store
            .get(slot)
            .map(|stored| entity_stamp(stored.uid, &stored.entity))
    }

    /// The stamp of the object with persistent id `uid`.
    pub fn stamp_of_uid(&self, uid: Uuid) -> Option<String> {
        self.store.slot_of(uid).and_then(|slot| self.stamp(slot))
    }

    /// The stamp of the block definition `id`.
    pub fn block_stamp(&self, id: BlockId) -> Option<String> {
        self.block(id).map(block_stamp)
    }

    /// The layer tree's stamp.
    pub fn layers_stamp(&self) -> String {
        stamp_of(&json!({
            "nodes": self.layers.nodes(),
            "active": self.layers.active(),
        }))
    }

    /// The drawing's name and the project's settings' stamp.
    pub fn settings_stamp(&self) -> String {
        stamp_of(&json!({ "name": self.name, "settings": self.settings }))
    }

    /// The project's style library's stamp.
    pub fn styles_stamp(&self) -> String {
        stamp_of(&serde_json::to_value(&self.styles).unwrap_or(Value::Null))
    }

    /// The constraints' and parameters' stamp (docs/adr/0336 §3).
    pub fn constraints_stamp(&self) -> String {
        stamp_of(&serde_json::to_value(&*self.constraints).unwrap_or(Value::Null))
    }

    /// The complete recipe state, including suppressed segments and manual row placement.
    pub fn dimension_sets_stamp(&self) -> String {
        let sets: Vec<_> = self.dimension_sets.iter().map(|s| &**s).collect();
        stamp_of(&serde_json::to_value(sets).unwrap_or(Value::Null))
    }

    /// The sheet book's stamp.
    pub fn sheets_stamp(&self) -> String {
        let value = match self.sheets.as_deref() {
            None => Value::Null,
            Some(sheets) => json!({
                "book": sheets.book,
                "pictures": sheets.pictures.iter().map(|p| p.sha256.as_str()).collect::<Vec<_>>(),
            }),
        };
        stamp_of(&value)
    }
}
