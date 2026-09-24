//! File exchange (CLAUDE.md §9.7, §14): what the format readers and writers
//! of `crates/formats` take and give. The browser runs them in a Web Worker
//! (the `kentos-formats-wasm` module); the server's import job will run the
//! same code natively. Coordinates are float64 exactly as the file wrote
//! them: nothing here rounds, rescales or reprojects (§5, §23).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::document::Bounds;
use crate::entity::{Entity, Vec2};
use crate::layer::LineType;

/// Version of this boundary; the WASM module reports the one it was built with.
pub const FORMATS_VERSION: u32 = 1;

// ── Every import ────────────────────────────────────────────────────────

/// A layer as the source file defines it. Objects name it in `layerId`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportLayer {
    pub name: String,
    /// Hex colour or a theme token (DXF colour 7 → "ink").
    pub color: String,
    pub visible: bool,
    pub locked: bool,
    pub line_type: LineType,
    /// Plot line weight in mm, when the file gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub line_weight: Option<f64>,
    /// Objects read onto this layer.
    pub count: u32,
}

/// One line of an import or export report: what, how many, and what happened to it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportItem {
    /// The source's name for it ("IMAGE", "Genişlikli çoklu çizgi").
    pub what: String,
    pub count: u32,
    /// What happened and, where it helps, what to do (Turkish, for the user).
    pub reason: String,
    /// The first source lines where it occurs.
    pub lines: Vec<u32>,
}

/// A fact about the source file, shown before the import ("Sürüm": "AutoCAD 2000").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SourceFact {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImportReport {
    /// Objects read, by kind (`point`, `line`, …).
    pub counts: BTreeMap<String, u32>,
    /// Not imported, with the reason.
    pub skipped: Vec<ReportItem>,
    /// Imported with a change the user should know about (a conversion, an approximation).
    pub notes: Vec<ReportItem>,
    pub source: Vec<SourceFact>,
}

/// What a reader produced. Objects have id 0 (the app numbers them when it
/// adds them) and `layerId` holds the name of their layer in `layers`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImportResult {
    pub entities: Vec<Entity>,
    pub layers: Vec<ImportLayer>,
    pub report: ImportReport,
    /// Extent of the objects' defining points (shown before the import).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bounds: Option<Bounds>,
}

// ── Coordinate lists (Netcad NCN, TXT, CSV) ─────────────────────────────

/// What a column of a coordinate list holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum CoordColumn {
    /// Point name (becomes the label and the `Ad` attribute).
    Name,
    /// Y, to the right (east): the model's x.
    Y,
    /// X, up (north): the model's y.
    X,
    /// Elevation.
    Z,
    /// Point code or description (the `Kod` attribute).
    Code,
    /// Not read.
    Skip,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum CoordDelimiter {
    #[default]
    Auto,
    Tab,
    Semicolon,
    Comma,
    /// One or more spaces (Netcad NCN).
    Space,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum DecimalMark {
    #[default]
    Auto,
    Point,
    /// A decimal comma (Turkish spreadsheets); only with a delimiter other than the comma.
    Comma,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum HeaderMode {
    #[default]
    Auto,
    Yes,
    No,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CoordReadOptions {
    pub delimiter: CoordDelimiter,
    pub decimal: DecimalMark,
    pub header: HeaderMode,
    /// What each column holds, in file order; empty lets the reader suggest.
    pub columns: Vec<CoordColumn>,
    /// Rows for the preview table.
    pub preview_rows: u32,
    /// Whether to build the objects (the dialog asks for them only when importing).
    pub entities: bool,
}

/// A row of the preview: the fields as written and, if the row is not a point, why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoordRow {
    pub line: u32,
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LineError {
    pub line: u32,
    pub message: String,
}

/// A coordinate list read with the given options, and what the reader found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CoordRead {
    /// Text encoding of the file ("UTF-8", "Windows-1254 (Türkçe)").
    pub encoding: String,
    /// The delimiter and decimal mark in use (never `auto`).
    pub delimiter: CoordDelimiter,
    pub decimal: DecimalMark,
    /// Whether the first data line is a header, and its fields.
    pub header: bool,
    pub header_fields: Vec<String>,
    /// What each column holds, in use (as given, or suggested).
    pub columns: Vec<CoordColumn>,
    pub preview: Vec<CoordRow>,
    /// Lines that are neither blank nor comments (header included).
    pub data_lines: u32,
    pub points: u32,
    /// The first rows that are not points, with the reason.
    pub errors: Vec<LineError>,
    pub error_count: u32,
    /// Points whose name another point already has.
    pub duplicate_names: u32,
    /// Things worth checking before the import (Turkish).
    pub hints: Vec<String>,
    /// Extent of the points (Y as x, X as y), to show where the data lies before the import.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bounds: Option<Bounds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub result: Option<ImportResult>,
}

/// A point to write into a coordinate list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoordPoint {
    pub name: String,
    pub p: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub z: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub code: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum TextEncoding {
    #[default]
    Utf8,
    /// Windows-1254 (Turkish); other characters are left out and reported.
    Windows1254,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CoordWriteInput {
    pub points: Vec<CoordPoint>,
    /// Not `auto`.
    pub delimiter: CoordDelimiter,
    /// Column order (`skip` is not written).
    pub columns: Vec<CoordColumn>,
    /// A first line with the column names.
    pub header: bool,
    pub encoding: TextEncoding,
}

/// What a writer did besides writing: counts, and anything it could not write as it was.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportReport {
    pub counts: BTreeMap<String, u32>,
    pub notes: Vec<ReportItem>,
    pub skipped: Vec<ReportItem>,
}
