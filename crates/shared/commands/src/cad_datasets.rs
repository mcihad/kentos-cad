//! `cad.datasets.edit` and `cad.datasets.describe` (docs/adr/0633): a layer as a dataset. The description (title,
//! responsible party, license, quality) is the layer's and written as one undo step; what it was derived from is
//! written by the processing run that made it. Describing works out the rest from the drawing whenever asked: the
//! extent, the system, the objects, the data dictionary, whether a derived layer's inputs changed since, and the
//! layers it came from and the layers made from it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{DatasetInfo, LayerDerivation, LayerFieldKind, LayerNode};

pub const CAD_DATASETS_EDIT: &str = "cad.datasets.edit";
pub const CAD_DATASETS_EDIT_VERSION: u32 = 1;
pub const CAD_DATASETS_DESCRIBE: &str = "cad.datasets.describe";
pub const CAD_DATASETS_DESCRIBE_VERSION: u32 = 1;

/// Input of `cad.datasets.edit` v1: the layer's description as a dataset, or none to take it away.
///
/// Refusals, in this order: `invalid_dataset`; `invalid_revision`, `revision_conflict` (status `conflict`);
/// `layer_not_found`, `not_a_layer`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatasetsEdit {
    /// The layer's id.
    pub layer: String,
    /// Its description; absent: none (the description is taken away).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dataset: Option<DatasetInfo>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.datasets.edit` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatasetsEdited {
    pub layer: String,
    /// Whether it differed (an undo step “Veri kümesi bilgisi” was written).
    pub changed: bool,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.datasets.edit` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatasetsEditPlan {
    /// The layer as execute would leave it.
    pub node: LayerNode,
    pub changed: bool,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.datasets.describe` v1: which layers; none, every layer holding objects in the tree's order.
///
/// Refusals: `layer_not_found`, `not_a_layer` (path `layers.<i>`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatasetsDescribe {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// Each layer also as an OGC API Records record (GeoJSON, `record`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub records: bool,
}

/// A field as the data dictionary says it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DictionaryEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alias: Option<String>,
    pub kind: LayerFieldKind,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub required: bool,
    /// Its unit, when it names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
    /// How many values its list allows (its own or its value list's); none when any value goes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub choices: Option<u32>,
    /// The objects with a value in it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub filled: u32,
}

/// Where an input of a derivation stands now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum InputState {
    /// Its content is what the run read.
    Current,
    /// Its objects changed since.
    Changed,
    /// The layer is not in the drawing any more.
    Gone,
}

/// An input of a derivation now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DerivationInputNow {
    pub layer: String,
    /// The name it had when read.
    pub name: String,
    pub state: InputState,
}

/// A layer described.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatasetDescription {
    pub layer: String,
    pub name: String,
    /// Its groups' names and its own, from the top (`Kadastro / Parsel`).
    pub path: String,
    /// The description's title, else the layer's name.
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dataset: Option<DatasetInfo>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The objects by kind (`polygon`, `point` …).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kinds: BTreeMap<String, u32>,
    /// The objects' box in the project's system, [min x, min y, max x, max y]; none without objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub extent: Option<[f64; 4]>,
    /// The same box in WGS 84 longitude and latitude; none for a local project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub geographic: Option<[f64; 4]>,
    /// The project's system (EPSG); none for a local project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    pub crs: String,
    /// The system its objects came from, when known (docs/adr/0583).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source_crs: Option<String>,
    /// The bound table it is a working copy of (docs/adr/0503).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DictionaryEntry>>", optional))]
    pub dictionary: Vec<DictionaryEntry>,
    /// What a processing tool made it from, as recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub derivation: Option<LayerDerivation>,
    /// Its inputs now.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DerivationInputNow>>", optional))]
    pub inputs: Vec<DerivationInputNow>,
    /// Whether one of its inputs changed or went since it was made, or a layer it came from is stale itself.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub stale: bool,
    /// The layers it came from, nearest first, each once (through their own derivations).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub upstream: Vec<String>,
    /// The layers made from it, nearest first, each once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub downstream: Vec<String>,
    /// The layer as an OGC API Records record (GeoJSON Feature), when asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(type = "unknown", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<serde_json::Value>"))]
    pub record: Option<Value>,
}

/// Output of `cad.datasets.describe` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatasetsDescribed {
    pub datasets: Vec<DatasetDescription>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
