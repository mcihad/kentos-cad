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

use std::sync::{Arc, OnceLock};

use kentos_contracts::canonical::stamp_of;
use kentos_contracts::{
    BlockDefinition, BlockId, ConstraintSet, DimensionSet, Entity, ProjectSettings, ProjectSheets,
    ProjectStyles,
};
use serde_json::{Value, json};

use crate::document::Document;
use crate::identity::{Slot, Uuid};
use crate::layers::LayerTree;

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

fn layers_stamp_of(layers: &LayerTree) -> String {
    stamp_of(&json!({
        "nodes": layers.nodes(),
        "active": layers.active(),
    }))
}

fn settings_stamp_of(name: &str, settings: &ProjectSettings) -> String {
    stamp_of(&json!({ "name": name, "settings": settings }))
}

fn styles_stamp_of(styles: &ProjectStyles) -> String {
    stamp_of(&serde_json::to_value(styles).unwrap_or(Value::Null))
}

fn constraints_stamp_of(constraints: &ConstraintSet) -> String {
    stamp_of(&serde_json::to_value(constraints).unwrap_or(Value::Null))
}

fn dimension_sets_stamp_of(sets: &[Arc<DimensionSet>]) -> String {
    let sets: Vec<_> = sets.iter().map(|s| &**s).collect();
    stamp_of(&serde_json::to_value(sets).unwrap_or(Value::Null))
}

fn sheets_stamp_of(sheets: Option<&ProjectSheets>) -> String {
    let value = match sheets {
        None => Value::Null,
        Some(sheets) => json!({
            "book": sheets.book,
            "pictures": sheets.pictures.iter().map(|p| p.sha256.as_str()).collect::<Vec<_>>(),
        }),
    };
    stamp_of(&value)
}

/// The project's parts as they stood, kept to be stamped later (docs/adr/8054 §2): shared, nothing serialised until a
/// stamp is asked, each stamp taken at most once. A recorded call keeps them before its write and stamps only the parts
/// the write changed; before, it serialised and hashed all six before every call (40 ms with 16 MB of bound sources'
/// bases, 135 ms with 64 MB). The stamps are the document's own (`settings_stamp` …).
#[derive(Clone, Debug)]
pub struct PartsAt {
    name: String,
    settings: Arc<ProjectSettings>,
    styles: Arc<ProjectStyles>,
    sheets: Option<Arc<ProjectSheets>>,
    constraints: Arc<ConstraintSet>,
    dimension_sets: Vec<Arc<DimensionSet>>,
    layers: LayerTree,
    stamps: [OnceLock<String>; 6],
}

impl PartsAt {
    fn once(&self, part: usize, stamp: impl FnOnce() -> String) -> String {
        self.stamps[part].get_or_init(stamp).clone()
    }

    /// The layer tree's stamp, as [`Document::layers_stamp`] then.
    pub fn layers_stamp(&self) -> String {
        self.once(0, || layers_stamp_of(&self.layers))
    }

    /// The name's and settings' stamp, as [`Document::settings_stamp`] then.
    pub fn settings_stamp(&self) -> String {
        self.once(1, || settings_stamp_of(&self.name, &self.settings))
    }

    /// The style library's stamp, as [`Document::styles_stamp`] then.
    pub fn styles_stamp(&self) -> String {
        self.once(2, || styles_stamp_of(&self.styles))
    }

    /// The sheet book's stamp, as [`Document::sheets_stamp`] then.
    pub fn sheets_stamp(&self) -> String {
        self.once(3, || sheets_stamp_of(self.sheets.as_deref()))
    }

    /// The constraints' stamp, as [`Document::constraints_stamp`] then.
    pub fn constraints_stamp(&self) -> String {
        self.once(4, || constraints_stamp_of(&self.constraints))
    }

    /// The dimension sets' stamp, as [`Document::dimension_sets_stamp`] then.
    pub fn dimension_sets_stamp(&self) -> String {
        self.once(5, || dimension_sets_stamp_of(&self.dimension_sets))
    }
}

impl Document {
    /// The project's parts as they are now, to be stamped later (docs/adr/8054 §2): nothing copied but the layer tree.
    pub fn parts_at(&self) -> PartsAt {
        PartsAt {
            name: self.name.clone(),
            settings: self.settings.clone(),
            styles: self.styles.clone(),
            sheets: self.sheets.clone(),
            constraints: self.constraints.clone(),
            dimension_sets: self.dimension_sets.clone(),
            layers: self.layers.clone(),
            stamps: Default::default(),
        }
    }

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
        layers_stamp_of(&self.layers)
    }

    /// The drawing's name and the project's settings' stamp.
    pub fn settings_stamp(&self) -> String {
        settings_stamp_of(&self.name, &self.settings)
    }

    /// The project's style library's stamp.
    pub fn styles_stamp(&self) -> String {
        styles_stamp_of(&self.styles)
    }

    /// The constraints' and parameters' stamp (docs/adr/0336 §3).
    pub fn constraints_stamp(&self) -> String {
        constraints_stamp_of(&self.constraints)
    }

    /// The complete recipe state, including suppressed segments and manual row placement.
    pub fn dimension_sets_stamp(&self) -> String {
        dimension_sets_stamp_of(&self.dimension_sets)
    }

    /// The sheet book's stamp.
    pub fn sheets_stamp(&self) -> String {
        sheets_stamp_of(self.sheets.as_deref())
    }
}
