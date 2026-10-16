//! `cad.records.create` (docs/adr/0410 §3): records without a geometry, a table's rows, on a named layer.
//! The desktop's handler is `crates/native/application/src/records.rs`; the shared cases are
//! `fixtures/commands/v1/cad.records.create.json`. The web is paused (docs/adr/0228).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Entity;

/// Writes records without a geometry on a named layer.
pub const CAD_RECORDS_CREATE: &str = "cad.records.create";
pub const CAD_RECORDS_CREATE_VERSION: u32 = 1;

/// The most records one call writes.
pub const MAX_RECORDS: usize = 10_000;

/// Input of `cad.records.create` v1: records on a named layer, written as one undo step (“Kayıt ekle”).
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_count` (`rows`: 1 to 10 000),
/// `invalid_revision`, `revision_conflict` (status `conflict`), `layer_not_found`, `not_a_layer`,
/// `layer_locked`, then each row's attributes by the layer's fields (`invalid_attribute`,
/// `attribute_required`, at `rows.<i>.attrs.<name>`); on the desktop also `slots_exhausted`. Warning:
/// `layer_hidden` (they are written all the same).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordsCreate {
    /// The layer they go on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// The records, in order: 1 to 10 000.
    pub rows: Vec<RecordRow>,
    /// The document revision the caller saw (decimal text, as a result's `revision`). Absent: no check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// One record: its attributes and its label.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordRow {
    /// Its attributes, by the layer's fields: canonical, a field's default where none is given. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    /// Its label (`EntityBase.label`), shown in the table and the forms. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
}

/// Output of `cad.records.create` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordsCreated {
    /// Their persistent ids (UUIDv7, docs/adr/0014), in the rows' order.
    pub uids: Vec<String>,
    /// Their slots in the open document, in the rows' order.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.records.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordsPlan {
    /// The records as they would be stored; their `id`s are 0: the slots are given when they are written.
    pub entities: Vec<Entity>,
    /// The revision to expect for exactly this.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
