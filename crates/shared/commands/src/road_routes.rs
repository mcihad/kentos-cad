//! KentOS Yol's preliminary routes (plan/road.md Y12.1, docs/adr/0910 §1; ROAD-ALIGN-13): `road.routes.find` finds the
//! least-cost path between two places over a terrain surface's grid, steps steeper than the grade asked not taken and
//! areas to avoid not entered; `road.routes.draw` draws it as a 3D polyline on a layer, to be fitted as an alignment
//! (`road.alignments.fit`) and checked there. The limits are the user's; KentOS gives none. The geometry is
//! `kentos-road`'s `route`. The desktop's handlers are `crates/native/application/src/road_routes*.rs`; they and the
//! Python SDK pass the shared cases in `fixtures/commands/v1/road.routes.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::Vec2;

pub const ROAD_ROUTES_FIND: &str = "road.routes.find";
pub const ROAD_ROUTES_FIND_VERSION: u32 = 1;
pub const ROAD_ROUTES_DRAW: &str = "road.routes.draw";
pub const ROAD_ROUTES_DRAW_VERSION: u32 = 1;

/// What a route is asked with: the terrain surface (id or name), the two places, the grid's cell and the margin
/// around the places, the steepest step (percent), the cost of a metre risen or fallen (in metres of length), the
/// layers whose areas are avoided and the plan tolerance the line is simplified within (none: every node).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RouteAsk {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub margin: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub grade: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub climb: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub avoid: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
}

/// Input of `road.routes.find` v1.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_terrain`, `invalid_route` (a cell, margin, grade or climb
/// out of bounds, a grid of more than two million nodes, a layer to avoid not in the drawing, the surface not
/// building), `route_blocked` (the start or the end off the terrain or in an area to avoid), `route_unreached` (no path
/// within the grade).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoutesFind {
    #[serde(flatten)]
    pub ask: RouteAsk,
}

/// A place of a route: plan and the terrain's height there (rounded to the micrometre).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoutePoint {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
}

/// Output of `road.routes.find` v1: the route's points (the start and the end where asked, the grid's nodes
/// between, simplified within the tolerance), its plan length, the height it rises and falls in all, its steepest
/// stretch (percent, between the points kept), its cost, the grid's nodes and those settled.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RouteFound {
    pub points: Vec<RoutePoint>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub climb: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub steepest: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cost: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub nodes: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub settled: u32,
}

/// Input of `road.routes.draw` v1: the route found as `road.routes.find` finds it, drawn on `layerId` as one 3D
/// polyline.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `road.routes.find`'s, `layer_not_found`,
/// `not_a_layer`, `layer_locked`. Warnings: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoutesDraw {
    #[serde(flatten)]
    pub ask: RouteAsk,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.routes.draw` v1, and its plan: the route and the object written (in the plan, none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoutesDrawn {
    pub route: RouteFound,
    pub uids: Vec<String>,
    pub revision: String,
}
