//! GeoParquet in and out (docs/adr/0628): a Parquet file of rows with a WKB (or GeoArrow) geometry column and the
//! `geo` key of GeoParquet 1.1 saying its encoding, its types, its box and its system (PROJJSON). Read and written by
//! KentOS's own Parquet codec (`kentos_formats::parquet`); the rows go into the drawing by the GeoPackage import's
//! path, with the merge every file import has.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The drawing's layers as one GeoParquet.
pub const FILE_EXPORT_PARQUET: &str = "file.export.parquet";
pub const FILE_EXPORT_PARQUET_VERSION: u32 = 1;
/// A GeoParquet's rows taken into the drawing.
pub const FILE_IMPORT_PARQUET: &str = "file.import.parquet";
pub const FILE_IMPORT_PARQUET_VERSION: u32 = 1;

/// How the pages are compressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ParquetCompression {
    #[default]
    Snappy,
    None,
}

fn yes() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

/// Input of `file.export.parquet` v1: the layers' objects as one GeoParquet 1.1 at `path`: a `geometry` column of ISO
/// WKB (arcs as chords within 0.1 mm, said; M left out, said), the layers' fields as typed columns, the other
/// attributes as text columns, `kentos_uid` the objects' lasting ids, `katman` each object's layer when there are
/// more than one; the `bbox` covering column unless `covering` is false; outer rings counter-clockwise; the project's
/// system as PROJJSON (a local project's none). Written whole or not at all. Refusals, in order: `invalid_file_path`,
/// `file_exists`, `layer_not_found`, `nothing_to_export`, `file_unwritable`. Warnings: `export_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportParquet {
    /// An absolute path ending `.parquet`.
    pub path: String,
    /// The layers, by id (a group gives its layers); empty: every layer with objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// Write over the file when it is there.
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub compression: ParquetCompression,
    /// Whether the `bbox` covering column is written (GeoParquet 1.1).
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub covering: bool,
}

/// What `file.export.parquet` wrote (or with `plan` would write).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParquetExported {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub row_groups: u32,
    /// The columns, in order, and their Parquet types (`BYTE_ARRAY STRING`, `INT64`, `DOUBLE` …).
    pub columns: Vec<String>,
    pub types: Vec<String>,
    /// GeoParquet's `geometry_types`: the types the rows hold (`Polygon`, `Point Z` …).
    pub geometry_types: Vec<String>,
    /// The system's EPSG code, none for a local project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// The rows' box: west, south, east, north; none without a position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub bbox: Option<[f64; 4]>,
    #[serde(default)]
    pub compression: ParquetCompression,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// Objects not written, by kind.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: BTreeMap<String, u32>,
    /// What the file could not hold, a sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub lost: Vec<String>,
}

/// Input of `file.import.parquet` v1: the GeoParquet (or plain Parquet) at `path` taken into the drawing as one undo
/// step “GeoParquet: ad.parquet”, each row an object (a record without a geometry), its columns its attributes. The
/// primary geometry column of the `geo` key is read as WKB, as GeoArrow or, in a file without the key, as Parquet's
/// GEOMETRY type; another geometry column becomes WKT text, said. The system is the column's PROJJSON's id (none:
/// OGC:CRS84; `null`: not known, then `srid`). The rows go onto `layer`, else onto the drawing's layer named as the
/// file, else onto a new layer with the columns as its fields; `merge` meets them with a layer's objects. Refusals as
/// `file.import.gpkg`'s and `duplicate_key`, `merge_key_required`, `merge_key_not_found`, `file_unwritable` (the
/// rejects file). Warnings: `crs_undeclared`, `import_losses`, `rows_rejected`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportParquet {
    /// An absolute path to a `.parquet`.
    pub path: String,
    /// The layer the rows go onto, by id.
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
    /// Rows meeting the objects already on the layer they go onto (docs/adr/0628 §5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub merge: Option<crate::ImportMerge>,
    /// Where the rows refused go, a CSV with their reasons (docs/adr/0628 §6; `execute` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rejects: Option<String>,
}
