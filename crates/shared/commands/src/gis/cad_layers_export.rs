//! `cad.layers.export` v1 (docs/adr/9106): a layer's objects (a drawing layer's) or rows (a data layer's) written to a
//! GeoPackage table, a PostGIS table, a GeoJSON, Shapefile, CSV or KML file, as Katmanlar's Dışa aktar › Farklı kaydet…
//! and Seçilenleri farklı kaydet… write them, and as Python and the assistants export. What is written can be narrowed
//! (the selection, a box, a condition), its fields chosen, renamed and given their listed values' labels, its geometry
//! made one kind, multi-part, with or without Z and M, moved into another system. A data layer's rows are read from
//! their source a part at a time and written as they come: a table of a million rows is never held whole.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_LAYERS_EXPORT: &str = "cad.layers.export";
pub const CAD_LAYERS_EXPORT_VERSION: u32 = 1;

/// Where `cad.layers.export` writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExportFormat {
    /// A table of a GeoPackage file (`path`), made when the file is not there.
    #[default]
    Gpkg,
    /// A table of a PostGIS database (`address`), in `schema`.
    Postgis,
    /// A GeoJSON file: RFC 7946 (longitude, latitude in WGS 84) unless `options.projectCoordinates`.
    Geojson,
    /// A Shapefile (`path` ends `.shp`; its `.shx`, `.dbf`, `.prj`, `.cpg` beside it).
    Shapefile,
    /// A CSV file: the fields, the geometry as WKT or X and Y columns.
    Csv,
    /// A KML file (always longitude, latitude in WGS 84).
    Kml,
}

/// The geometry the written rows take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExportGeometry {
    /// The layer's own kind (a mixed drawing layer: each object's).
    #[default]
    Auto,
    Point,
    Line,
    Area,
    /// No geometry: a table of the fields alone.
    None,
}

/// What is done when the target is there already.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExportExisting {
    /// Nothing is written (`target_exists`).
    #[default]
    Refuse,
    /// The file is written anew (a GeoPackage loses its other tables).
    OverwriteFile,
    /// The table is dropped and made again; the file's other tables stay (GeoPackage, PostGIS).
    ReplaceLayer,
    /// The rows are added to the table; a field it does not have is left out with a warning (GeoPackage, PostGIS).
    Append,
}

/// The text encoding of a Shapefile's `.dbf` and of a CSV file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExportEncoding {
    #[default]
    Utf8,
    /// Windows-1254 (Turkish Windows): for programs that read only the old code pages.
    Windows1254,
}

/// A field written: the layer's field by name, the name it is written under, its listed values' labels.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportField {
    /// The layer's field.
    pub name: String,
    /// The name written; none: its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to: Option<String>,
    /// Its listed values written as their labels (Görünen değer); a value not listed stays as it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub labels: Option<bool>,
}

/// The format's own choices; each one a format does not have is refused.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportOptions {
    /// GeoPackage, PostGIS: the table's description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    /// GeoPackage, PostGIS: the key column's name; none: `fid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    /// GeoPackage, PostGIS: the geometry column's name; none: `geom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry_column: Option<String>,
    /// GeoPackage, PostGIS: no spatial index (one is made otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub no_spatial_index: Option<bool>,
    /// CSV: the separator, one character; none: `,`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub separator: Option<String>,
    /// CSV: the geometry as `x` and `y` columns (points only) instead of a `wkt` column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub xy: Option<bool>,
    /// GeoJSON: the coordinates in the system written (`srid`, or the layer's) with the old `crs` member, not moved
    /// into WGS 84.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub project_coordinates: Option<bool>,
}

/// Input of `cad.layers.export` v1: `layer`'s objects or rows written as `format`.
///
/// What is written: the layer's objects (a data layer's rows read from its source), those of `uids` (a drawing layer)
/// or `keys` (a data layer) only, those whose box meets `bbox`, those `filter` holds for; the texts and dimensions of a
/// drawing layer are left out of a table with geometry. Fields: `fields` in their order (none: every field of the
/// layer). The system: `srid` (none: a data layer's table's own, a drawing layer's the project's); GeoJSON (unless
/// `options.projectCoordinates`) and KML are always WGS 84.
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer` (`layer` empty), `no_target` (a file format without
/// `path`, PostGIS without `address`), `invalid_option` (a choice the format does not have: the message says which),
/// `invalid_name` (a table or field name that is empty or longer than the format takes), `layer_not_found`,
/// `not_a_layer` (a group), `service_layer`, `field_not_found` (in `fields`), `invalid_expression` (`filter`),
/// `unknown_srid` (a system this project cannot write), `target_exists` (the file or table is there and `existing` is `refuse`), `source_unreachable` (the
/// layer's source or the target could not be read or written; the message says why), `nothing_to_export` (no object or
/// row was kept). Warnings: `export_skipped` (objects that could not be written as the geometry asked, counted),
/// `field_left_out` (`append` to a table without the field).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersExport {
    /// The layer (its id).
    pub layer: String,
    pub format: ExportFormat,
    /// The file written (a GeoPackage, GeoJSON, Shapefile, CSV or KML file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// PostGIS: the database, `postgresql://user@host:port/db` (its password is never written).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub address: Option<String>,
    /// PostGIS: the schema; none: `public`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub schema: Option<String>,
    /// The table (GeoPackage, PostGIS) or the KML document's name; none: the layer's name made a table name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// The system written, an EPSG code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub srid: Option<u32>,
    /// Shapefile, CSV: the text encoding; none: UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub encoding: Option<ExportEncoding>,
    /// A drawing layer's objects to write (persistent ids): Seçilenleri farklı kaydet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    /// A data layer's rows to write (keys): Seçilenleri farklı kaydet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub keys: Option<Vec<i64>>,
    /// Only what meets this box, in the project's system: west, south, east, north.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bbox: Option<[f64; 4]>,
    /// Only what this condition in İfadeyle seç's language holds for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<String>,
    /// The fields written, in order; none: every field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fields: Option<Vec<ExportField>>,
    /// The geometry written; none: the layer's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<ExportGeometry>,
    /// Every geometry written as a multi-part one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub multi: Option<bool>,
    /// With Z (an object without its elevation 0); none: as the layer has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<bool>,
    /// With M; none: as the layer has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub m: Option<bool>,
    /// The target there already.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub existing: Option<ExportExisting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub options: Option<ExportOptions>,
    /// GeoPackage: the layer's dataset information written as the table's metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub metadata: Option<bool>,
    /// GeoPackage, PostGIS: the written table added to the layer tree as a data layer (one undo step).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub add_to_map: Option<bool>,
}

/// Output of `cad.layers.export` v1.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersExported {
    pub format: ExportFormat,
    /// The files written, absolute (a Shapefile's every file); none for PostGIS.
    pub files: Vec<String>,
    /// The table written (GeoPackage, PostGIS: `schema.table`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
    /// The rows (features, records) written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub written: u64,
    /// The fields as written, in order.
    pub fields: Vec<String>,
    /// The geometry written: its type name (`MULTIPOLYGON`, `POINT Z` …); none: no geometry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<String>,
    /// The system written, an EPSG code; none: none written (a CSV, a local project).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub srid: Option<u32>,
    /// `addToMap`: the data layer added (its id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The document's revision after the export (it moves only when a layer was added), as decimal text.
    pub revision: String,
}
