//! Shapefile, GeoJSON and KML/KMZ taken into the drawing as product commands (docs/adr/0563, plan/gis.md CBS-05):
//! `file.import.shp`, `file.import.geojson` and `file.import.kml` do what the desktop's windows did by themselves
//! (docs/adr/0046, 0053, 0222): the shared reader reads the file once, its layers' objects go onto a drawing layer or
//! a new one with the file's fields, the file's coordinate system is a question with an answer in the command's
//! input (`srid`, `reproject`) and in its output (the way the coordinates were moved and how sure that is), what the
//! reader could not take is said, and the whole import is one undo step recorded under the command's own name.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{DeclaredCrs, ImportedLayer};

/// A Shapefile (its .shp with the files beside it, or a zip archive) taken into the drawing.
pub const FILE_IMPORT_SHP: &str = "file.import.shp";
pub const FILE_IMPORT_SHP_VERSION: u32 = 1;
/// A GeoJSON file (RFC 7946, and the 2008 `crs` member) taken into the drawing.
pub const FILE_IMPORT_GEOJSON: &str = "file.import.geojson";
pub const FILE_IMPORT_GEOJSON_VERSION: u32 = 1;
/// A KML file or a KMZ archive taken into the drawing.
pub const FILE_IMPORT_KML: &str = "file.import.kml";
pub const FILE_IMPORT_KML_VERSION: u32 = 1;

/// Input of `file.import.shp` v1: the Shapefile at `path` taken into the drawing as one undo step “Shapefile:
/// ad.shp”. `path` is a `.shp` (the `.shx`, `.dbf`, `.prj` and `.cpg` of its name beside it, found without case; a
/// file missing is read without it and said) or a `.zip` whose every `.shp` is a layer (`entry` names one). The
/// system comes from the `.prj`; a file without one is taken to be in the project's system (`crs_undeclared`), one
/// whose `.prj` KentOS does not know needs `srid`. A file in another system than the project's comes in only with
/// `reproject`. Each source layer goes onto the layer `layers` names, else `layer`, else the drawing's layer of the
/// same name (case and Turkish marks aside), else onto a new layer in the import's group with the DBF's fields.
/// Refusals, in order: `invalid_file_path` (empty, relative, a folder, neither .shp nor .zip), `invalid_revision`,
/// `revision_conflict`, `file_not_found`, `import_unreadable`, `entry_not_found` (at `entry`), `crs_unknown` (at
/// `srid`), `layer_not_found`, `not_a_layer` and `layer_locked` (at `layer` or `layers.<name>`), `crs_mismatch` (at
/// `reproject`: the file is in another system and the move was not asked, or cannot be made), `nothing_to_import`.
/// Warnings: `crs_undeclared`, `crs_overridden`, `crs_unofficial`, `import_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportShp {
    /// The `.shp` or `.zip` to read, an absolute path.
    pub path: String,
    /// A zip archive's layer: its path in the archive (`katmanlar/parseller.shp`) or its name (`parseller`); absent:
    /// every layer in the archive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub entry: Option<String>,
    /// The system the coordinates are in (EPSG), whatever the file says: a file without a `.prj`, or one KentOS
    /// cannot read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// Move the coordinates of a file in another system into the project's (docs/adr/0222 §2).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reproject: bool,
    /// The file's layers (by their names in the file) whose objects are left out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub skip_layers: Vec<String>,
    /// A file's layer (its name in the file) → the drawing's layer (its id) its objects go onto.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub layers: BTreeMap<String, String>,
    /// The drawing's layer (its id) every layer `layers` does not name goes onto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The group the new layers go into, found by its name or made. Absent: one named after the file; empty: the
    /// top of the tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// The drawing's revision the input was prepared against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// Input of `file.import.geojson` v1: the GeoJSON file at `path` taken into the drawing as one undo step “GeoJSON:
/// ad.geojson”. The objects of a file that names its layers (`kentos.layer`, KentOS's own members) go onto those
/// layers, the others onto the layer the document's `name` (else the file's name) gives. The system is the `crs`
/// member's, else WGS 84 longitude, latitude (RFC 7946). Refusals, warnings and the rest as `file.import.shp`'s
/// (`entry_not_found` does not apply; a GeoJSON file always declares a system, so `crs_undeclared` does not either).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportGeojson {
    /// The file to read, an absolute path.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reproject: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub skip_layers: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub layers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// Input of `file.import.kml` v1: the KML file or KMZ archive at `path` taken into the drawing as one undo step
/// “KML: ad.kml”. Folders and placemarks become layers and objects (a placemark's style its colour and weight, its
/// name, description, time and data its label and attributes); the coordinates are WGS 84 longitude, latitude (KML 2.2).
/// Refusals, warnings and the rest as `file.import.shp`'s (`entry_not_found` does not apply; a KML file is always in
/// WGS 84, so `crs_unknown` and `crs_undeclared` do not either).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportKml {
    /// The file to read, an absolute path (`.kml` or `.kmz`).
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reproject: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub skip_layers: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub layers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// How a file's coordinates were moved into the project's system, and how sure that is (docs/adr/0222 §2): the two
/// systems' EPSG codes (the project's is 0 when it is its own definition), the datum shift's operations (empty when
/// only the projection changed), its accuracy in metres (none when a step's is not written), whether an operation
/// that is not the official transformation was used, the same as a sentence, and what the move left out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsMove {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub from: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub to: u32,
    /// The datum shift's operations joined by “ + ”; empty when only the projection changed.
    pub via: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub accuracy: Option<f64>,
    pub unofficial: bool,
    /// “±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil”, “kesin, yalnız projeksiyon”.
    pub text: String,
    /// Objects moved, and objects left out because a vertex could not be moved (the report says why).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moved: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: u32,
}

/// What `file.import.shp`, `.geojson` or `.kml` took in, or with `plan` would take.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GisImported {
    /// The file's name, its bytes and their SHA-256 (a Shapefile's: its `.shp`'s).
    pub file: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// What the file says of its system (a `.prj`'s, a `crs` member, RFC 7946's or KML's WGS 84); none: nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub declared: Option<DeclaredCrs>,
    /// The system the coordinates were taken to be in: `srid`, else the file's, else the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// How the coordinates were moved; absent when they were taken as they are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub moved: Option<CrsMove>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The file's layers read (a zip's every `.shp`), as they are named in the file.
    pub source_layers: Vec<String>,
    /// A file's layer → the drawing's layer (its id) its objects went onto.
    #[serde(default)]
    pub targets: BTreeMap<String, String>,
    /// The layers made, in the tree's order, and the group they went into.
    pub layers: Vec<ImportedLayer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// What the reader counted and could not take (`ImportReport`).
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub report: serde_json::Value,
    /// The drawing's revision after the step (plan: before it).
    pub revision: String,
}
