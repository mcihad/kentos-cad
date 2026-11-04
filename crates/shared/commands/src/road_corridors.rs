//! KentOS Yol's corridor commands (plan/road.md Y7, docs/adr/0905): `road.corridors.edit` writes, rebinds or removes a
//! corridor (on an alignment, its profile, surface and superelevation schedule, its swath, how often it is sampled and
//! its regions each with a template); `road.corridors.query` builds every corridor, or one, now: its stations, the
//! stretches not solved, its feature lines and its top surface. The geometry is `kentos-road`'s; a corridor is a
//! project setting kept in the undo history, its result is built on every read. The desktop's handlers are
//! `crates/native/application/src/road_corridors*.rs`; they and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.corridors.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{CorridorRefineDef, CorridorRegionDef, RoadEditOperation};

pub const ROAD_CORRIDORS_EDIT: &str = "road.corridors.edit";
pub const ROAD_CORRIDORS_EDIT_VERSION: u32 = 1;
pub const ROAD_CORRIDORS_QUERY: &str = "road.corridors.query";
pub const ROAD_CORRIDORS_QUERY_VERSION: u32 = 1;
pub const ROAD_CORRIDORS_DRAW: &str = "road.corridors.draw";
pub const ROAD_CORRIDORS_DRAW_VERSION: u32 = 1;

/// Input of `road.corridors.edit` v1: the corridor by its id or name (absent: a new one). A new one needs `name`,
/// `alignment`, `profile`, `surface`, `left`, `right`, `tangent`, `curve` and `regions`; it holds its alignment's
/// present revision. `superelevation` "" takes the schedule away, `sag` 0 the chord tolerance, `verticalSag` 0 the
/// vertical curves' chord height, a `refine` whose `least` is 0 the sampling check (docs/adr/0939).
///
/// Refusals, in this order: `not_a_road_project`, `unknown_corridor` (remove or rebind one not in the project),
/// `unknown_alignment`, `unknown_road_profile`, `unknown_terrain`, `unknown_superelevation`, `unknown_road_template`,
/// `invalid_road_corridor` (a new one without its fields, set with nothing to set, the definition by `road_problem`,
/// regions overlapping, empty or off the alignment, a station written off it, more stations than a corridor samples),
/// `corridor_in_use` (remove one an earthwork volume uses),
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corridor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alignment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tangent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub curve: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sag: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile_points: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub critical: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub regions: Option<Vec<CorridorRegionDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub vertical_sag: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub refine: Option<CorridorRefineDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A station a corridor samples: the alignment's own, its km, and why (`region`, `geometry`, `critical`, `profile`,
/// `user`, `curve`, `interval`, `vertical`: a vertical curve's division, `refine`: added where the section between
/// strayed, docs/adr/0939).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorStation {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub km_text: String,
    pub why: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
}

/// A stretch of a side not solved: the region, the side (`left`, `right`), why (`noSection`, `open`) and its stations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorStretch {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
    pub side: String,
    pub why: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
}

/// A feature line: its code (`axis`, `left.c1` …, `left.hinge`, `left.ditchToe`, `left.ditchBottom`,
/// `left.daylight`), its region, its first and last station, its 3D length and, when asked, its points (station, x, y,
/// z).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorFeature {
    pub code: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 4]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub points: Vec<[f64; 4]>,
}

/// A corridor as it is now: its sources, whether its alignment changed since, its stations, the gaps between its
/// regions, the stretches not solved, where consecutive sections cross in plan, its feature lines, its top surface's
/// triangle count and plan area; why it does not build when it does not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorReport {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub alignment: String,
    pub profile: String,
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<String>,
    pub stale: bool,
    pub stations: Vec<CorridorStation>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub gaps: Vec<[f64; 2]>,
    pub unsolved: Vec<CorridorStretch>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub folded: Vec<[f64; 2]>,
    pub features: Vec<CorridorFeature>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
    /// How the sampling was checked, when the corridor asks for it (docs/adr/0939 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sampling: Option<CorridorSampling>,
}

/// The largest strays measured (docs/adr/0939 §2): the plan distance and the height a section strayed from the join of
/// its neighbours (m), each with the station of that section and the feature line's code; none where nothing was
/// measured.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorStray {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub plan: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub plan_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plan_code: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub height_code: Option<String>,
}

/// An interval refining left as it is (docs/adr/0939 §2): its region and stations, why (`change`: the section changes
/// kind within it, found to the least step; `least`: it still strays at the least step; `budget`: not measured, the
/// stations allowed are used up) and how far its middle strayed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorLeft {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub region: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    pub why: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub plan: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: f64,
}

/// A corridor's sampling as it was checked (docs/adr/0939): the middles built and measured, the stations added, the
/// largest strays of the first intervals (`before`) and of the intervals at the end (`after`: those found within and
/// those left at the least step without a change of kind), the changes of kind found, and the intervals left.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorSampling {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub measured: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    pub before: CorridorStray,
    pub after: CorridorStray,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changes: u32,
    pub left: Vec<CorridorLeft>,
}

/// A corridor in a list: its id, name, alignment, revision and whether its alignment changed since.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorListed {
    pub id: String,
    pub name: String,
    pub alignment: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub stale: bool,
}

/// Output of `road.corridors.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsWritten {
    /// The corridor written, built; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corridor: Option<CorridorReport>,
    pub corridors: Vec<CorridorListed>,
    pub revision: String,
}

/// Input of `road.corridors.query` v1: every corridor built, or one (`corridor`: its id or name); `points` gives the
/// feature lines' points, `triangles` the top surface's triangles (one corridor only).
///
/// Refusals: `not_a_road_project`, `unknown_corridor`, `invalid_road_corridor` (`points` or `triangles` without a
/// corridor). Warnings: `corridor_stale`, `corridor_unsolved`, `corridor_folded`, `corridor_gap`,
/// `corridor_surface_gone`, `corridor_superelevation_gone`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corridor: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub points: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub triangles: bool,
}

/// Output of `road.corridors.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsQueried {
    pub corridors: Vec<CorridorReport>,
    /// The top surface's triangles (x, y, z each corner), counterclockwise in plan; asked only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[[f64; 3]; 3]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub triangles: Vec<[[f64; 3]; 3]>,
}

/// Input of `road.corridors.draw` v1: the corridor (its id or name) drawn on a layer as its slope plan (docs/adr/0905
/// §6): its feature lines but the axis as polylines with their elevations, the slopes' ticks every `tick` metres of
/// station from the top of the slope (the daylight in a cut, the hinge in a fill) toward its foot, long and short in
/// turn, and the axis elevation (red) at each station solved, `height` paper millimetres high (default 2.5).
///
/// Refusals: `not_a_road_project`, `unknown_corridor`, `invalid_road_corridor` (the corridor does not build),
/// `invalid_label` (a tick or a height not above 0), `no_terrain` (no section solved), `layer_not_found`,
/// `not_a_layer`, `layer_locked`, `invalid_revision`, `revision_conflict`. Warnings: the corridor's (`corridor_stale`,
/// `corridor_unsolved`, `corridor_folded`, `corridor_gap`, `corridor_superelevation_gone`), `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsDraw {
    pub corridor: String,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tick: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.corridors.draw` v1: how many feature lines, slope ticks and elevations it drew, their ids.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsDrawn {
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ticks: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub elevations: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tick: f64,
    pub uids: Vec<String>,
    pub revision: String,
}

pub const ROAD_CORRIDORS_WATER: &str = "road.corridors.water";
pub const ROAD_CORRIDORS_WATER_VERSION: u32 = 1;

/// Input of `road.corridors.water` v1 (docs/adr/0923; ROAD-DEVER-09): water on a corridor's platform as built now: its
/// lane and shoulder triangles flatter than `least` (rise over run, the user's; where the grade and the crossfall
/// leave water standing), the stations where a side's first lane's crossfall passes through zero, and the low points
/// along the platform's edges (the axis and the components' edges). Reads only.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_corridor`, `invalid_platform_water` (least not above 0),
/// and the corridor's when it no longer builds. Warnings: `corridor_flat` and the corridor's own (stale, unsolved,
/// folded …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsWater {
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub least: f64,
}

/// A stretch of flat platform: its region, its first and last station, the plan area flatter than the least and the
/// least slope found (rise over run).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformFlat {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub least: f64,
}

/// A station where a side's first lane's crossfall is zero.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrossfallZero {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
    pub right: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
}

/// A low point on a platform edge: its feature code, region, station and place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformLow {
    pub code: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub z: f64,
}

/// Output of `road.corridors.water` v1: the corridor's id, the least asked, the flat triangles' count and plan area, the
/// flat stretches, the zero crossfall stations and the low points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsWatered {
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub least: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub flat_triangles: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub flat_area: f64,
    pub flats: Vec<PlatformFlat>,
    pub zeros: Vec<CrossfallZero>,
    pub lows: Vec<PlatformLow>,
}
