//! `cad.layers.session` v1 (docs/adr/9103 §3): a data layer's edit session. A layer is opened for editing, its changes
//! wait in the drawing and are written to its table in one transaction (Kaydet), let go (Geri al) or let go as it closes
//! (Vazgeç); a layer without changes closes (Düzenlemeyi bitir). Python and the assistants edit a data layer with it and
//! the object commands: a new row is an object drawn on the layer; a row taken by its key (`take`) is an object the
//! object commands change, move or delete; `delete` deletes rows by key without taking them. A save that meets rows
//! others changed since they were taken says them field by field (`conflicts`) and is settled by `choices`, a field,
//! the shape and a deleted row at a time (docs/adr/9193).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const CAD_LAYERS_SESSION: &str = "cad.layers.session";
pub const CAD_LAYERS_SESSION_VERSION: u32 = 1;
/// The most keys `take` and `delete` name at once.
pub const LAYER_SESSION_KEYS_MOST: usize = 10_000;

/// What `cad.layers.session` does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum SessionAction {
    /// Opens the layers for editing; not an edit.
    Start,
    /// Writes each layer's changes to its table, one transaction a layer; the layer stays open.
    Save,
    /// Lets go of each layer's changes; the layer stays open.
    Rollback,
    /// Lets go of each layer's changes and closes it.
    Cancel,
    /// Closes each layer; refused for one with changes not saved.
    Stop,
    /// Takes rows of the layer into the drawing by key, as objects the object commands change.
    Take,
    /// Deletes rows of the layer by key, as one undo step.
    Delete,
    /// Says each layer's session; writes nothing.
    #[default]
    Status,
    /// Says the clashes a save would meet now (docs/adr/9193): the taken rows others changed or deleted since, field by
    /// field with the three values; reads the tables in a transaction that writes nothing.
    Conflicts,
}

/// Which side of a clash stays (docs/adr/9193).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ClashSide {
    /// The value, shape or change here.
    Mine,
    /// The table's.
    Theirs,
}

/// How a save settles one clashing row (docs/adr/9193): by its layer and key, the side each clashing field keeps, the
/// side the shape keeps, and for a row deleted on one side what is done with it (`mine`: a row deleted in the table is
/// written again as a new row, a row deleted here is deleted still; `theirs`: the change here is let go, the row kept).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClashChoice {
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub key: i64,
    /// Each field both sides changed, and the side it keeps.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, ClashSide>,
    /// The shape moved on both sides: the side it keeps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<ClashSide>,
    /// A row deleted on one side: what is done with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<ClashSide>,
}

/// Where a clashing row was deleted (docs/adr/9193).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ClashDeleted {
    /// In the table, the row changed here.
    Theirs,
    /// Here, the row changed in the table.
    Here,
}

/// A field both sides changed to different values, with its three values (none: empty).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct FieldConflict {
    pub field: String,
    /// As the row was taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<String>,
    /// Here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mine: Option<String>,
    /// In the table now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theirs: Option<String>,
}

/// A taken row a save cannot write as it is (docs/adr/9193): its layer, key and object, the fields that clash, whether
/// the shape does, where it was deleted; whether the table takes it again as a new row (the connection's right).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RowConflict {
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub key: i64,
    /// The object's persistent id.
    pub uid: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldConflict>,
    /// The shape moved on both sides.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shape: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted: Option<ClashDeleted>,
    /// A row deleted in the table: whether it can be written again as a new one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub may_add: bool,
}

/// Input of `cad.layers.session` v1.
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer` (`start`, `take` or `delete` without a layer),
/// `one_layer` (`take` or `delete` naming more than one), `no_keys` (`take` or `delete` without keys),
/// `invalid_count` (more than 10 000 keys), `layer_not_found`, `not_a_data_layer`, `data_layer_read_only` (`start` of a
/// layer kept read-only), `data_layer_not_editing` (any other action naming a layer not being edited),
/// `data_edits_unsaved` (`stop` of a layer with changes, or of one that stops with it), `source_unreachable` (the table
/// could not be read or opened; the message says why), `data_save_failed` (the table refused the writes: nothing of the
/// layers written together was written, their changes wait; with several groups, a warning of those not written while
/// the others were), `rows_changed` (a row taken was changed or deleted by someone else since: nothing written; `force`
/// writes over it, `choices` settle it field by field; a clash `choices` leaves unchosen), `clash_choice` (a choice no
/// clash of the save asks for: its row, field or way).
///
/// The project's edit grouping (`settings.editGrouping`, docs/adr/9104 §2) widens `start`, `save`, `rollback`, `cancel`,
/// `stop` and `status` of a named layer to the layers of its source, or to every data layer; `save` then writes a
/// source's layers in one transaction.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayersSession {
    pub action: SessionAction,
    /// The data layers (their ids). None: every layer being edited (`save`, `rollback`, `cancel`, `stop`, `status`).
    /// `start` names at least one; `take` and `delete` exactly one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<String>>,
    /// `take`, `delete`: the rows' keys, 1–10 000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub keys: Option<Vec<i64>>,
    /// `save`: write over the rows others changed or deleted since they were taken (a `rows_changed` otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
    /// `save`: how each clash the save meets is settled (docs/adr/9193), as `conflicts` says them: every clashing
    /// field, shape and deleted row chosen. The rows are made as chosen (one undo step, “Çakışmayı çöz”) and written,
    /// the table checked once more. A clash not chosen is a `rows_changed`; a choice no clash asks for a
    /// `clash_choice`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<ClashChoice>>,
}

/// A layer's session after the action.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SessionLayer {
    pub layer: String,
    /// Being edited after the action.
    pub editing: bool,
    /// New rows waiting; after `save`, the new rows written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    /// Changed rows waiting; after `save`, written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    /// Deleted rows waiting; after `save`, written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub deleted: u32,
    /// `save`: the keys the table gave the new rows, in the drawing's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub keys: Vec<i64>,
    /// `save` of several layers: why this one was not written (its changes wait); none: written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
}

/// Output of `cad.layers.session` v1.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LayersSessionDone {
    /// The layers acted on, in the tree's order.
    pub layers: Vec<SessionLayer>,
    /// `take`: the objects' persistent ids for the keys, in order; none for a row deleted or not in the table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<Option<String>>,
    /// `conflicts`: the clashes a save would meet now; `save` with `choices`: the clashes it settled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<RowConflict>,
    pub revision: String,
}
