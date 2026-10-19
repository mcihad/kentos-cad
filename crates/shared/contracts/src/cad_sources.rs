//! The commands of bound layers (docs/adr/0503, plan/gis.md CBS-01, CBS-02, CBS-11, CBS-36): what a file's tables are
//! and what the project's bound layers' state is (`cad.sources.describe`), binding, unbinding and relocating a layer
//! (`cad.sources.edit`), and writing a layer's changes to its table or reading the table again (`cad.sources.sync`).
//! A layer and its table are compared object by object, by lasting id, on their geometry and attributes.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::fields::LayerFieldKind;
use crate::file_import::ImportedLayer;
use crate::sources::SourceLink;

pub const CAD_SOURCES_DESCRIBE: &str = "cad.sources.describe";
pub const CAD_SOURCES_DESCRIBE_VERSION: u32 = 1;
pub const CAD_SOURCES_EDIT: &str = "cad.sources.edit";
pub const CAD_SOURCES_EDIT_VERSION: u32 = 1;
pub const CAD_SOURCES_SYNC: &str = "cad.sources.sync";
pub const CAD_SOURCES_SYNC_VERSION: u32 = 1;

/// What a source can do (docs/adr/0503 §1): each told apart, so a refusal says which.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceCapabilities {
    pub read: bool,
    pub insert: bool,
    pub update: bool,
    pub delete: bool,
    /// Every write of a sync in one transaction.
    pub transaction: bool,
    pub z: bool,
    pub m: bool,
    /// Arcs kept as arcs.
    pub curves: bool,
}

/// A column of a table and the field kind it reads as; none for one that is not read (a blob).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceColumn {
    pub name: String,
    /// As the file declares it (`MEDIUMINT`, `TEXT(40)` …).
    pub declared: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<LayerFieldKind>,
}

/// A table of a file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceTable {
    pub name: String,
    /// `features` or `attributes`.
    pub data: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u64,
    /// The geometry column's type, none for an attribute table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    pub columns: Vec<SourceColumn>,
    pub capabilities: SourceCapabilities,
}

/// A bound layer's state: whether its file is there, whether the file changed since the layer last matched it, and
/// how the layer differs from it now (objects the layer has and the table lacks, both have but differ, the table has
/// and the layer lacks).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceState {
    pub link: SourceLink,
    pub found: bool,
    /// The file changed since the layer last matched it.
    pub changed: bool,
    /// The layer changed since it last matched the file.
    pub edited: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub modified: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
}

/// Input of `cad.sources.describe` v1: a file's or a PostGIS database's tables (`path`), or the project's bound layers'
/// state (none). Refusals: `invalid_file_path`, `file_not_found`, `import_unreadable`, `invalid_address`,
/// `source_unreachable` (docs/adr/0505).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesDescribe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
}

/// Output of `cad.sources.describe` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesDescribed {
    /// The file's tables (with `path`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SourceTable>>", optional))]
    pub tables: Vec<SourceTable>,
    /// The project's bound layers (without `path`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SourceState>>", optional))]
    pub links: Vec<SourceState>,
    pub revision: String,
}

/// An edit of the project's bound layers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SourcesEditOp {
    /// Binds a table: onto a new layer with its rows (one undo step, as `file.import.gpkg` takes it), or onto the
    /// `layer` given, which must hold the table's objects as they are. A table in another coordinate system is bound
    /// only with `reproject` (docs/adr/0504): the layer holds its objects in the project's system, the table keeps
    /// its own, and every sync moves them across.
    Link {
        path: String,
        table: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        layer: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        reproject: bool,
        /// Onto a new layer from a file, only the rows whose box meets this area (minimum east, north, maximum east,
        /// north, in the project's system; docs/adr/0510): the working copy of a part of a large table.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        extent: Option<[f64; 4]>,
    },
    /// Unbinds a layer; its objects stay.
    Unlink { id: String },
    /// The file moved: binds the layer to the file at `path`, which must be the same file (the revision's).
    Relink { id: String, path: String },
}

/// Input of `cad.sources.edit` v1. Refusals: `invalid_file_path`, `file_not_found`, `import_unreadable`,
/// `table_not_found`, `layer_not_found`, `not_a_layer`, `layer_locked`, `crs_mismatch`, `source_bound` (the layer or
/// the table is bound already), `source_differs` (the layer given does not hold the table's objects),
/// `source_not_found` (an id), `source_changed` (relink to another file), `too_many_sources`, `invalid_extent` (an area
/// that is not one, on a table without geometry, onto a layer given, or of a database).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesEdit {
    pub edit: SourcesEditOp,
}

/// Output of `cad.sources.edit` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesEdited {
    /// The binding as it is now; none after unlinking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub link: Option<SourceLink>,
    /// The objects taken in, and the layers made (linking onto a new layer).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ImportedLayer>>", optional))]
    pub layers: Vec<ImportedLayer>,
    pub revision: String,
}

/// Which way a sync goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SyncDirection {
    /// The layer's changes into its table.
    Write,
    /// The table's rows into the layer.
    Read,
}

/// How a write takes the table's rows outside a part's area (docs/adr/0531): those sharing a corner or an edge the
/// written objects changed follow them, in the same transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SyncTopology {
    /// All or nothing: a row that would be left invalid refuses the whole write (`outside_neighbour_invalid`).
    Strict,
    /// The rows that can follow do; one that would be left invalid keeps its shape and is said (`neighbours_left`).
    Loose,
}

/// Input of `cad.sources.sync` v1. Refusals: `source_not_found` (the layer is not bound), `layer_not_found`,
/// `source_gone` (the file is gone), `source_readonly`, `geometry_mismatch`, `file_unwritable`; conflicts:
/// `source_changed` (writing, and the file changed since the layer last matched it), `pending_changes` (reading, and
/// the layer differs from the table, without `discard`). Warnings: `export_losses` (objects a table has no geometry
/// for stay in the drawing only).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesSync {
    /// The bound layer, by id.
    pub layer: String,
    pub direction: SyncDirection,
    /// Reading: take the table's rows though the layer differs (one undo step brings the layer back).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub discard: bool,
    /// Writing a part's working copy (docs/adr/0531): the table's rows outside the area that share the changed corners
    /// and edges follow in the same transaction. Reading refuses it (`topology_on_read`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub topology: Option<SyncTopology>,
}

/// Output of `cad.sources.sync` v1, and its plan: what was (or would be) added, changed and removed on the other side,
/// the columns a write added to the table, and the file's revision after.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesSynced {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub modified: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub columns: Vec<String>,
    /// The file's SHA-256 after (plan: before).
    pub source: String,
    pub revision: String,
    /// The rows outside the part's area that followed the written objects' shared corners and edges (docs/adr/0531).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub followed: u32,
    /// Those that would have been left invalid and kept their shape (`loose`).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}
