//! A bound layer reconciled with its table after both changed (docs/adr/0538; plan/gis.md `CBS-37`).
//!
//! `cad.sources.reconcile` compares the layer and the table's rows with the base (`SourceBase`): each object's hashes
//! when the two last matched. An object only the layer changed is written to the table; one only the table changed
//! (a QGIS edit, another KentOS) is taken into the layer; one both changed the same way agrees. One both changed in
//! different ways, or one side removed and the other changed, is a conflict: the fields that differ and whether the
//! geometry does are said, and it is resolved explicitly by taking a side (`take`, `rest`); the last writer does not
//! win. A plan says it all and writes nothing; executing with every conflict resolved takes the table's side into the
//! layer and writes the layer's to the table, in one undo step and one transaction.
//!
//! Docs/adr/0553 extends it three ways. An object both sides changed is merged field by field against the base when
//! they changed different attributes (or one the attributes and the other the geometry): both win, and only an
//! attribute or a geometry both changed differently stays a conflict, with both values. Changes that belong together
//! (neighbours whose shared vertex both sides moved, a parent one side removed and a child the other added or changed)
//! are one group, resolved together. Several bound layers of one GeoPackage file are reconciled in one call and one
//! transaction (`with`).

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

/// One conflict resolved by hand (an object of a group takes the whole group's side).
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
/// `geometry_mismatch` (a layer's object the table's column cannot hold), `transaction_split` (a layer of `with` in
/// another file), `group_split` (the objects of one group taken to different sides); conflict: `unresolved_conflicts`
/// (executing, a conflict without a side). Warning: `field_base_missing` (objects both sides changed that the base keeps
/// no fields of, which conflict as a whole).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesReconcile {
    /// The bound layer, by id.
    pub layer: String,
    /// More bound layers of the same GeoPackage file, reconciled with it in one transaction (the file's revision is
    /// checked once).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub with: Vec<String>,
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
    /// Not in conflict itself: it belongs to a group with one (docs/adr/0553 §2) and goes the way the group does.
    Related,
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
    /// The object's layer, when more than one layer is reconciled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    pub kind: ConflictKind,
    /// The group it belongs to (the lowest lasting id among its objects), none for a conflict of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// Whether the kind or the geometry differs (a removal: always).
    pub geometry: bool,
    /// The attributes that differ, by name; of an object both changed, only those both changed differently (the rest
    /// merged), unless the base keeps no fields of it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ConflictField>>", optional))]
    pub fields: Vec<ConflictField>,
    /// The side it is resolved to; none: unresolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<ReconcileSide>,
}

/// An object both sides changed and the merge put together (docs/adr/0553 §1): the attributes each side changed, and
/// whose geometry went into it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReconcileMerge {
    pub object: String,
    /// The object's layer, when more than one layer is reconciled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The attributes only the layer changed (its value is in the merge), by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub from_layer: Vec<String>,
    /// The attributes only the table changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub from_table: Vec<String>,
    /// The side whose geometry the merge has, when only one changed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<ReconcileSide>,
}

/// Why objects are one group (docs/adr/0553 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GroupReason {
    /// Neighbours that shared a vertex when the layer and the table last matched, which both sides moved.
    SharedVertex,
    /// A parent one side removed and a child (a relation's) the other side added or changed.
    Relation,
}

/// Objects whose changes belong together, resolved together: one side for all of them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReconcileGroup {
    /// The lowest lasting id among the members.
    pub id: String,
    /// The members' lasting ids, in the ids' order.
    pub objects: Vec<String>,
    pub reasons: Vec<GroupReason>,
    /// The side the group is resolved to; none: unresolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<ReconcileSide>,
}

/// One layer's share of a reconcile of several (docs/adr/0553 §3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReconcileLayer {
    pub layer: String,
    pub from_table: ReconcileCounts,
    pub to_table: ReconcileCounts,
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
    /// Objects both sides changed that were merged field by field (docs/adr/0553 §1), in the lasting ids' order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ReconcileMerge>>", optional))]
    pub merged: Vec<ReconcileMerge>,
    /// Groups of related changes (docs/adr/0553 §2), by their ids' order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ReconcileGroup>>", optional))]
    pub groups: Vec<ReconcileGroup>,
    /// Each layer's share, when more than one layer is reconciled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ReconcileLayer>>", optional))]
    pub layers: Vec<ReconcileLayer>,
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
