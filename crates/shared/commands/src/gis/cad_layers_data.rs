//! `cad.layers.data` v1 (docs/adr/9101 §4): a data layer added to the layer tree, changed or removed, as one undo step
//! named after the operation. A data layer is drawn from a table of a GeoPackage or a database as it is now; its rows
//! stay in the source and are never written into the drawing (plan/veri.md VY-02).

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::data_layer::DataLayer;
use crate::fields::LayerField;
use crate::layer::LayerNode;

pub const CAD_LAYERS_DATA: &str = "cad.layers.data";
pub const CAD_LAYERS_DATA_VERSION: u32 = 1;

/// What `cad.layers.data` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum LayerDataOperation {
    /// Veri katmanı ekle.
    Add,
    /// Veri katmanını değiştir: its name, its table (another place of the same table, a moved file) or its fields.
    Update,
    /// Veri katmanını kaldır: the layer goes, the table stays as it is.
    Remove,
}

/// Input of `cad.layers.data` v1.
///
/// - `add`: `name`, `data`, `fields` (the table's columns as the layer's fields; absent: none); `parent` and `index`
///   place it as `cad.layers.service` places a layer;
/// - `update`: `layer`, and any of `name`, `data`, `fields`;
/// - `remove`: `layer`.
///
/// The host reads the table's catalog (its columns, key, geometry column, system) and gives it here: the command reads
/// no source. Refusals (`CommandError.code`), checked in this order: `no_layer` (update and remove without `layer`),
/// `empty_name` (add; update with a name that is empty or only white space), `no_data` (add without `data`),
/// `invalid_data` (`DataLayer::problem`), `invalid_fields` (`fields_problem`); then `invalid_revision`,
/// `revision_conflict` (status `conflict`), `layer_not_found`, `not_a_layer` (a group), `not_a_data_layer` (update or
/// remove of a layer that is not a data layer).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayersData {
    pub operation: LayerDataOperation,
    /// `update`, `remove`: the layer's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// `add`: the new layer's name; `update`: a new name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The group it goes into, or a layer whose group it goes into; absent, the top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Its place among the group's nodes, first on top; absent or past the end, last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: Option<u32>,
    /// `add`: where its rows are; `update`: where they are now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<DataLayer>,
    /// The table's columns as the layer's fields (docs/adr/0199 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<LayerField>>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.layers.data` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LayersDataDone {
    /// The layer added, changed or removed.
    pub layer: String,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.layers.data` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LayersDataPlan {
    /// The layer as execute would leave it (`add`: with the id it would be given); absent for `remove`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<LayerNode>,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
