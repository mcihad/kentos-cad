//! GeoPackage in and out (docs/adr/0502, plan/gis.md CBS-03): a file's feature and attribute tables taken into the
//! drawing as layers of objects and records with their fields, as one undo step named after the file; and the
//! drawing's layers written as a GeoPackage's tables, whole or not at all. Each row keeps a lasting id: the file's
//! (`kentos_uids`, written by KentOS) or one derived from its table and fid, the same every time.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ImportGpkg {
    /// The file to read, an absolute path.
    pub path: String,
    /// The tables to take; empty: every feature and attribute table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<String>,
    /// A table → the drawing's layer (its id) its rows go onto. A table not named here goes onto the drawing's layer
    /// of the same name (case and Turkish marks aside), else onto a new layer with the table's fields.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub layers: BTreeMap<String, String>,
    /// Move the coordinates from the file's system into the project's (the project's datum choices taken); without
    /// it a file in another system is refused.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reproject: bool,
    /// Rows meeting the objects already on the layer they go onto: updated, added or refused by a key (docs/adr/0628
    /// §5); none: every row is added.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge: Option<crate::ImportMerge>,
    /// Where the rows refused go, a CSV with their reasons (docs/adr/0628 §6; `execute` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejects: Option<String>,
}

/// A table taken.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct GpkgImported {
    /// The file's name, its bytes and their SHA-256.
    pub file: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// The EPSG code the tables declare, none for an undefined system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// Whether the coordinates were moved into the project's system.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub moved: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    pub tables: Vec<GpkgTableImported>,
    /// The layers made, in the tree's order, and the group they went into.
    pub layers: Vec<ImportedLayer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// The project's relations made from the file's related tables (docs/adr/0627 §6): KentOS's own, and another
    /// writer's many-to-many relations as multi-valued reference fields; their ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<String>,
    /// Layers the import made with their table's default style (`layer_styles`) as their look (docs/adr/0627 §7).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub styles: u32,
    /// Layers the import made with their table's description as a dataset (docs/adr/0633 §5).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub datasets: u32,
    /// The tables' indexes the layers the import made took (docs/adr/0758 §5).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub indexes: u32,
    /// The drawing's revision after the step (plan: before it).
    pub revision: String,
    /// What the merge did on each layer (docs/adr/0628 §5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merged: Vec<crate::MergeReport>,
    /// The CSV of the rows refused (docs/adr/0628 §6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejects: Option<crate::RejectsWritten>,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// Input of `file.export.gpkg` v1: the layers (all with objects when none), or only the objects named, as tables of a
/// new GeoPackage, each object a row with its lasting id (`kentos_uids`), its layer's fields and its other attributes as columns; the project's
/// coordinate system as the tables'. A file that is there is written as `write` says (docs/adr/9191). Refusals, in
/// order: `invalid_file_path`, `file_exists`, `layer_not_found`, `entity_not_found`, `nothing_to_export`,
/// `geometry_mismatch`, `crs_mismatch`, `source_not_null` (rows a table of the file cannot hold, `write: append`),
/// `file_unwritable`. Warnings: `export_losses` (objects a GeoPackage has no geometry for, curves written as chords,
/// fields an appended table has no column for, objects it has already).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ExportGpkg {
    /// The file to write, an absolute path (`.gpkg`).
    pub path: String,
    /// The layers, by id; empty: every layer with objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<String>,
    /// Only these objects, by persistent id (Seçili nesneler, docs/adr/1073): each on its layer's table, the other
    /// objects of those layers and the other layers left out; with `layers`, only those of them on the given layers.
    /// Refused `entity_not_found` for an id the drawing does not have.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uids: Vec<String>,
    /// With no `layers` and no `uids`: also every layer without objects whose type (point, line, area) or fields make
    /// a table, written empty with its columns (a field team's template, docs/adr/1076).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub empty_layers: bool,
    /// Write over a file that is there.
    #[serde(default)]
    pub overwrite: bool,
    /// How a file that is there is written (with `overwrite`; docs/adr/9191): `file` (the default) a new file in its
    /// place, `tables` only the tables written again, `append` the rows added to its tables. A file not there is made
    /// new whatever it says.
    #[serde(default, skip_serializing_if = "GpkgWrite::is_file")]
    pub write: GpkgWrite,
    /// Only the objects of a valid time and a day known (docs/adr/0646 §2), by `cad.time.query`'s rules: of each
    /// written layer with time settings (of every one when `fields` names them); a layer without them whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<crate::temporal::QueryTime>,
}

/// How `file.export.gpkg` writes a file that is there (docs/adr/9191).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum GpkgWrite {
    /// A new file in its place: the file's other tables go.
    #[default]
    File,
    /// Only the tables written are written again (a table the file lacks is made); the file's other tables, their
    /// styles and relations stay. A relation the file declares on a table written again goes with its old rows.
    Tables,
    /// The rows are added to the file's tables of the same names (a table the file lacks is made); their columns,
    /// styles and relations stay. A field a table has no column for is left out, an object whose lasting id the table
    /// has is not added again; a table that cannot hold the rows (another geometry family, multi-part or curves its
    /// column does not take, Z or M, another system, a NOT NULL column the layer does not fill) is refused.
    Append,
}

impl GpkgWrite {
    pub fn is_file(&self) -> bool {
        *self == GpkgWrite::File
    }
}

/// A table written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct GpkgTableExported {
    pub table: String,
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    /// Its geometry column's type (`MULTIPOLYGON` …); none for an attribute table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,
    /// The layer's indexes written over it (docs/adr/0758 §5).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub indexes: u32,
    /// The file's table of this name, written again (`write: tables`, docs/adr/9191).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replaced: bool,
    /// The file's table of this name, its `rows` added to it (`write: append`); its columns, style and relations are
    /// the file's.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub appended: bool,
    /// Objects not added because the table has their lasting id already (`write: append`).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub already: u32,
}

/// A project relation written as an OGC 18-000 related table (docs/adr/0627 §6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct GpkgRelationExported {
    /// The mapping table, its rows the child's and the parent's fids.
    pub mapping: String,
    /// The child layer's table (the relation's base) and the parent's (the related).
    pub base: String,
    pub related: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
}

/// What `file.export.gpkg` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct GpkgExported {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    pub tables: Vec<GpkgTableExported>,
    /// Objects not written, by kind.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: BTreeMap<String, u32>,
    /// Curves (ellipses, splines, NURBS) written as chords within 0.1 mm.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub approximated: u32,
    /// The project's relations between the tables written (docs/adr/0627 §6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<GpkgRelationExported>,
    /// Feature tables given their layer's look as their default style (`layer_styles`, docs/adr/0627 §7).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub styles: u32,
    /// The file's tables left as they were (`write: tables` or `append`, docs/adr/9191).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kept: Vec<String>,
}
