//! File exchange (CLAUDE.md §9.7, §14): what the format readers and writers
//! of `crates/shared/formats` take and give. The browser runs them in a Web Worker
//! (the `kentos-formats-wasm` module); the server's import job will run the
//! same code natively. Coordinates are float64 exactly as the file wrote
//! them: nothing here rounds, rescales or reprojects (§5, §23).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::document::Bounds;
use crate::entity::{BlockDefinition, Entity, Vec2};
use crate::layer::LineType;

/// Version of this boundary; the WASM module reports the one it was built with.
/// 2: `writeDxf` (`DxfWriteInput`), and the reader takes back KentOS's DXF data.
/// 3: dimensions are written as DXF dimensions (`dimensionValues`) and read back.
/// 4: `v1Identities`, the persistent ids of a v1 drawing's objects (`identity`, docs/adr/0014).
/// 5: `encodeKcad`, `decodeKcad`: the binary `.kcad` v2 (docs/specs/kcad-v2.md, docs/adr/0025).
/// 6: the drawing crosses as typed columns (`kentos_kcad::columns`) with progress, not as JSON (docs/adr/0030).
/// 7: GeoJSON read and write, Shapefile read (`readGeoJson`, `writeGeoJson`, `readShapefile`);
///    an import says the coordinate system its file declares (`declaredCrs`, docs/adr/0046).
/// 8: Netcad NCZ (`NczReadOptions`, `CrsSource::Ncz`, docs/adr/0138); DXF and NCZ read in modules
///    of their own, loaded when such a file is imported, reporting their progress, their objects
///    crossing as typed columns; a layer's objects by kind and their box (`ImportLayer.kinds`,
///    `ImportLayer.bounds`) and where the view shows an import (`ImportResult.view`).
/// 9: an object's own line weight (`EntityBase.line_weight`, docs/adr/0139): DXF's group 370 and
///    an NCZ's pen read into it, written back as 370.
/// 10: vertex elevations (docs/adr/0142): `.kcad` document schema 4 and the typed columns' layout
///    (`kentos_kcad::columns`): a line's `za` and `zb`, a path's and a hole's `zs`, NaN in the
///    columns for a vertex without an elevation.
/// 11: multi-part areas (docs/adr/0143): `.kcad` document schema 5 and the typed columns' layout,
///    a polygon's `parts` after its holes, each part's flags, ring, bulges, elevations and holes.
/// 12: blocks (docs/adr/0144): `.kcad` document schema 6, the definitions in the drawing's JSON
///    and the `insert` kind in the typed columns (kind 13: p, scale, rotation, the block's id as
///    text, mirror a flag).
/// 13: DXF blocks kept (docs/adr/0144 §5): `ImportResult.blocks`, the definitions an import's
///    inserts place, and `DxfReadOptions.explode_blocks` (Blokları patlat).
/// 14: DXF blocks written (docs/adr/0144 §5): `DxfWriteInput.blocks`, and the writers' objects
///    take inserts.
/// 15: GeoJSON writes an insert as its block's objects placed (docs/adr/0144 §5):
///    `GeoJsonWriteInput.blocks`.
/// 16: a text's alignment, width factor and mask (docs/adr/0145): `.kcad` document schema 7,
///    the text's optional fields in the typed columns (align an int, width factor a float, mask
///    a flag) and an attribute definition's alignment and width factor in the drawing's JSON.
/// 17: the leader (docs/adr/0146): `.kcad` document schema 8, a new kind in the typed columns
///    (its vertices, height and turn; the note a text, the arrowhead an int, the mask a flag).
/// 18: the new dimensions (docs/adr/0147): `.kcad` document schema 9, five more dimension styles
///    in the typed columns' numbering and the dimension's mask (a flag), `za` and `zb` (floats).
pub const FORMATS_VERSION: u32 = 18;

// ── Every import ────────────────────────────────────────────────────────

/// A layer as the source file defines it. Objects name it in `layerId`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportLayer {
    pub name: String,
    /// Hex colour or a theme token (DXF colour 7 → "ink").
    pub color: String,
    pub visible: bool,
    pub locked: bool,
    pub line_type: LineType,
    /// Plot line weight in mm, when the file gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_weight: Option<f64>,
    /// Objects read onto this layer.
    pub count: u32,
    /// Those objects by kind (`point`, `line`, …): what a choice of layers
    /// brings, counted without walking the objects (docs/adr/0138).
    #[serde(default)]
    pub kinds: BTreeMap<String, u32>,
    /// The box those objects' defining points span (a text's is its
    /// insertion point): where a choice of layers is, without walking them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bounds: Option<Bounds>,
}

/// One line of an import or export report: what, how many, and what happened to it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceFact {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportReport {
    /// Objects read, by kind (`point`, `line`, …).
    pub counts: BTreeMap<String, u32>,
    /// Not imported, with the reason.
    pub skipped: Vec<ReportItem>,
    /// Imported with a change the user should know about (a conversion, an approximation).
    pub notes: Vec<ReportItem>,
    pub source: Vec<SourceFact>,
}

/// Where a file's statement of its coordinate system comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CrsSource {
    /// A GeoJSON file without a `crs` member: RFC 7946 makes it WGS 84 longitude, latitude.
    Rfc7946,
    /// A GeoJSON file's legacy `crs` member (the 2008 specification).
    GeoJsonCrs,
    /// A Shapefile's `.prj` (WKT).
    Prj,
    /// A Netcad NCZ drawing's MPROJ block (datum, projection, zone) and the
    /// SRS its TILED_XML block names (docs/adr/0138).
    Ncz,
}

/// The coordinate system a file declares: what the file says, never a
/// guess (CLAUDE.md §5). The import window shows it as the answer to its
/// question, and the user confirms or changes it; nothing is transformed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DeclaredCrs {
    /// The EPSG code the statement names, when it names one the reader recognises.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub srid: Option<u32>,
    /// The statement as the window shows it ("urn:ogc:def:crs:EPSG::5256", "TUREF_TM36").
    pub text: String,
    pub source: CrsSource,
}

/// What a reader produced. Objects have id 0 (the app numbers them when it
/// adds them) and `layerId` holds the name of their layer in `layers`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportResult {
    pub entities: Vec<Entity>,
    pub layers: Vec<ImportLayer>,
    pub report: ImportReport,
    /// Extent of the objects' defining points (shown before the import).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bounds: Option<Bounds>,
    /// The coordinate system the file declares (GeoJSON, Shapefile; docs/adr/0046).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub declared_crs: Option<DeclaredCrs>,
    /// Where the view shows the objects once they are in: their extent without
    /// the far strays a file can hold (a slip drawn at 0, 0 beside a city,
    /// `kentos_formats::import::view_bounds`; docs/adr/0138).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view: Option<Bounds>,
    /// The block definitions the objects' inserts place (a DXF's BLOCKs,
    /// docs/adr/0144 §5), in the file's order: ids the reader numbered (the
    /// app gives them new ones and a name the drawing does not have yet),
    /// their objects on layer "0". None when blocks were exploded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockDefinition>>", optional))]
    pub blocks: Vec<BlockDefinition>,
}

// ── Coordinate lists (Netcad NCN, TXT, CSV) ─────────────────────────────

/// What a column of a coordinate list holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CoordDelimiter {
    #[default]
    Auto,
    Tab,
    Semicolon,
    Comma,
    /// One or more spaces (Netcad NCN).
    Space,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DecimalMark {
    #[default]
    Auto,
    Point,
    /// A decimal comma (Turkish spreadsheets); only with a delimiter other than the comma.
    Comma,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HeaderMode {
    #[default]
    Auto,
    Yes,
    No,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CoordRow {
    pub line: u32,
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineError {
    pub line: u32,
    pub message: String,
}

/// A coordinate list read with the given options, and what the reader found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
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
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bounds: Option<Bounds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub result: Option<ImportResult>,
}

/// A point to write into a coordinate list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CoordPoint {
    pub name: String,
    pub p: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub code: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TextEncoding {
    #[default]
    Utf8,
    /// Windows-1254 (Turkish); other characters are left out and reported.
    Windows1254,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
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

// ── DXF ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DxfReadOptions {
    /// Stop after this many objects (0: one million); the rest is counted and reported.
    pub max_entities: u32,
    /// Blokları patlat: every insert opened into its objects, no definitions
    /// kept (docs/adr/0144 §5). Off, a block is a definition and an insert
    /// places it; the inserts a block cannot hold are opened all the same.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub explode_blocks: bool,
}

/// A layer as the DXF writer receives it. DXF layers are flat and their
/// names ignore case: the writer makes each name valid and unique (the
/// group path tells layers of the same name apart) and reports what it changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DxfWriteLayer {
    /// What the objects' `layerId` holds.
    pub id: String,
    pub name: String,
    /// Names of the groups above the layer, outermost first.
    pub path: Vec<String>,
    /// Hex colour or a theme token ("ink" is DXF colour 7).
    pub color: String,
    /// As the drawing shows it (inherited from the groups): a hidden layer is written off.
    pub visible: bool,
    pub locked: bool,
    pub line_type: LineType,
    /// Plot line weight in mm (written as AutoCAD's nearest weight).
    pub line_weight: f64,
}

/// What `file.export.dxf` writes (an AutoCAD 2007 DXF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DxfWriteInput {
    pub entities: Vec<Entity>,
    pub layers: Vec<DxfWriteLayer>,
    /// Plot scale denominator (1000 for 1:1000): line type patterns and point marks are sized for paper at it.
    pub scale: f64,
    /// Decimals the project shows lengths with ($LUPREC).
    pub length_decimals: u32,
    /// Angles are shown in grads, else in degrees ($AUNITS).
    pub grads: bool,
    /// What each dimension without a text of its own shows, by object id:
    /// the measured value as the app formats it (project units). The
    /// dimension's block draws it; the DXF dimension keeps no text, so a
    /// program that redraws it measures again.
    #[serde(default)]
    pub dimension_values: BTreeMap<u32, String>,
    /// The drawing's block definitions (docs/adr/0144 §5): those the
    /// objects' inserts place, and those nested in them, are written as
    /// BLOCKs, inner ones first; the others are left out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockDefinition>>", optional))]
    pub blocks: Vec<BlockDefinition>,
}

/// What a writer did besides writing: counts, and anything it could not write as it was.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportReport {
    pub counts: BTreeMap<String, u32>,
    pub notes: Vec<ReportItem>,
    pub skipped: Vec<ReportItem>,
}

// ── Netcad NCZ (docs/adr/0138) ──────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NczReadOptions {
    /// Stop after this many objects (0: one million); the rest is counted and reported.
    pub max_entities: u32,
    /// The drawing's typeface (`ProjectSettings.drawingFont`; empty: the
    /// default). A smart object's centred text is placed by its width in the
    /// face the drawing shows it in: the app's texts sit on their lower left.
    #[serde(default)]
    pub drawing_font: String,
}

// ── GeoJSON (RFC 7946) and Shapefile (docs/adr/0046) ───────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeoJsonReadOptions {
    /// The layer of the objects the file does not place: the document's
    /// `name` member when it has one, else this (the file's name).
    pub layer: String,
    /// Stop after this many objects (0: one million); the rest is counted and reported.
    pub max_entities: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShapefileReadOptions {
    /// The layer of the objects (the .shp file's name).
    pub layer: String,
    /// Stop after this many objects (0: one million); the rest is counted and reported.
    pub max_entities: u32,
}

/// A layer the GeoJSON writer names in each feature's `kentos.layer`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeoJsonLayer {
    /// What the objects' `layerId` holds.
    pub id: String,
    pub name: String,
}

/// What `file.export.geojson` writes: a FeatureCollection. A WGS 84
/// project (EPSG:4326) is written as RFC 7946 has it; any other in its own
/// coordinate system with the legacy `crs` member naming it, which RFC
/// 7946 does not allow (the export window says so). Nothing is transformed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeoJsonWriteInput {
    pub entities: Vec<Entity>,
    pub layers: Vec<GeoJsonLayer>,
    /// The project's coordinate system (EPSG code).
    pub srid: u32,
    /// The collection's `name` (the drawing's).
    pub name: String,
    /// The drawing's block definitions (docs/adr/0144 §5): an insert is
    /// written as its block's objects placed, one GeometryCollection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockDefinition>>", optional))]
    pub blocks: Vec<BlockDefinition>,
}
