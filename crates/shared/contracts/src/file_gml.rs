//! GML and XYZ in and out (docs/adr/0545, plan/gis.md CBS-05): OGC GML 3.2 simple features as GDAL's GML driver writes
//! and reads them (one feature type for each layer, an `.xsd` beside it, arcs as `ArcString` segments, exact, the
//! system as its `urn:ogc:def:crs:EPSG::` name with the axes in EPSG's order), and the points or vertices of a
//! drawing as text lines of X Y Z.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The drawing's layers as one GML.
pub const FILE_EXPORT_GML: &str = "file.export.gml";
pub const FILE_EXPORT_GML_VERSION: u32 = 1;
/// A GML's features taken into the drawing.
pub const FILE_IMPORT_GML: &str = "file.import.gml";
pub const FILE_IMPORT_GML_VERSION: u32 = 1;
/// The drawing's points or vertices as X Y Z lines.
pub const FILE_EXPORT_XYZ: &str = "file.export.xyz";
pub const FILE_EXPORT_XYZ_VERSION: u32 = 1;

fn yes() -> bool {
    true
}

fn is_yes(b: &bool) -> bool {
    *b
}

/// Input of `file.export.gml` v1: the layers' objects as one GML 3.2 file at `path`, each layer a feature type named
/// as the layer (made an XML name), its fields the feature's properties, its geometry the `geometryProperty`; the
/// project's system as `srsName` with the coordinates in the order EPSG gives the system's axes (TUREF's TM zones and
/// the geographic systems north or latitude first). An application schema `<name>.xsd` is written beside it unless
/// `schema` is false. Written whole or not at all (the schema first, the GML last). Refusals, in order:
/// `invalid_file_path`, `file_exists`, `layer_not_found`, `nothing_to_export`, `file_unwritable`. Warnings:
/// `export_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportGml {
    /// An absolute path ending `.gml` (or `.xml`).
    pub path: String,
    /// The layers, by id (a group gives its layers); empty: every layer with objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// Write the `.xsd` beside the file (GDAL and QGIS read the fields' types from it); absent: written.
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub schema: bool,
    /// Write over the GML and its `.xsd` when they are there.
    #[serde(default)]
    pub overwrite: bool,
}

/// A file written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GmlFileWritten {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}

/// A feature type written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GmlLayerWritten {
    /// The feature type's element name in the file.
    pub name: String,
    /// The drawing's layer (its id).
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub features: u32,
    /// The properties, in order, and their types as the `.xsd` names them.
    pub columns: Vec<String>,
    pub types: Vec<String>,
    /// The geometry property's type as GML names it (`GeometryPropertyType` for mixed types); `None` without geometry.
    pub geometry: String,
}

/// What `file.export.gml` wrote (or with `plan` would write).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GmlExported {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub features: u32,
    pub layers: Vec<GmlLayerWritten>,
    /// The GML and, unless left out, its `.xsd`.
    pub files: Vec<GmlFileWritten>,
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

/// Input of `file.import.gml` v1: the GML at `path` taken into the drawing as one undo step “GML: ad.gml”, each
/// feature an object (or a record without a geometry), its properties its attributes. GML 2, 3.1 and 3.2 are read
/// (`featureMember`, `featureMembers` and `member`; `Point`, `LineString`, `Polygon`, the multi geometries, `Curve`
/// with `ArcString` and `Arc` segments, `CompositeCurve`, `Surface`, with `pos`, `posList` and `coordinates`); a
/// feature type is a table as GeoPackage import's: onto the layer `layers` names, else onto the drawing's layer of
/// the type's name, else onto a new layer with the properties as its fields. The properties' types come from the
/// `.xsd` beside the file, else from its `.gfs`, else from the values. Refusals as `file.import.gpkg`'s:
/// `invalid_file_path`, `file_not_found`, `import_unreadable`, `table_not_found`, `layer_not_found`, `not_a_layer`,
/// `locked_layer`, `crs_mismatch`, `entity_exists`, `nothing_to_import`. Warnings: `crs_undeclared`,
/// `import_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportGml {
    /// An absolute path to a `.gml` (or `.xml`).
    pub path: String,
    /// The feature types to take (their element names); empty: every one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub tables: Vec<String>,
    /// A feature type → the drawing's layer (its id) its features go onto.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub layers: BTreeMap<String, String>,
    /// The system the coordinates are in when the file names none (EPSG), in east–north order.
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

/// Which positions an XYZ file holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XyzVertices {
    /// The points' positions (a multi point's parts too); every other object is left out, counted.
    #[default]
    Points,
    /// Every vertex of every object: the points, the vertices of lines and of areas' rings and holes (a closed
    /// ring's repeated last vertex once), an arc's end points (not its middle point).
    All,
}

/// The XYZ file's column separator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XyzSeparator {
    #[default]
    Space,
    Comma,
    Semicolon,
    Tab,
}

/// What a position without an elevation becomes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XyzWithoutZ {
    /// Written with Z 0 (the file has three columns; the drawing's position stays without an elevation).
    #[default]
    Zero,
    /// Not written, counted.
    Skip,
}

/// Input of `file.export.xyz` v1: the layers' positions as text lines `X Y Z` at `path` (UTF-8, LF lines; X east and Y
/// north in the project's system, as GDAL's XYZ driver and point cloud tools read them; the file names no system, the
/// answer does). Numbers are the shortest text that reads back to the double, or `decimals` places. Written whole or
/// not at all. Refusals, in order: `invalid_file_path`, `file_exists`, `invalid_input` (`decimals` over 15),
/// `layer_not_found`, `nothing_to_export`, `file_unwritable`. Warnings: `export_losses`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportXyz {
    /// An absolute path ending `.xyz` (or `.txt`, `.csv`, `.pts`).
    pub path: String,
    /// The layers, by id (a group gives its layers); empty: every layer with objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    #[serde(default)]
    pub vertices: XyzVertices,
    #[serde(default)]
    pub separator: XyzSeparator,
    /// Decimal places (0 to 15), fixed; absent: the shortest text that reads back to the number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: Option<u8>,
    /// A first line naming the columns (`X Y Z`).
    #[serde(default)]
    pub header: bool,
    #[serde(default)]
    pub without_z: XyzWithoutZ,
    /// Write over the file when it is there.
    #[serde(default)]
    pub overwrite: bool,
}

/// What `file.export.xyz` wrote (or with `plan` would write).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XyzExported {
    pub path: String,
    /// Position lines written (the header aside).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    /// Objects that gave at least one line.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// Lines whose elevation is the drawing's (the others have Z 0).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub with_z: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// The system the numbers are in (EPSG), which the file does not name; none for a project without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// Objects not written, by kind (with points only, every object that is no point).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: BTreeMap<String, u32>,
    /// What the file could not hold, a sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub lost: Vec<String>,
}
