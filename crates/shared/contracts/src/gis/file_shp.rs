//! Shapefile out (docs/adr/0514, plan/gis.md CBS-05): the drawing's layers as ESRI Shapefiles, one a geometry family
//! of each layer (a Shapefile holds one type), written whole or not at all; what the format cannot hold is written as
//! the nearest thing it can and said (`export_losses`), never dropped silently.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ExportReport;

/// The drawing's layers as Shapefiles.
pub const FILE_EXPORT_SHP: &str = "file.export.shp";
pub const FILE_EXPORT_SHP_VERSION: u32 = 1;

/// The .dbf's text code page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ShpEncoding {
    /// UTF-8, named in the .cpg (QGIS and GDAL write it so today).
    #[default]
    #[serde(rename = "utf-8")]
    Utf8,
    /// Windows-1254 (Turkish Windows), named in the .cpg and the language driver byte, for programs that read only
    /// the old code pages; a character it lacks is written `?` and counted.
    #[serde(rename = "windows-1254")]
    Windows1254,
}

/// Input of `file.export.shp` v1. `path` is where: a `.zip` (one archive with every Shapefile), a `.shp` (the one
/// Shapefile the export makes, its parts beside it) or a folder, new or empty (every Shapefile in it, named after its
/// layer). Refusals, in order: `invalid_file_path`, `file_exists` (the archive, a part of the `.shp`, or a folder that
/// is not empty), `layer_not_found`, `nothing_to_export`, `several_shapefiles` (a `.shp` path for an export of more than
/// one Shapefile), `file_unwritable`. Warnings: `export_losses`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportShp {
    /// An absolute path: `.zip`, `.shp`, or a folder.
    pub path: String,
    /// The layers, by id (a group gives its layers); empty: every layer with objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// The .dbf's code page; UTF-8 when not given.
    #[serde(default)]
    pub encoding: ShpEncoding,
    /// Write over the archive, or the `.shp`'s parts, that are there (a folder is never written over).
    #[serde(default)]
    pub overwrite: bool,
    /// Only the objects of a valid time and a day known (docs/adr/0646 §2), by `cad.time.query`'s rules: of each
    /// written layer with time settings (of every one when `fields` names them); a layer without them whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub time: Option<crate::temporal::QueryTime>,
}

/// A file written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShpFileWritten {
    /// Its name (in the archive, or in the folder).
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}

/// A Shapefile written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShpSetExported {
    /// Its name without extension: the layer's, with `_nokta`, `_cizgi` or `_alan` when the layer has several
    /// geometry families.
    pub name: String,
    /// The layer it came from, by id.
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    /// Its shape type (`POLYGONZ` …); none for a table alone (a layer of records without a geometry).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<String>,
}

/// A column whose name is not its attribute's (dBASE: ten bytes, one whatever the case).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShpColumnRenamed {
    pub set: String,
    pub attribute: String,
    pub column: String,
}

/// A field written as text, and why (a date-time, a number past dBASE's digits, values its rule refuses).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShpColumnAsText {
    pub set: String,
    pub column: String,
    pub reason: String,
}

/// What `file.export.shp` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShpExported {
    pub path: String,
    /// `zip`, `shp` or `folder`.
    pub into: String,
    pub sets: Vec<ShpSetExported>,
    /// The files, in the order written (an archive: the archive itself).
    pub files: Vec<ShpFileWritten>,
    /// Whether a .prj names the coordinate system (none for a local project, or a system the writer cannot name).
    pub prj: bool,
    /// Objects not written, by kind.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: BTreeMap<String, u32>,
    /// Objects with curves written as chords within 0.1 mm.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub approximated: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ShpColumnRenamed>>", optional))]
    pub renamed: Vec<ShpColumnRenamed>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ShpColumnAsText>>", optional))]
    pub as_text: Vec<ShpColumnAsText>,
    /// Text values cut to the column's 254 bytes; characters the code page lacks, written `?`; empty texts (read
    /// back as no value).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub cut: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub replaced: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub blank_text: u32,
    /// Everything the writer said, by kind and reason.
    pub report: ExportReport,
}
