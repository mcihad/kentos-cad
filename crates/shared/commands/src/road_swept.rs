//! A design vehicle's swept path (plan/road.md Y17, docs/adr/0915; ROAD-JUNC-06, 07): `road.swept.query` drives a
//! vehicle's front axle along a path (places, a line, polyline or arc of the drawing, or a junction's quadrant: along
//! one leg, round its curb return, out along the other) and reads its axles' and body corners' tracks, how far the
//! rear runs inside the front and the least clearance to obstacles; `road.swept.draw` draws the tracks and, every few
//! samples, the bodies on a layer. The kinematics are `kentos_road::swept`'s; nothing is kept. The vehicle's sizes are
//! the user's (the agency's design vehicle). The desktop's handlers are `crates/native/application/src/road_swept*.rs`;
//! they and the Python SDK pass the shared cases in `fixtures/commands/v1/road.swept.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const ROAD_SWEPT_QUERY: &str = "road.swept.query";
pub const ROAD_SWEPT_QUERY_VERSION: u32 = 1;
pub const ROAD_SWEPT_DRAW: &str = "road.swept.draw";
pub const ROAD_SWEPT_DRAW_VERSION: u32 = 1;

/// The most samples one drive has, and the most obstacles one query reads.
pub const MOST_SWEPT_SAMPLES: usize = 100_000;
pub const MOST_SWEPT_OBSTACLES: usize = 1000;

/// A trailer behind the tractor's kingpin: the kingpin's place behind the tractor's rear axle on its axis (negative:
/// ahead of it), from the kingpin to the trailer's axle, the trailer's body ahead of the kingpin and behind its axle,
/// and its width; m.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SweptTrailerDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub kingpin: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub wheelbase: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub front: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rear: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub width: f64,
}

/// A design vehicle: its wheelbase, its body ahead of the front axle and behind the rear axle, its width (m) and an
/// optional trailer. KentOS gives no vehicle: the sizes are the agency's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SweptVehicleDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub wheelbase: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub front: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rear: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub width: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trailer: Option<SweptTrailerDef>,
}

/// A junction's quadrant as the path: the junction (its id or name), the quadrant (0–3 in `road.junctions.query`'s
/// order), the front axle's distance outward of the curb (`offset`, m; the path runs at the return's radius plus it)
/// and the straight run along each leg before and after the return (`approach`, m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SweptJunctionPath {
    pub junction: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub quadrant: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub approach: f64,
}

/// Input of `road.swept.query` v1: the vehicle, one path (`path`: places; `uid`: a line, an open one-part polyline or
/// an arc of the drawing; `junction`: a junction's quadrant, in a road project), driven from its start (`reverse`: from
/// its end) with the front axle placed every `step` m along its chords (arcs taken as chords no longer than `step`),
/// and the obstacles the clearance is read to (`obstacles`: lines, polylines, arcs, circles of the drawing; with a
/// junction its quadrant's curb is one too).
///
/// Refusals, in this order: `invalid_swept` (no path or more than one, a vehicle size not right, a step not positive,
/// a quadrant over 3, an offset or approach negative, fewer than two places, more than [`MOST_SWEPT_OBSTACLES`]
/// obstacles), `not_a_road_project`, `unknown_junction`, `invalid_junction` (it does not read, or it has no curb
/// returns), `entity_not_found`, `invalid_swept` (an object not of the kinds, more than [`MOST_SWEPT_SAMPLES`]
/// samples). Warnings: `swept_touches` (a body meets an obstacle).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SweptQuery {
    pub vehicle: SweptVehicleDef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub path: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub junction: Option<SweptJunctionPath>,
    #[serde(default)]
    pub reverse: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub obstacles: Vec<String>,
}

/// The least clearance: its distance (0 where a body meets the obstacle), how far the front had gone, the front's
/// place then, which body (0 the tractor, 1 the trailer) and the obstacle (its uid; the junction's id for its curb).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SweptClearance {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub along: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub body: u32,
    pub obstacle: String,
    pub curb: bool,
}

/// Output of `road.swept.query` v1: the front's path length and samples, the front, rear and trailer axles' tracks,
/// the bodies' corners' tracks (front left, front right, rear right, rear left; the trailer's four after the
/// tractor's), the largest off-tracking of the rear axle and of the trailer's (m), and the least clearance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SweptQueried {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub samples: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub front: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rear: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub trailer: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub corners: Vec<Vec<[f64; 2]>>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offtracking: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub trailer_offtracking: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub clearance: Option<SweptClearance>,
}

/// Input of `road.swept.draw` v1: a query's tracks drawn on `layerId`: the front axle's path and the bodies' corners'
/// tracks as polylines, and every `every`th sample's bodies (the first and the last always) as closed polylines; with
/// `labels` the off-tracking and the clearance at the path's end. One undo step “Araç izi çiz”.
///
/// Refusals: the query's, `invalid_swept` (`every` zero), the layer's, `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SweptDraw {
    pub query: SweptQuery,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub every: Option<u32>,
    #[serde(default)]
    pub labels: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.swept.draw` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SweptDrawn {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
