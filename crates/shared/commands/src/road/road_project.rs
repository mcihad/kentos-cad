//! KentOS Yol's project tree (plan/road.md Y13, docs/adr/0911; ROAD-MOD-04, 06): `road.project.query` reads a road
//! project's sources and what its commands drew as one tree, each with how it stands now and why. The desktop's
//! handler is `crates/native/application/src/road_project*.rs`; it and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.project.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const ROAD_PROJECT_QUERY: &str = "road.project.query";
pub const ROAD_PROJECT_QUERY_VERSION: u32 = 1;
pub const ROAD_PROJECT_REBUILD: &str = "road.project.rebuild";
pub const ROAD_PROJECT_REBUILD_VERSION: u32 = 1;
pub const ROAD_PROJECT_EDIT: &str = "road.project.edit";
pub const ROAD_PROJECT_EDIT_VERSION: u32 = 1;

/// Input of `road.project.query` v1: `deep` makes the surfaces and corridors and computes the approved volumes, so the
/// tree also says which do not build (`unbuildable`), which leave stretches unsolved (`unsolved`) and which approved
/// volumes changed (`changed`); without it only the definitions and the surfaces' sources are read.
///
/// Refusals: `not_a_road_project`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectQuery {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub deep: bool,
}

/// What a row of the project tree is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoadNodeKind {
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
}

/// How a row stands: up to date; a surface never accepted; its sources changed; a surface that does not build; a
/// corridor with unsolved stretches; an approved volume as approved, or its sources changed; a drawing some of whose
/// objects are gone, whose objects were changed by hand, or whose source is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoadNodeState {
    Current,
    New,
    Stale,
    Unbuildable,
    Unsolved,
    Approved,
    Changed,
    Missing,
    Edited,
    Gone,
}

/// A row of the project tree: what it is, its id and name, its parent's index (none: a top row), how it stands and
/// why in sentences; a drawing's command and how many of its objects the drawing holds now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadNode {
    pub kind: RoadNodeKind,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub parent: Option<u32>,
    pub state: RoadNodeState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub why: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: Option<u32>,
}

/// Output of `road.project.query` v1: the rows in the tree's order (a parent before its children), how many rows are
/// stale, how many have a problem (unbuildable, unsolved, changed, missing, edited, gone), how many drawings could be
/// drawn again (stale or missing), whether it was deep, whether drawing again by itself is on, whether drawing again
/// writes only what changed (docs/adr/4740), and the revision read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectTree {
    pub nodes: Vec<RoadNode>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub problems: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rebuildable: u32,
    pub deep: bool,
    pub auto_rebuild: bool,
    pub regional: bool,
    pub revision: String,
}

/// Input of `road.project.rebuild` v1 (docs/adr/0911 §3): the drawings by id to draw again; absent, every drawing that
/// is stale or missing objects. A drawing changed by hand is drawn again only when named and `edited` says so (its
/// changes are lost). Each is drawn again with the input it was drawn with: its objects removed, new ones written on
/// its layer, its sources' stamps and objects kept anew; one undo step “Yeniden üret”.
///
/// `regional` (docs/adr/4320; plan/map-cad.md MAP-14) writes only what changed: each drawing is made again on a copy
/// first; one whose objects come out as they are is not written (its record takes its sources' stamps now). Otherwise,
/// in the command's order, a new object takes the first old one exactly alike not taken yet and stays where that is,
/// unless an object before it in the order is written, or stays from a later place, and the two could draw another
/// picture in the other order (docs/adr/4740): only lines and polylines without a label and of one colour and
/// transparency draw the same picture in any order. The old objects left are removed, the new ones left written at the
/// drawing's end in its order.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_drawing`,
/// `drawing_source_gone` (a named drawing whose main source is gone), `drawing_edited` (a named drawing changed by hand
/// without `edited`), and the drawing commands' own (a layer gone or locked, a source that no longer draws).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectRebuild {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawings: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub edited: bool,
    /// Write only what changed (docs/adr/4320); absent: every drawing is drawn whole, as before.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub regional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// A drawing drawn again: its id and command, how many objects it removed and its objects now (the drawing's order).
/// Asked `regional` (docs/adr/4320): `kept`, the objects left as they were, and, where a line drawing kept some or wrote
/// any, `region`, the box (min x, min y, max x, max y) of the lines it removed and wrote: where the drawing changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadDrawingRebuilt {
    pub drawing: String,
    pub command: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub region: Option<[f64; 4]>,
    pub uids: Vec<String>,
}

/// A drawing left as it is, and why: changed by hand (`edited`), its main source gone (`gone`), or it does not draw now
/// (`refused`, the command's sentence).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadDrawingLeft {
    pub drawing: String,
    pub why: String,
    pub message: String,
}

/// Output of `road.project.rebuild` v1: the drawings drawn again in order, the ones left, and the revision after.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectRebuilt {
    pub rebuilt: Vec<RoadDrawingRebuilt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RoadDrawingLeft>>", optional))]
    pub left: Vec<RoadDrawingLeft>,
    pub revision: String,
}

/// Input of `road.project.edit` v1 (docs/adr/0911 §3): `autoRebuild` turns drawing the stale drawings again by itself, in
/// the step of the write that left them stale, on or off; `regional` (docs/adr/4740) chooses whether drawing again, by
/// itself or from the project tree, writes only what changed (docs/adr/4320; the project's first choice) or draws each
/// drawing whole. One undo step: “Kendiliğinden yeniden üret”, “Yalnız değişeni yaz” or, both changed, “Yeniden üretim”.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `nothing_to_set` (neither
/// given, or what is given is already so: at `autoRebuild` when it is given, else at `regional`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectEdit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub auto_rebuild: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub regional: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// Output of `road.project.edit` v1: drawing again by itself now, whether drawing again writes only what changed
/// (docs/adr/4740), and the revision after.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProjectEdited {
    pub auto_rebuild: bool,
    pub regional: bool,
    pub revision: String,
}
