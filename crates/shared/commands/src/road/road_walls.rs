//! KentOS Yol's retaining walls (plan/road.md Y11.3, docs/adr/0909 §3; ROAD-DRAIN-11): `road.walls.edit` writes,
//! rebinds or removes a wall (a cantilever wall standing at a corridor side's hinge between two stations, its cross the
//! project's own); `road.walls.query` reads every one, or one, now: at each station its face's place, the road's edge
//! and the ground there, its top, its footing and its stem's height and cross area, and along it its length, face area,
//! concrete and highest stem; `road.walls.draw` draws its face and footing in plan. The geometry is `kentos-road`'s; a
//! wall is a project setting kept in the undo history, its reading is computed on every read. The desktop's handlers
//! are `crates/native/application/src/road_walls*.rs`; they and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.walls.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{RoadEditOperation, WallShapeDef, WallSide};

pub const ROAD_WALLS_EDIT: &str = "road.walls.edit";
pub const ROAD_WALLS_EDIT_VERSION: u32 = 1;
pub const ROAD_WALLS_QUERY: &str = "road.walls.query";
pub const ROAD_WALLS_QUERY_VERSION: u32 = 1;
pub const ROAD_WALLS_DRAW: &str = "road.walls.draw";
pub const ROAD_WALLS_DRAW_VERSION: u32 = 1;

/// Input of `road.walls.edit` v1: the wall by its id or name (absent: a new one). A new one needs `name`, `corridor`,
/// `side`, `from`, `to`, `step` and `shape`. Rebind takes the corridor's present revision. `highest` and `material`
/// null take them off.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_retaining_wall` (rebind or remove one not in the project),
/// `unknown_corridor`, `invalid_retaining_wall` (a new one without its fields, set with nothing to set, the definition by
/// `road_problem`: stations out of order, a step, sizes or batter out of bounds, a toe and stem wider than the footing;
/// stations off the corridor or without a section, no ground under the face, a corridor that does not build),
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wall: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corridor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<WallSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<WallShapeDef>,
    /// The highest stem asked; null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub highest: Option<Option<f64>>,
    /// The wall's material; null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub material: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A wall at a station: its km, its face's offset (right positive) and plan place, the road's edge and the ground
/// there, its stem's top, its footing's top, its stem's height, its cross's concrete area and whether it holds a fill.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub km_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub road: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub ground: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub top: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub footing: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    pub fill: bool,
}

/// A wall as read now: its names, whether its corridor changed since (`stale`), its side and stations, its rows, its
/// plan length along its face, its stem's face area, its concrete, and its highest stem and that stem's station.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallReport {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub stale: bool,
    pub side: WallSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub rows: Vec<RetainingWallRow>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub face: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub concrete: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub highest: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub highest_at: f64,
}

/// A wall in the project's list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallListed {
    pub id: String,
    pub name: String,
    pub corridor: String,
    pub side: WallSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// Output of `road.walls.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallsWritten {
    /// The wall written, read; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wall: Option<RetainingWallReport>,
    pub walls: Vec<RetainingWallListed>,
    pub revision: String,
}

/// Input of `road.walls.query` v1: every wall read, or one (`wall`: its id or name).
///
/// Refusals: `not_a_road_project`, `unknown_retaining_wall`, `invalid_retaining_wall` (the one asked does not read). Warnings:
/// `retaining_wall_stale`, `retaining_wall_high`, and the corridor's own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wall: Option<String>,
}

/// Output of `road.walls.query` v1: the walls read (one that does not read with no rows: `length` 0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallsQueried {
    pub walls: Vec<RetainingWallReport>,
}

/// Input of `road.walls.draw` v1: the wall drawn on `layerId`: its face along its stations and its footing's outline;
/// with `labels` a text at its middle saying its km range and highest stem.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_retaining_wall`,
/// `invalid_retaining_wall`, `layer_not_found`, `not_a_layer`, `layer_locked`. Warnings: `layer_hidden`, and the read's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallsDraw {
    pub wall: String,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub labels: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.walls.draw` v1, and its plan: the wall, its length and the objects written (in the plan, none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallsDrawn {
    pub wall: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    pub uids: Vec<String>,
    pub revision: String,
}
