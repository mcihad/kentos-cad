//! `cad.layers.session` v1 (docs/adr/9103 §3): a data layer's edit session. A layer is opened for editing, its changes
//! wait in the drawing and are written to its table in one transaction (Kaydet), let go (Geri al) or let go as it closes
//! (Vazgeç); a layer without changes closes (Düzenlemeyi bitir). Python and the assistants edit a data layer with it and
//! the object commands: a new row is an object drawn on the layer; a row taken by its key (`take`) is an object the
//! object commands change, move or delete; `delete` deletes rows by key without taking them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_LAYERS_SESSION: &str = "cad.layers.session";
pub const CAD_LAYERS_SESSION_VERSION: u32 = 1;
/// The most keys `take` and `delete` name at once.
pub const LAYER_SESSION_KEYS_MOST: usize = 10_000;

/// What `cad.layers.session` does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
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
/// writes over it).
///
/// The project's edit grouping (`settings.editGrouping`, docs/adr/9104 §2) widens `start`, `save`, `rollback`, `cancel`,
/// `stop` and `status` of a named layer to the layers of its source, or to every data layer; `save` then writes a
/// source's layers in one transaction.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersSession {
    pub action: SessionAction,
    /// The data layers (their ids). None: every layer being edited (`save`, `rollback`, `cancel`, `stop`, `status`).
    /// `start` names at least one; `take` and `delete` exactly one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
    /// `take`, `delete`: the rows' keys, 1–10 000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub keys: Option<Vec<i64>>,
    /// `save`: write over the rows others changed or deleted since they were taken (a `rows_changed` otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub force: Option<bool>,
}

/// A layer's session after the action.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
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
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<i64>>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub keys: Vec<i64>,
    /// `save` of several layers: why this one was not written (its changes wait); none: written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub failed: Option<String>,
}

/// Output of `cad.layers.session` v1.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersSessionDone {
    /// The layers acted on, in the tree's order.
    pub layers: Vec<SessionLayer>,
    /// `take`: the objects' persistent ids for the keys, in order; none for a row deleted or not in the table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<Option<String>>>"))]
    pub objects: Vec<Option<String>>,
    pub revision: String,
}
