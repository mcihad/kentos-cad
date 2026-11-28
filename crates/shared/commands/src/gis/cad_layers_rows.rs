//! `cad.layers.rows` v1 (docs/adr/9102 §4): a data layer's rows read from its source as Öznitelik tablosu pages them,
//! and as Python and the assistants read them: an order (a text in the Turkish alphabet's order), a page, a search, a
//! filter in İfadeyle seç's language, the view's box and a selection's keys, a count. The rows stay in the source:
//! nothing is written.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::QueryOrder;

pub const CAD_LAYERS_ROWS: &str = "cad.layers.rows";
pub const CAD_LAYERS_ROWS_VERSION: u32 = 1;
/// The most rows a page holds.
pub const LAYER_ROWS_PAGE_MOST: u32 = 1000;

/// Input of `cad.layers.rows` v1: the rows of `layer` (a data layer) that `bbox`, `keys`, `search` and `filter` all
/// keep, in `order` (then by key), `limit` of them from `offset`.
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer` (`layer` empty), `invalid_count` (`limit` past
/// 1 000), `invalid_extent` (a box reversed or not finite), `layer_not_found`, `not_a_data_layer`, `field_not_found`
/// (in `fields` or `order`), `invalid_expression` (the filter does not compile, reads `$sıra` or `$ölçek`, or looks at
/// other objects), `source_unreachable` (the source could not be read; the message says why). A job stopped answers
/// `cancelled`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayersRows {
    /// The data layer (its id).
    pub layer: String,
    /// Rows whose geometry's box meets this one, in the project's system: west, south, east, north (Görünümdekiler).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bbox: Option<[f64; 4]>,
    /// Rows of these keys only (Seçililer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub keys: Option<Vec<i64>>,
    /// Rows any of whose fields, read as text, holds these words, case aside the Turkish way (Ara).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    /// A condition in İfadeyle seç's language: the rows it holds for (İfade süzgeci).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// The fields returned, by name; none: all of them in the table's order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    /// Return each row's geometry in the project's system (ISO WKT).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<bool>,
    /// The order, field after field; then by key. A text is ordered in the Turkish alphabet's order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<QueryOrder>>,
    /// How many of the ordered rows come before the page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub offset: Option<u64>,
    /// How many rows the page holds, 0–1 000; none: 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
    /// Count every row the question keeps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<bool>,
    /// Return every kept row's key, in the order (a selection of the rows a filter keeps, a table's pages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all_keys: Option<bool>,
}

/// A row of a data layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LayerRow {
    /// Its key in the source.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub key: i64,
    /// The fields asked for, as the drawing's attributes read them (an empty value left out).
    pub attrs: BTreeMap<String, String>,
    /// Its geometry in the project's system, ISO WKT (with `geometry`; none for a row without one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wkt: Option<String>,
}

/// Where a question's filter was checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum RowsChecked {
    /// By the source: the box, the keys, the search and the filter, if any, which the source answered exactly
    /// (docs/adr/9109).
    Source,
    /// Here: every row the source kept (by the filter's part it could answer) was read and the filter asked of it.
    Here,
}

/// Output of `cad.layers.rows` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LayerRowsAnswer {
    /// The fields of the rows, in the table's order.
    pub fields: Vec<String>,
    /// The page's rows, in the order.
    pub rows: Vec<LayerRow>,
    /// The page's first row's place among the ordered rows.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub offset: u64,
    /// How many rows the question keeps (with `count`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub total: Option<u64>,
    /// Every kept row's key in the order (with `allKeys`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub keys: Option<Vec<i64>>,
    pub checked: RowsChecked,
    /// The layer's fields its table does not have (docs/adr/9318): not read, each row empty in them, the others read as
    /// they are. Veri kaynağına sütun ekle adds the column, Kaynak şemasını yenile takes the field off the layer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
}
