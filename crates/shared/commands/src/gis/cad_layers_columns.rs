//! `cad.layers.columns` v1 (docs/adr/9318): a data layer's table and its fields kept together. `add` adds columns to
//! the table through its driver (Veri kaynağına sütun ekle) and, once the table read again has them, the layer's fields;
//! `refresh` reads the table's columns again (Kaynak şemasını yenile) and takes the layer's fields to them: a column
//! made elsewhere comes in, a field the table no longer has goes. The layer's fields change as one undo step; a column
//! added stays in the table, as a row written by `cad.layers.session` stays.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::fields::LayerField;

pub const CAD_LAYERS_COLUMNS: &str = "cad.layers.columns";
pub const CAD_LAYERS_COLUMNS_VERSION: u32 = 1;

/// What `cad.layers.columns` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LayerColumnsAction {
    /// Veri kaynağına sütun ekle: each field a nullable column of the table, empty in its rows; then the layer's field.
    Add,
    /// Kaynak şemasını yenile: the layer's fields taken to the table's columns as it is now.
    Refresh,
}

/// Input of `cad.layers.columns` v1.
///
/// - `add`: `layer`, `fields` (the new columns as the layer's fields: name, kind and the field's own rules); a column
///   is nullable, so the table's rows are empty (NULL) in it; the layer's field keeps its rules (a default for new rows,
///   a value list);
/// - `refresh`: `layer`.
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer` (no `layer`), `no_columns` (add without fields),
/// `invalid_name` (a name a source cannot take: empty, longer than 63 bytes, a control character, a name kept for the
/// source's own tables, the key's or the geometry's column), `column_exists` (a field of the layer or a column of the
/// table, folded the Turkish way, or named twice), `invalid_fields` (the fields' rules over the layer's and the new);
/// then `invalid_revision`, `revision_conflict` (status `conflict`), `layer_not_found`, `not_a_layer`,
/// `not_a_data_layer`, `data_layer_read_only` (add to a layer kept read-only); `source_unreachable` (the table cannot
/// be read, or the source refused the change: the table is as it was); `columns_not_permitted` (add where the source
/// or the account may not add a column: a PostgreSQL table the account does not own, a file opened read-only, a query).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersColumns {
    pub action: LayerColumnsAction,
    /// The data layer's id.
    pub layer: String,
    /// `add`: the new columns as the layer's fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fields: Option<Vec<LayerField>>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.layers.columns` v1, and its plan: what is (or would be) done.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersColumnsDone {
    pub layer: String,
    /// The table the layer reads (a GeoPackage's table name, a database's `schema.table`).
    pub table: String,
    /// `add`: the columns added to the table; `refresh`: the table's columns taken in as fields.
    pub added: Vec<String>,
    /// `refresh`: the layer's fields the table no longer has, taken off the layer.
    pub removed: Vec<String>,
    /// The layer's fields after.
    pub fields: Vec<LayerField>,
    /// `add`: the table's rows, each empty (NULL) in an added column; an estimate where the source counts so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: Option<u64>,
    /// The document's revision after (the plan's: as it is), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
