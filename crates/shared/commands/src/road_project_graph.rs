//! `road.project.graph` v1 (plan/map-cad.md MAP-20, docs/adr/1940): İş grafiği, a road or map project's chain from
//! its sources to its sheets as one graph, read only. The rows of `road.project.query`'s tree (surfaces, alignments …
//! and what their commands drew), the layers a surface is made of before them and the sheets that show a drawing
//! after them; the edges say what builds a surface (`builds`), what a row is made on (`basis`), what a drawing was made
//! from (`source`, every source, with its stamp then and now) and which sheet's fixed map shows it (`shows`). Every
//! drawing carries its recipe: the command and its version, the input as kept, the objects written and their stamp;
//! the project's coordinate system, epoch and heights come with the answer. A sheet that shows a drawing not up to
//! date is stale: the print check refuses it (`outdated_drawing`, docs/adr/1720 §5). The shared cases are
//! `fixtures/commands/v1/road.project.graph.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::RoadNodeState;

pub const ROAD_PROJECT_GRAPH: &str = "road.project.graph";
pub const ROAD_PROJECT_GRAPH_VERSION: u32 = 1;

/// Input of `road.project.graph` v1: `deep` reads the rows as `road.project.query` does deep (surfaces and corridors
/// made, approved volumes computed).
///
/// Refusals: `not_a_road_project`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectGraphQuery {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub deep: bool,
}

/// What a node of the graph is: a layer a surface is made of, a row of the project tree, or a sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoadGraphKind {
    Layer,
    Surface,
    Alignment,
    Profile,
    Sections,
    Superelevation,
    Junction,
    Template,
    Corridor,
    Volume,
    Haul,
    Guardrail,
    Culvert,
    Wall,
    Platform,
    SheetSet,
    Pay,
    /// A utility network a pipe run profile was drawn from (docs/adr/3560).
    Network,
    Drawing,
    Sheet,
}

/// How a drawing is made again: its command and the version that draws it again (a record keeps no version; every
/// drawing command is v1), its input as kept (its main source by id), how many objects it wrote and how many of them
/// are there now, and the stamp of its objects as written (16 hex digits).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadDrawingRecipe {
    pub command: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: u32,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    #[cfg_attr(feature = "schema", schemars(with = "std::collections::BTreeMap<String, serde_json::Value>"))]
    pub input: BTreeMap<String, serde_json::Value>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub written: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    pub digest: String,
    /// The version that drew it, as its record keeps it (docs/adr/2840); none: a record from before schema 209.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub made: Option<u32>,
    /// The drawing's revision its write made, as its record keeps it (docs/adr/2840).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
}

/// A node: what it is, its id and name, how it stands and why (a layer: current, or gone; a sheet: stale when it shows
/// a drawing not up to date); a drawing's recipe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadGraphNode {
    pub kind: RoadGraphKind,
    pub id: String,
    pub name: String,
    pub state: RoadNodeState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub why: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recipe: Option<RoadDrawingRecipe>,
}

/// What an edge says: a layer builds a surface; a row is made on another (a profile on its alignment, a pad on its
/// surface, a volume on its corridor); a drawing was made from a source; a sheet's map shows a drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoadEdgeKind {
    Builds,
    Basis,
    Source,
    Shows,
}

/// An edge from a node to another by their places: what it says; a source's whether it is the main one (what a
/// rebuild needs), its stamp then and now (none: gone); a map's name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadGraphEdge {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub from: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub to: u32,
    pub kind: RoadEdgeKind,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub main: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kept: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub now: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub map: Option<String>,
}

/// Output of `road.project.graph` v1: the nodes (the surfaces' layers, the tree's rows in its order, the sheets that
/// show a drawing in the book's order) and the edges; how many drawings, how many of them are not up to date (stale,
/// missing, edited, gone), how many sheets and how many of them are stale; the project's system (`EPSG:5256` and its
/// name; none: a local project), epoch and heights in words; whether it was deep; the revision read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectGraph {
    pub nodes: Vec<RoadGraphNode>,
    pub edges: Vec<RoadGraphEdge>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub drawings: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outdated: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub sheets: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub blocked: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub epsg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub heights: Option<String>,
    pub deep: bool,
    pub revision: String,
}
