//! A CSV table of objects in and out (docs/adr/0522, plan/gis.md CBS-05): the geometry as ISO WKT in its own column
//! (arcs as circular strings, exact) or as X and Y columns, the attributes in the others; the column types in a GDAL
//! `.csvt` beside it, the system in a `.prj`. QGIS's delimited text and GDAL's CSV driver read and write the same.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The drawing's layers as one CSV.
pub const FILE_EXPORT_CSV: &str = "file.export.csv";
pub const FILE_EXPORT_CSV_VERSION: u32 = 1;
/// A CSV's rows taken into the drawing.
pub const FILE_IMPORT_CSV: &str = "file.import.csv";
pub const FILE_IMPORT_CSV_VERSION: u32 = 1;

/// Where a CSV's geometry is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CsvGeometry {
    /// ISO WKT in a column named `WKT`, first (GDAL's `GEOMETRY=AS_WKT`).
    #[default]
    Wkt,
    /// Points as `X` (east) and `Y` (north) columns, and `Z` when a point has one (GDAL's `GEOMETRY=AS_XY`);
    /// objects that are no points are not written, and counted.
    Xy,
    /// No geometry: the attributes alone.
    None,
}

/// The CSV's field separator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CsvSeparator {
    #[default]
    Comma,
    Semicolon,
    Tab,
}

/// Input of `file.export.csv` v1: the layers' objects as one CSV at `path` (UTF-8, CRLF lines, RFC 4180 quoting),
/// a `.csvt` beside it naming each column's GDAL type and a `.prj` naming the project's system. With more than one
/// layer a `katman` column names each row's. Written whole or not at all. Refusals, in order: `invalid_file_path`,
/// `file_exists`, `layer_not_found`, `nothing_to_export`, `file_unwritable`. Warnings: `export_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportCsv {
    /// An absolute path ending `.csv`.
    pub path: String,
    /// The layers, by id (a group gives its layers); empty: every layer with objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    #[serde(default)]
    pub geometry: CsvGeometry,
    #[serde(default)]
    pub separator: CsvSeparator,
    /// Write over the CSV and its `.csvt` and `.prj` when they are there.
    #[serde(default)]
    pub overwrite: bool,
    /// Only the objects of a valid time and a day known (docs/adr/0646 §2), by `cad.time.query`'s rules: of each
    /// written layer with time settings (of every one when `fields` names them); a layer without them whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub time: Option<crate::temporal::QueryTime>,
}

/// A file written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CsvFileWritten {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}

/// What `file.export.csv` wrote (or with `plan` would write).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CsvExported {
    pub path: String,
    /// Rows written (the header aside).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    /// The columns, in order, and their GDAL types as the `.csvt` names them.
    pub columns: Vec<String>,
    pub types: Vec<String>,
    /// The CSV, its `.csvt` and, when the project has a system, its `.prj`.
    pub files: Vec<CsvFileWritten>,
    /// Objects not written, by kind (a text, a dimension …; with X/Y every object that is no point).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: BTreeMap<String, u32>,
    /// What the CSV could not hold, a sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub lost: Vec<String>,
}

/// Where an imported CSV's geometry is.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", tag = "kind")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CsvGeometryIn {
    /// A column named `WKT`, `geom`, `geometry`, `the_geom` or `wkt_geom` (any case) holds WKT; else `X` and `Y`
    /// (`x`/`y`, `lon`/`lat`, `longitude`/`latitude`, `easting`/`northing`) hold a point, `Z` its elevation; else
    /// none.
    #[default]
    Auto,
    /// The column holding WKT.
    Wkt { column: String },
    /// The columns holding east, north and, when given, the elevation.
    Xy {
        x: String,
        y: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        z: Option<String>,
    },
    /// No geometry: every row a record.
    None,
}

/// Input of `file.import.csv` v1: the CSV at `path` taken into the drawing as one undo step “CSV: ad.csv”, each row
/// an object (or a record without a geometry), its other columns its attributes; a `.csvt` beside it gives the columns'
/// types (else they are worked out: integer, decimal, date, text), a `.prj` its system (else `srid`, else none). The
/// rows go onto `layer` when given, else onto the drawing's layer named as the file, else onto a new layer with the
/// columns as its fields. Refusals as `file.import.gpkg`'s: `invalid_file_path`, `file_not_found`, `import_unreadable`
/// (also a column the geometry names that is not there), `layer_not_found`, `not_a_layer`, `locked_layer`,
/// `crs_mismatch`, `entity_exists`, `nothing_to_import`. Warnings: `crs_undeclared`, `import_losses` (rows whose
/// geometry is not read, with their line numbers).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportCsv {
    /// An absolute path to a `.csv` (or `.txt`).
    pub path: String,
    /// Where the geometry is; none: `auto`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<CsvGeometryIn>,
    /// The layer the rows go onto, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The system the coordinates are in when no `.prj` says (EPSG).
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
