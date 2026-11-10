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

use crate::{LineType, RoadNodeState};

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

/// A drawing's layer as the recipe's project had it (docs/adr/3640): its id and name, and the look a layer made from it
/// takes (colour, built-in line type, plot line weight in mm; a project line type comes as its nearest built-in one).
/// `road.project.produce` finds the layer by this name; where none is, it adds one with this look. Docs/adr/4040: the
/// groups it was in, its whole style and the project line type that style names, as its project defines it; a layer
/// added from it goes into those groups with that style. Docs/adr/4440: how it was composed into the drawing (its
/// opacity and blending mode), what it held and whether it was printed; a layer added from it takes them too. A recipe
/// from before has none of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadRecipeLayer {
    pub id: String,
    pub name: String,
    pub color: String,
    pub line_type: LineType,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: f64,
    /// The names of the groups it was in, from the top of the tree; empty: at the top.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub group: Vec<String>,
    /// Its whole style (colour, line types, weight, fill, point, labels …); its colour, line type and weight are the
    /// fields above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<crate::LayerStyle>,
    /// The project line type its style names (`style.linePattern`), as its project defines it; none when the style
    /// names none or its project has no such line type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type_def: Option<crate::LineTypeDef>,
    /// How it was composed into the drawing (Katman çizimi, docs/adr/4440): its opacity and blending mode, never a mask
    /// (a mask cuts into its project's other layers by their ids); none: opaque and normal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub compose: Option<crate::LayerCompose>,
    /// What it held (Katman türü): points, lines, areas or annotation; none: any kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<crate::LayerGeometry>,
    /// Yazdır: `false`, it was drawn on screen but not printed; none: printed. Only `false` is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plot: Option<bool>,
    /// Its fields (Alanlar, docs/adr/0199, 4860), in their order, as its project keeps them but for a bound table's
    /// column (the binding is its project's). A field linked to one of the project's value lists keeps the list's id;
    /// the list is in `domains`. Empty: no fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<crate::LayerField>>"))]
    pub fields: Vec<crate::LayerField>,
    /// The project's value lists its fields link (Değer listeleri, docs/adr/0430, 4860), as its project defines them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<crate::FieldDomain>>"))]
    pub domains: Vec<crate::FieldDomain>,
    /// Its subtypes (Alt türler, docs/adr/0422, 4860): its field that sorts the records and what each kind narrows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub subtypes: Option<crate::LayerSubtypes>,
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
    /// Its layer's definition (docs/adr/3640); none: a recipe from before, or its layer is gone (it goes by its id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<RoadRecipeLayer>,
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
