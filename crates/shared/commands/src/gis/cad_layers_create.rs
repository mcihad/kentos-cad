//! `cad.layers.create` v1 (docs/adr/9107 §7): a new layer of a GIS project made as a new table of a data source — a
//! GeoPackage file (made when it is not there) or a PostgreSQL schema — and added as a data layer, as Katmanlar's
//! Yeni katman ▸ Nokta, Çizgi, Alan katmanı… make it in a CBS project. Its objects are never kept in the project file:
//! they are the table's rows. The table is made in its own transaction; the layer added is one undo step (undone,
//! the layer goes and the table stays in the source, as any table a layer reads).

use serde::{Deserialize, Serialize};

use crate::LayerField;

pub const CAD_LAYERS_CREATE: &str = "cad.layers.create";
pub const CAD_LAYERS_CREATE_VERSION: u32 = 1;

/// Where the table is made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum CreateTarget {
    /// A table of a GeoPackage file (`path`), the file made when it is not there.
    #[default]
    Gpkg,
    /// A table of a PostgreSQL database (`address`) in `schema`.
    Postgis,
}

/// What the table's rows are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum CreateGeometry {
    #[default]
    Point,
    Line,
    Area,
    /// No geometry: a table of records.
    None,
}

/// Input of `cad.layers.create` v1.
///
/// The table: `table` (none: the layer's `name` made a table name), in `path` (`gpkg`) or `address` and `schema`
/// (`postgis`); its rows `geometry`, multi-part with `multi`, with Z with `z`, in `srid` (none: the project's system);
/// its columns `fields` (their names and kinds; a text's length); its key column `key` (none: `fid`) and geometry
/// column `geometryColumn` (none: `geom`). The layer: `name`, placed by `parent` and `index` as `cad.layers.data`
/// places one.
///
/// Refusals (`CommandError.code`), checked in this order: `empty_name` (`name` empty or only white space), `no_target`
/// (`gpkg` without `path`, `postgis` without `address`), `invalid_name` (a table or column name the source cannot take:
/// letters, digits and `_`, not starting with a digit, at most 63 characters), `invalid_fields` (`fields` as layers
/// take them: a repeated name, a kind's choices), `unknown_srid` (a system this project cannot write), `target_exists`
/// (the table is there), `source_unreachable` (the source could not be read or written; the message says why).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayersCreate {
    /// The layer's name in Katmanlar.
    pub name: String,
    pub target: CreateTarget,
    /// `gpkg`: the GeoPackage file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// `postgis`: the database, `postgresql://user@host:port/db` (its password is never written).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// `postgis`: the schema; none: `public`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// The table's name; none: the layer's name made a table name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    pub geometry: CreateGeometry,
    /// Every geometry a multi-part one (`MULTIPOINT` …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi: Option<bool>,
    /// With Z.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z: Option<bool>,
    /// The table's system, an EPSG code; none: the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub srid: Option<u32>,
    /// The columns, in order: each one's name, kind and (a text's) length; none: no column but the key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<LayerField>>,
    /// The key column's name; none: `fid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The geometry column's name; none: `geom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_column: Option<String>,
    /// The group the layer goes in; none: the top of the tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Its place among its siblings; none: the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub index: Option<u32>,
}

/// What `cad.layers.create` made: the layer added, the table, where it is, its geometry type, its system.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LayersCreated {
    /// The data layer added (its id); empty in a plan.
    pub layer: String,
    /// The table: a GeoPackage's table name, PostGIS's `schema.table`.
    pub table: String,
    /// The source: the GeoPackage's path, the database's address.
    pub address: String,
    /// OGC's name of the geometry type made (`MULTIPOLYGON Z`); none for a table of records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,
    /// The table's system; none for a table of records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub srid: Option<u32>,
    pub revision: String,
}
