//! FlatGeobuf in and out (docs/adr/0544, plan/gis.md CBS-05): one layer's features in one file with the packed Hilbert
//! R-tree of their boxes, as GDAL, QGIS and the format's own libraries write and read it. Arcs go as circular strings
//! and compound curves, exact; the system as its EPSG code.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The drawing's layers as one FlatGeobuf.
pub const FILE_EXPORT_FGB: &str = "file.export.fgb";
pub const FILE_EXPORT_FGB_VERSION: u32 = 1;
/// A FlatGeobuf's features taken into the drawing.
pub const FILE_IMPORT_FGB: &str = "file.import.fgb";
pub const FILE_IMPORT_FGB_VERSION: u32 = 1;

/// Input of `file.export.fgb` v1: the layers' objects as one FlatGeobuf at `path`, the layers' fields as its columns
/// (with more than one layer a `katman` column names each feature's), the project's system as its EPSG code, the
/// spatial index written unless a record (an object without a position) is among them. Written whole or not at all.
/// Refusals, in order: `invalid_file_path`, `file_exists`, `layer_not_found`, `nothing_to_export`, `file_unwritable`.
/// Warnings: `export_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportFgb {
    /// An absolute path ending `.fgb`.
    pub path: String,
    /// The layers, by id (a group gives its layers); empty: every layer with objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// Write over the file when it is there.
    #[serde(default)]
    pub overwrite: bool,
}

/// What `file.export.fgb` wrote (or with `plan` would write).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FgbExported {
    pub path: String,
    /// The layer's name in the file (the file's name without `.fgb`).
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub features: u32,
    /// The columns, in order, and their types as GDAL names them.
    pub columns: Vec<String>,
    pub types: Vec<String>,
    /// The layer's geometry type as GDAL names it (`Unknown` for mixed types).
    pub geometry: String,
    /// Whether the spatial index is written.
    pub indexed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// Objects not written, by kind (a text, a dimension …).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: BTreeMap<String, u32>,
    /// What the file could not hold, a sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub lost: Vec<String>,
}

/// Input of `file.import.fgb` v1: the FlatGeobuf at `path` taken into the drawing as one undo step “FlatGeobuf:
/// ad.fgb”, each feature an object (or a record without a geometry), its properties its attributes. The features go
/// onto `layer` when given, else onto the drawing's layer named as the file's layer, else onto a new layer with the
/// columns as its fields. A date-time column whose values are all days is taken as a date field. Refusals as
/// `file.import.gpkg`'s: `invalid_file_path`, `file_not_found`, `import_unreadable`, `layer_not_found`, `not_a_layer`,
/// `locked_layer`, `crs_mismatch`, `entity_exists`, `nothing_to_import`. Warnings: `crs_undeclared`, `import_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportFgb {
    /// An absolute path to a `.fgb`.
    pub path: String,
    /// The layer the features go onto, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The system the coordinates are in when the file names none (EPSG).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// Move coordinates in another system into the project's.
    #[serde(default)]
    pub reproject: bool,
    /// Rows meeting the objects already on the layer they go onto: updated, added or refused by a key (docs/adr/0628
    /// §5); none: every row is added.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub merge: Option<crate::ImportMerge>,
    /// Where the rows refused go, a CSV with their reasons (docs/adr/0628 §6; `execute` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rejects: Option<String>,
}
