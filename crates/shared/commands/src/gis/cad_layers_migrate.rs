//! `cad.layers.migrate` v1 (docs/adr/9107 §8): a drawing layer of a GIS project whose objects are kept in the
//! project file moved into a data source in one step — its objects written as the rows of a new table of a GeoPackage
//! (made when it is not there) or a PostgreSQL schema, and the layer replaced by a data layer reading that table, in
//! its place, with its name, style and labels. One undo step: undone, the drawing layer and its objects are back; the
//! table stays in its source.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::CreateTarget;

pub const CAD_LAYERS_MIGRATE: &str = "cad.layers.migrate";
pub const CAD_LAYERS_MIGRATE_VERSION: u32 = 1;

/// Input of `cad.layers.migrate` v1.
///
/// `layer`'s objects written to `table` (none: the layer's name made a table name) in `path` (`gpkg`) or `address` and
/// `schema` (`postgis`), in the project's system, every field of the layer a column, every object of the layer (those
/// its layer filter hides too: the layer goes, so none may stay behind).
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer` (`layer` empty), `no_target` (`gpkg` without
/// `path`, `postgis` without `address`), `invalid_name`, `layer_not_found`, `not_a_layer` (a group), `service_layer`,
/// `data_layer` (the layer reads a table already), `layer_locked`, `target_exists` (the table is there),
/// `source_unreachable`, `nothing_to_export` (nothing a table holds: an empty layer, or one of texts only),
/// `unwritable_objects` (texts, dimensions or anything a table cannot hold beside what it can: they would be lost with
/// the layer, so nothing is written; their count and the first's kind are said).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersMigrate {
    /// The drawing layer (its id).
    pub layer: String,
    pub target: CreateTarget,
    /// `gpkg`: the GeoPackage file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// `postgis`: the database, `postgresql://user@host:port/db` (its password is never written).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub address: Option<String>,
    /// `postgis`: the schema; none: `public`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub schema: Option<String>,
    /// The table's name; none: the layer's name made a table name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
}

/// What `cad.layers.migrate` did: the data layer that took the layer's place, the table, where, how many rows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersMigrated {
    /// The data layer (its id); empty in a plan.
    pub layer: String,
    /// The table: a GeoPackage's table name, PostGIS's `schema.table`.
    pub table: String,
    /// The source: the GeoPackage's path, the database's address.
    pub address: String,
    /// The rows written: the layer's objects a table holds.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub written: u64,
    /// OGC's name of the geometry type written; none for a table of records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<String>,
    pub revision: String,
}
