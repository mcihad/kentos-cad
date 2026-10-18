//! GeoPackage in and out (docs/adr/0502, plan/gis.md CBS-03): a file's feature and attribute tables taken into the
//! drawing as layers of objects and records with their fields, as one undo step named after the file; and the
//! drawing's layers written as a GeoPackage's tables, whole or not at all. Each row keeps a lasting id: the file's
//! (`kentos_uids`, written by KentOS) or one derived from its table and fid, the same every time.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::file_import::ImportedLayer;

/// A GeoPackage into the drawing.
pub const FILE_IMPORT_GPKG: &str = "file.import.gpkg";
pub const FILE_IMPORT_GPKG_VERSION: u32 = 1;
/// The drawing's layers as a GeoPackage.
pub const FILE_EXPORT_GPKG: &str = "file.export.gpkg";
pub const FILE_EXPORT_GPKG_VERSION: u32 = 1;

/// Input of `file.import.gpkg` v1. Refusals (`CommandError.code`), in order: `invalid_file_path`, `file_not_found`,
/// `import_unreadable` (not a GeoPackage, or broken), `table_not_found` (a table of `tables`), `layer_not_found` and
/// `not_a_layer` (a target of `layers`), `layer_locked`, `crs_mismatch` (the file's system is not the project's and
/// `reproject` was not asked), `entity_exists` (a row's lasting id is an object of the drawing already),
/// `nothing_to_import`. Warnings: `crs_undeclared`, `import_losses`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportGpkg {
    /// The file to read, an absolute path.
    pub path: String,
    /// The tables to take; empty: every feature and attribute table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub tables: Vec<String>,
    /// A table → the drawing's layer (its id) its rows go onto. A table not named here goes onto the drawing's layer
    /// of the same name (case and Turkish marks aside), else onto a new layer with the table's fields.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub layers: BTreeMap<String, String>,
    /// Move the coordinates from the file's system into the project's (the project's datum choices taken); without
    /// it a file in another system is refused.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reproject: bool,
}

/// A table taken.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GpkgTableImported {
    pub table: String,
    /// The drawing's layer its rows went onto.
    pub layer: String,
    /// Rows read, objects taken (records among them), rows left out.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub records: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
}

/// What `file.import.gpkg` took in, or with `plan` would take.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GpkgImported {
    /// The file's name, its bytes and their SHA-256.
    pub file: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// The EPSG code the tables declare, none for an undefined system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// Whether the coordinates were moved into the project's system.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub moved: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    pub tables: Vec<GpkgTableImported>,
    /// The layers made, in the tree's order, and the group they went into.
    pub layers: Vec<ImportedLayer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// The drawing's revision after the step (plan: before it).
    pub revision: String,
}

/// Input of `file.export.gpkg` v1: the layers (all with objects when none) as tables of a new GeoPackage, each object
/// a row with its lasting id (`kentos_uids`), its layer's fields and its other attributes as columns; the project's
/// coordinate system as the tables'. Refusals, in order: `invalid_file_path`, `file_exists`, `layer_not_found`,
/// `nothing_to_export`, `file_unwritable`. Warnings: `export_losses` (objects a GeoPackage has no geometry for, curves
/// written as chords).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportGpkg {
    /// The file to write, an absolute path (`.gpkg`).
    pub path: String,
    /// The layers, by id; empty: every layer with objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// Write over a file that is there.
    #[serde(default)]
    pub overwrite: bool,
}

/// A table written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GpkgTableExported {
    pub table: String,
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    /// Its geometry column's type (`MULTIPOLYGON` …); none for an attribute table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<String>,
}

/// What `file.export.gpkg` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GpkgExported {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    pub tables: Vec<GpkgTableExported>,
    /// Objects not written, by kind.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: BTreeMap<String, u32>,
    /// Curves (ellipses, splines, NURBS) written as chords within 0.1 mm.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub approximated: u32,
}
