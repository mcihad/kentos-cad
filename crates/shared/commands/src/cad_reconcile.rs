//! A bound layer reconciled with its table after both changed (docs/adr/0538; plan/gis.md `CBS-37`).
//!
//! `cad.sources.reconcile` compares the layer and the table's rows with the base (`SourceBase`): each object's hashes
//! when the two last matched. An object only the layer changed is written to the table; one only the table changed
//! (a QGIS edit, another KentOS) is taken into the layer; one both changed the same way agrees. One both changed in
//! different ways, or one side removed and the other changed, is a conflict: the fields that differ and whether the
//! geometry does are said, and it is resolved explicitly by taking a side (`take`, `rest`); the last writer does not
//! win. A plan says it all and writes nothing; executing with every conflict resolved takes the table's side into the
//! layer and writes the layer's to the table, in one undo step and one transaction.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_SOURCES_RECONCILE: &str = "cad.sources.reconcile";
pub const CAD_SOURCES_RECONCILE_VERSION: u32 = 1;

/// The side a conflict is resolved to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReconcileSide {
    /// The layer's object (or its removal) is written to the table.
    Layer,
    /// The table's row (or its removal) is taken into the layer.
    Table,
}

/// One conflict resolved by hand.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReconcileTake {
    /// The object's lasting id.
    pub object: String,
    pub side: ReconcileSide,
}

/// Input of `cad.sources.reconcile` v1. Refusals: `source_not_found`, `layer_not_found`, `source_gone`,
/// `derived_dataset`, `crs_mismatch`, `invalid_extent`, `source_readonly`, `layer_locked`, `no_source_base` (the link has no
/// base for its revision and digest: bound before this version, or a layer of more than `MAX_BASE_ROWS` objects),
/// `not_a_conflict` (a `take` for an object that is not one), `invalid_input` (an object taken twice),
/// `geometry_mismatch` (a layer's object the table's column cannot hold); conflict: `unresolved_conflicts`
/// (executing, a conflict without a side).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesReconcile {
    /// The bound layer, by id.
    pub layer: String,
    /// Conflicts resolved one by one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ReconcileTake>>", optional))]
    pub take: Vec<ReconcileTake>,
    /// The side of every conflict `take` does not name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rest: Option<ReconcileSide>,
}

/// How the two sides of a conflict differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ConflictKind {
    /// Both changed it, differently.
    Changed,
    /// The layer removed it, the table changed it.
    RemovedInLayer,
    /// The table removed it, the layer changed it.
    RemovedInTable,
}

/// An attribute the two sides hold differently; none where a side has no value (or no object: a removed object's
/// attributes are all the other side's).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConflictField {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
}

/// One conflict: the object, how the sides differ (the attributes by name, and the geometry), the side it goes to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReconcileConflict {
    pub object: String,
    pub kind: ConflictKind,
    /// Whether the kind or the geometry differs (a removal: always).
    pub geometry: bool,
    /// The attributes that differ, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ConflictField>>", optional))]
    pub fields: Vec<ConflictField>,
    /// The side it is resolved to; none: unresolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<ReconcileSide>,
}

/// What goes one way: objects added, changed and removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReconcileCounts {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub modified: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
}

/// Output of `cad.sources.reconcile` v1, and its plan: what goes into the layer from the table and into the table from
/// the layer (resolved conflicts counted on their side), the objects both sides changed alike, the conflicts in the
/// lasting ids' order, and the file's revision (plan: before; executed: after).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesReconciled {
    pub from_table: ReconcileCounts,
    pub to_table: ReconcileCounts,
    /// Objects both sides changed (or removed) alike.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub agreed: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ReconcileConflict>>", optional))]
    pub conflicts: Vec<ReconcileConflict>,
    /// Conflicts without a side.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unresolved: u32,
    /// The file's SHA-256 (plan: as it is; executed: after the write).
    pub source: String,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
