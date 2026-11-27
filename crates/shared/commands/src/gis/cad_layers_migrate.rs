//! `cad.layers.migrate` v1 (docs/adr/9107 §8, 9321): a drawing layer of a GIS project whose objects are kept in the
//! project file moved into a data source in one step — its objects written as the rows of a new table of a GeoPackage
//! (made when it is not there) or a PostgreSQL schema; they leave the drawing and the layer itself reads that table,
//! under its own id, with its name, style, labels, filter and every reference to it (a network's edges, a relation, a
//! form, a join, a view). One undo step: undone, the drawing layer and its objects are back; the table stays in its
//! source. The layer's texts go into a table of their own beside it, read by a data layer that draws them as texts
//! (docs/adr/9114 §2).

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
/// its layer filter hides too: the layer reads the table afterwards, so none may stay behind).
///
/// Its texts are written to a table of their own beside it, `<table>_yazilar` (a layer of texts only: to `table`): a
/// point and the columns `metin`, `yukseklik`, `aci`, `hiza` and `genislik` (docs/adr/9114 §2). A data layer reading
/// it, its rows drawn as texts, goes above the layer's (a layer of texts only: in its place, with its name). A text's
/// style, mask, curve, scales, link to an object or paragraph formats are not written: said as `text_details_left`.
///
/// A table there already (docs/adr/9339): refused (`existing` absent or `refuse`), made again (`replaceLayer`: dropped
/// first, the source's other tables kept) or added to (`append`: the rows after the table's own, a field the table has
/// no column for left out and said); the texts' table beside it alike. The layer reads the whole table afterwards.
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer` (`layer` empty), `no_target` (`gpkg` without
/// `path`, `postgis` without `address`), `invalid_option` (`existing: overwriteFile`: the file's other tables would
/// go), `invalid_name`, `layer_not_found`, `not_a_layer` (a group), `service_layer`,
/// `data_layer` (the layer reads a table already), `layer_locked`, `target_exists` (the table, or the texts' table, is
/// there and `existing` refuses it), `source_unreachable`, `nothing_to_export` (an empty layer), `unwritable_objects` (dimensions, blocks, hatches
/// or anything else a table cannot hold, texts aside: they would be lost with the layer, so nothing is written; their
/// count and the first's kind are said).
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
    /// A table there already: `refuse` (absent), `replaceLayer` or `append`; `overwriteFile` is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub existing: Option<crate::ExportExisting>,
}

/// What `cad.layers.migrate` did: the layer, reading its table now, the table, where, how many rows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersMigrated {
    /// The layer, reading its table now: the id it had (docs/adr/9321); empty in a plan.
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
    /// The layer's texts' table beside it (docs/adr/9114 §2); none when it had no texts, or only texts (then the
    /// table above is theirs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub texts: Option<MigratedTexts>,
    pub revision: String,
}

/// A migrated layer's texts: the data layer drawing them, their table, how many rows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MigratedTexts {
    /// The data layer (its id); empty in a plan.
    pub layer: String,
    /// The table: a GeoPackage's table name, PostGIS's `schema.table`.
    pub table: String,
    /// The rows written: one a text.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub written: u64,
}
