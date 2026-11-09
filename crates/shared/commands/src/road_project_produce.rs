//! `road.project.produce` v1 (plan/map-cad.md MAP-20, docs/adr/3240): Tariften üret. The drawings of a recipe (İş
//! grafiği's Tarifi kaydet…: `road.project.graph`'s answer written as JSON, docs/adr/1940) drawn in this project: each
//! drawing's command and version, its input as the recipe keeps it, the sources it names by their names in the recipe
//! (a source of the recipe's project named by its id there is found here by its name), in the recipe's order, in one
//! undo step. A drawing this project already has (the same command from the same main source) is left: drawing it
//! again is `road.project.rebuild`'s. A drawing's layer (docs/adr/3640) is the one `layers` maps its recipe layer to;
//! else, the recipe keeping the layer's definition, the layer of that name here (folded, as Katmanları çevir finds a
//! standard layer), or one added with the recipe's look; a recipe from before goes by the layer's id. The shared cases
//! are `fixtures/commands/v1/road.project.produce.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{RoadDrawingLeft, RoadSourceKind};

pub const ROAD_PROJECT_PRODUCE: &str = "road.project.produce";
pub const ROAD_PROJECT_PRODUCE_VERSION: u32 = 1;

/// Input of `road.project.produce` v1: `path`, the recipe (an absolute `.json` path: İş grafiği's Tarifi kaydet…);
/// `drawings`, only these of its drawings (their ids in the recipe), none: every drawing it holds a recipe for;
/// `layers` (docs/adr/3640), a recipe layer's id → the layer of this project its drawings go on (AutoCAD's Map in the
/// Layer Translator); a layer not given goes by its name (Map Same) or is added.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`; `invalid_file_path`,
/// `file_not_found`, `invalid_project_recipe` (at `path`: not a recipe, no drawing in it, a drawing's command not one
/// that draws or its version not the one this KentOS draws); `unknown_drawing` (at `drawings`: not one of the
/// recipe's); `unknown_recipe_layer` (at `layers`: a key not a layer of the recipe's drawings), `layer_not_found`,
/// `not_a_layer`, `service_layer`, `layer_locked` (at `layers.<id>`: the layer given here); `ambiguous_recipe_layer`
/// (at `layers`: a named drawing's layer name held by more than one layer here); and a drawing command's own refusal (a
/// source not in this project, a layer not here, gone or locked), the recipe's drawing named in its words: nothing is
/// written.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectProduce {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawings: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub layers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// How a recipe layer was found here (docs/adr/3640): `given` in `layers`, by its `name`, by its `id` (a recipe from
/// before, keeping no definition), or `created` with the recipe's look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoadLayerMatch {
    Given,
    Name,
    Id,
    Created,
}

/// A recipe layer its produced drawings went on (docs/adr/3640): its id and name in the recipe, the layer here and how
/// it was found; in the order the drawings first use them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProducedLayer {
    pub from: String,
    pub name: String,
    pub to: String,
    pub to_name: String,
    pub how: RoadLayerMatch,
}

/// The main source a produced drawing was drawn from in this project: its kind, id and name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProducedSource {
    pub kind: RoadSourceKind,
    pub id: String,
    pub name: String,
}

/// A recipe's drawing drawn here: its id in the recipe (`from`), the drawing kept here, its command, its main source
/// here and the objects written (a plan: none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadDrawingProduced {
    pub from: String,
    pub drawing: String,
    pub command: String,
    pub source: RoadProducedSource,
    pub uids: Vec<String>,
}

/// Output of `road.project.produce` v1, and its plan: the drawings drawn in the recipe's order, those left (`exists`:
/// this project has the drawing; `rebuild` draws it again) and the revision after (a plan: before, no objects).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectProduced {
    pub produced: Vec<RoadDrawingProduced>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RoadDrawingLeft>>", optional))]
    pub left: Vec<RoadDrawingLeft>,
    /// The layers the produced drawings went on (docs/adr/3640).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RoadProducedLayer>>", optional))]
    pub layers: Vec<RoadProducedLayer>,
    pub revision: String,
}
