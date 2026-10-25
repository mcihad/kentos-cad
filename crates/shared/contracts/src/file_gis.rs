//! The drawing as a traceable GIS dataset (docs/adr/0530, plan/gis.md CBS-16): `file.export.gis` writes the layers as
//! a GeoPackage of derived features, each carrying its source object's lasting id and kind, with the arcs, circles,
//! ellipses and curves as straight chords within a stated tolerance and each feature's deviation recorded; and
//! `cad.gis.derivation` tells whether such a dataset still matches the drawing it was made from (a source changed
//! or gone, a row changed in the GIS). The drawing stays the source: nothing here writes it, and a derivative's
//! table is never bound back (`derived_dataset`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The layers as a derived GIS dataset (GeoPackage).
pub const FILE_EXPORT_GIS: &str = "file.export.gis";
pub const FILE_EXPORT_GIS_VERSION: u32 = 1;
/// Whether a derived dataset matches the drawing.
pub const CAD_GIS_DERIVATION: &str = "cad.gis.derivation";
pub const CAD_GIS_DERIVATION_VERSION: u32 = 1;
/// What a GIS changed in a derived dataset written back to the drawing (docs/adr/0634 §1).
pub const CAD_GIS_TAKEBACK: &str = "cad.gis.takeback";
pub const CAD_GIS_TAKEBACK_VERSION: u32 = 1;
/// The command that breaks a derived table's link to its source (docs/adr/0634 §2).
pub const CAD_GIS_DETACH: &str = "cad.gis.detach";
pub const CAD_GIS_DETACH_VERSION: u32 = 1;
/// The standard of the note a broken link leaves: the record it was, when and why it was broken.
pub const DETACHED_STANDARD: &str = "https://kentos.app/derivation-detached/1";
pub const DETACHED_FORMAT: &str = "kentos.derivation-detached";

/// The URI of KentOS's derivation document in `gpkg_metadata` (docs/adr/0530 §3).
pub const DERIVATION_STANDARD: &str = "https://kentos.app/derivation/1";
/// The `format` of that document.
pub const DERIVATION_FORMAT: &str = "kentos.derivation";
/// The rule the rows' `kentos_digest` is made by: SHA-256 of the canonical JSON of the object without its look.
pub const DERIVATION_DIGEST: &str = "kentos.derivation-digest/1";
/// The tolerance (m) a derivation keeps when none is asked: the drawing's own bound for a computation's chords.
pub const DEFAULT_DERIVATION_TOLERANCE: f64 = 1e-4;
/// The fewest and most entries `cad.gis.derivation` lists, and how many when `limit` is not given.
pub const DERIVATION_LIMIT_DEFAULT: u32 = 100;
pub const DERIVATION_LIMIT_MAX: u32 = 1000;

/// What to do with texts and blocks, which a GIS has no geometry of the same meaning for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GisAnnotations {
    /// Written to their own tables, never into the layers' feature tables: each block definition's insertions as
    /// points with the block's attributes as columns, texts as points with their words.
    #[default]
    Table,
    /// Left out and counted.
    Skip,
}

/// Where a derived dataset is written (docs/adr/0634 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GisFormat {
    /// A GeoPackage (`.gpkg`): each table with its lasting ids and its record in the metadata.
    #[default]
    GeoPackage,
    /// A zip of Shapefiles (`.zip`): each table a set of each geometry family, the records in `kentos-turev.json`.
    Shapefile,
    /// GeoJSON: one table a `.geojson` file, several a `.zip` of them; each its record as `kentos:table`.
    GeoJson,
    /// New tables in a PostGIS database (`path` its address, `schema` theirs): each table's record its comment, its
    /// rows' lasting ids in the schema's `kentos_uids`.
    PostGis,
}

/// What a derived table holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GisTableRole {
    /// A drawing layer's objects.
    Layer,
    /// The insertions of one block, its attributes as columns.
    Blocks,
    /// The drawing's texts.
    Annotations,
    /// The drawing's dimensions (docs/adr/0634 §4): the measured points as a line, the value and its kind as columns;
    /// never a layer's feature.
    Dimensions,
    /// One kind of architectural object (docs/adr/0634 §4): walls, spaces, openings, grids, columns, stairs or ramps,
    /// each its outline (a grid its path) with its parameters as columns.
    Arch,
}

/// Input of `file.export.gis` v1: the layers (every layer with objects when none) as tables of a new derived dataset:
/// a GeoPackage, a zip of Shapefiles, GeoJSON or new tables of a PostGIS database (`format`, docs/adr/0634 §3).
/// Refusals, in order: `invalid_file_path` (also a path whose extension is not the format's), `file_exists`,
/// `invalid_tolerance` (outside 1e-6 to 1 m), `layer_not_found`, `nothing_to_export`, `file_unwritable`; PostGIS:
/// `invalid_address`, `source_unreachable`, `table_exists`. Warnings: `export_losses` (objects left out and the reason
/// each kind is).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportGis {
    /// The file to write, an absolute path (`.gpkg`, `.zip`, `.geojson`), or a PostGIS database's address.
    pub path: String,
    /// The layers, by id (a group gives its layers); empty: every layer with objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// The farthest a chord may come from the curve it stands for, metres; 0.0001 when not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
    /// Texts and blocks: their own tables (the default) or left out.
    #[serde(default)]
    pub annotations: GisAnnotations,
    /// Write over a file that is there (a database's tables of the same names).
    #[serde(default)]
    pub overwrite: bool,
    /// Where it goes: a GeoPackage (the default), a zip of Shapefiles, GeoJSON or a PostGIS database.
    #[serde(default, skip_serializing_if = "is_geopackage")]
    #[cfg_attr(feature = "ts", ts(as = "Option<GisFormat>", optional))]
    pub format: GisFormat,
    /// PostGIS: the schema of the new tables (made when not there); `public` when not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub schema: Option<String>,
}

fn is_geopackage(f: &GisFormat) -> bool {
    *f == GisFormat::GeoPackage
}

/// A table of the derived dataset.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisTableExported {
    pub table: String,
    pub role: GisTableRole,
    /// The drawing layer a `layer` table came from, by id; the block's name for a `blocks` table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    /// The geometry column's type; none for an attribute table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<String>,
    /// Rows whose geometry is chords of a curve rather than the source's own points.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub chorded: u32,
    /// The farthest any of the table's chords comes from its curve (m).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub max_deviation: f64,
}

/// Objects of one kind left out of the dataset, and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisLeft {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    pub reason: String,
}

/// What `file.export.gis` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisExported {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// The algorithm and its version (`kentos.chords/1`), and the tolerance it kept (m).
    pub algorithm: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: f64,
    pub tables: Vec<GisTableExported>,
    /// Objects not written, by kind, each with its reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<GisLeft>>", optional))]
    pub left: Vec<GisLeft>,
    /// Rows written as chords of a curve, in all.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub chorded: u32,
    /// The farthest any chord of the dataset comes from its curve (m); at most `tolerance` unless a count cap held.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub max_deviation: f64,
    /// The drawing's revision the dataset was made from.
    pub revision: String,
}

/// Where a derived dataset came from, as its tables record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DerivationSource {
    /// The project's lasting id, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub project: Option<String>,
    pub name: String,
    /// The drawing's revision when the dataset was made.
    pub revision: String,
    /// The project's EPSG code (0 for a local project).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
}

/// The layer a derived table came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DerivationLayer {
    pub id: String,
    pub name: String,
}

/// A table's derivation document (`kentos.derivation` 1, in `gpkg_metadata` as `DERIVATION_STANDARD`): how its rows
/// were made and from what, so a later reader can tell the geometry is a derivative.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DerivationRecord {
    pub format: String,
    pub version: u32,
    pub table: String,
    pub role: GisTableRole,
    /// The layer of a `layer` table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<DerivationLayer>,
    /// The block of a `blocks` table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<String>,
    /// The objects' kind of an `arch` table (`wall`, `space` …; docs/adr/0634 §4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    /// The layers the export took, by id (a group's leaves): what a `blocks` or `annotations` table's objects were
    /// looked for on, and what a later check counts a missing row against.
    pub layers: Vec<String>,
    pub algorithm: String,
    /// The tolerance the chords were cut for, in `unit`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: f64,
    pub unit: String,
    pub source: DerivationSource,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    /// The farthest any chord of the table comes from its curve.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub max_deviation: f64,
    /// How each source kind's deviation is known: `exact`, `bound`, `sampled`, or `none` (its own points).
    pub kinds: BTreeMap<String, String>,
    /// A column written under another name than the attribute or field it holds (a name the table had already, in
    /// another case): the column's name → the source's. Absent when none was.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub renamed: BTreeMap<String, String>,
    /// The rule `kentos_digest` is made by (`DERIVATION_DIGEST`).
    pub digest: String,
    pub engine: String,
    /// When it was made, `YYYY-MM-DDTHH:MM:SS.SSSZ`.
    pub made: String,
}

/// Input of `cad.gis.derivation` v1, read only: the GeoPackage `path` made by `file.export.gis`, each of its rows
/// compared with the object it came from in the open drawing. Refusals, in order: `invalid_file_path`,
/// `file_not_found`, `import_unreadable`, `not_a_derivation`. Warnings: `derivation_stale`, `derivation_edited`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisDerivation {
    /// The file, an absolute path.
    pub path: String,
    /// How many rows to list (the ones that do not match come first); 100, at most 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0, max = 1000)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
}

/// How a derived row stands against its source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DerivationStatus {
    /// The source is as it was and the row is what the derivation gives.
    Current,
    /// The source changed after the dataset was made: the row is old.
    Stale,
    /// The source is as it was and the row was changed in the GIS (its geometry or an attribute).
    Edited,
    /// The source object is no longer in the drawing.
    SourceGone,
    /// A drawing object of an exported layer that the dataset has no row for (made, or moved there, since).
    NotExported,
}

/// One row (or missing row) that is listed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DerivationEntry {
    pub table: String,
    /// The row's fid; none for an object the dataset has no row for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub fid: Option<i64>,
    pub uid: String,
    pub status: DerivationStatus,
    /// The source object's kind as the row records it, or as the drawing has it now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    /// What differs in an edited row: `geometry` and the attributes' names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
    pub reason: String,
}

/// A table's counts against the drawing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DerivationTable {
    pub table: String,
    pub role: GisTableRole,
    /// The layer of a `layer` table, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub current: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edited: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub source_gone: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub not_exported: u32,
    /// Rows whose recorded deviation is over the table's tolerance.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub over_tolerance: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub max_deviation: f64,
}

/// What `cad.gis.derivation` found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisDerived {
    pub file: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    pub algorithm: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: f64,
    /// Where the dataset says it came from, and whether that is this drawing (its project id, when both have one,
    /// else its name).
    pub source: DerivationSource,
    pub same_project: bool,
    pub tables: Vec<DerivationTable>,
    /// Totals over the tables.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub current: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edited: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub source_gone: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub not_exported: u32,
    /// The rows that do not match first, then the matching ones, at most `limit`.
    pub entries: Vec<DerivationEntry>,
    /// How many rows were not listed because of `limit`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unlisted: u32,
    /// The drawing's revision now.
    pub revision: String,
}

impl DerivationRecord {
    /// A record's problem, in words; none when it is one this version reads.
    pub fn problem(&self) -> Option<String> {
        if self.format != DERIVATION_FORMAT {
            return Some(format!("“{}” bir türetim kaydı değil", self.format));
        }
        if self.version != 1 {
            return Some(format!(
                "türetim kaydının {}. sürümü okunmuyor",
                self.version
            ));
        }
        if !(self.tolerance.is_finite() && self.tolerance > 0.0) {
            return Some("türetim kaydının toleransı geçersiz".into());
        }
        None
    }
}

/// Whether `tolerance` (m) is one a derivation takes: finite and between 1e-6 and 1.
pub fn derivation_tolerance_ok(tolerance: f64) -> bool {
    tolerance.is_finite() && (1e-6..=1.0).contains(&tolerance)
}

/// What `cad.gis.takeback` takes from a derived row (docs/adr/0634 §1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TakebackPart {
    /// The columns changed in the GIS, written to the source's attributes (a text's words, a block's attributes);
    /// the source's geometry stays as it is.
    #[default]
    Attributes,
    /// The columns and the geometry: the source takes the GIS geometry. A source a curve stood for (a circle, an
    /// arc, a polyline's bulge, an ellipse, a curve) becomes the straight chords: its link to the analytic source is
    /// broken, and said.
    Geometry,
}

impl TakebackPart {
    pub fn is_default(&self) -> bool {
        *self == TakebackPart::Attributes
    }
}

/// A row of a derived dataset to take back: its table, its source's lasting id and what to take.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TakebackRow {
    pub table: String,
    pub uid: String,
    #[serde(default, skip_serializing_if = "TakebackPart::is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<TakebackPart>", optional))]
    pub take: TakebackPart,
}

/// The most rows one `cad.gis.takeback` takes.
pub const MOST_TAKEBACK_ROWS: usize = 10_000;

/// Input of `cad.gis.takeback` v1 (docs/adr/0634 §1): rows a GIS changed in a derived dataset written back to their
/// sources, in one undo step “GIS'ten geri al”. Only a row edited in the GIS whose source is as it was when exported
/// (`cad.gis.derivation`'s `edited`) is taken. Refusals, in order: `invalid_file_path`, `file_not_found`,
/// `import_unreadable`, `not_a_derivation`, `invalid_count` (no row, more than 10 000, a row twice);
/// `invalid_revision`, `revision_conflict`; `derivation_row_not_found`, `row_not_edited`, `row_stale`,
/// `row_source_gone`, `layer_locked`, `invalid_attribute`, `attribute_required`, `geometry_kind_mismatch`. Warnings:
/// `link_broken` (sources a curve stood for now chords), `geometry_left` (a row whose geometry changed, its
/// attributes taken only).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisTakeback {
    /// The derived file, an absolute path.
    pub path: String,
    pub rows: Vec<TakebackRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// A row taken back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TakenRow {
    pub table: String,
    pub uid: String,
    /// The source's attributes (or a text's, a block's parts) written, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
    /// Whether its geometry was taken.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub geometry: bool,
    /// The source's kind before, when its link to an analytic source was broken (`circle` became a `polygon`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<String>,
}

/// Output of `cad.gis.takeback` v1 (and its plan: nothing written).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisTaken {
    pub file: String,
    pub rows: Vec<TakenRow>,
    /// The objects that changed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    pub revision: String,
}

/// Input of `cad.gis.detach` v1 (docs/adr/0634 §2): a derived dataset's tables become the GIS's own data. Each
/// table's derivation record is replaced by a note of what it was (`DETACHED_STANDARD`) and its rows leave the
/// drawing's lasting ids, in one SQLite transaction: afterwards the table is bound, read and imported as any table,
/// its rows new objects; `cad.gis.derivation` no longer lists it. The drawing does not change. `tables` empty: every
/// derived table. Refusals: `invalid_file_path`, `file_not_found`, `import_unreadable`, `not_a_derivation`,
/// `derivation_row_not_found` (a table named that is not a derived table of the file), `invalid_count` (a table
/// twice).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisDetach {
    /// The derived file, an absolute path.
    pub path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub tables: Vec<String>,
    /// Why the link is broken, kept in the note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
}

/// A table whose link was broken.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DetachedTable {
    pub table: String,
    /// The source drawing's name the table came from.
    pub source: String,
    /// The rows that left the drawing's lasting ids.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
}

/// Output of `cad.gis.detach` v1 (and its plan: nothing written).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisDetached {
    pub file: String,
    pub tables: Vec<DetachedTable>,
    /// The derived tables left linked.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub linked: u32,
}

/// The note a broken link leaves in the file (`DETACHED_STANDARD`): the record it was, when and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationDetached {
    pub format: String,
    pub version: u32,
    /// When the link was broken, UTC `YYYY-MM-DDTHH:MM:SS.SSSZ`.
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub derivation: DerivationRecord,
}
