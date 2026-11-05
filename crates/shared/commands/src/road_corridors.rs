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

pub const ROAD_CORRIDORS_DAYLIGHTS: &str = "road.corridors.daylights";
pub const ROAD_CORRIDORS_DAYLIGHTS_VERSION: u32 = 1;

/// Input of `road.corridors.daylights` v1 (plan/road.md Y45, docs/adr/0951; ROAD-COR-10): how each section's slopes
/// meet the terrain as the corridor is built now — the daylight with the slope's crossings of the terrain on the swath
/// and how far the daylight moves per metre of terrain height (1/|m − g|), or why it has none: no terrain at the
/// hinge, the terrain stopping before the slope meets it, the swath ending first. With `sensitive` (m per m, above 0),
/// the daylights moving more than that are warned of.
///
/// Refusals: `not_a_road_project`, `unknown_corridor`, `invalid_daylight_check` (`sensitive` not above 0), and the
/// corridor's own (`invalid_corridor`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsDaylights {
    pub corridor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub sensitive: Option<f64>,
}

/// A side's daylight (docs/adr/0951): `hit`, `hinge`, `gap` or `swath`; for a hit its offset (the section's sign, left
/// negative), whether in a cut, the slope's crossings with the terrain, the next one's distance further out and the
/// sensitivity (m per m of terrain height; none where the slope lies along the terrain); for a gap where the terrain
/// stops, for a swath where it ends (offsets).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DaylightSide {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cut: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub crossings: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub next: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub sensitivity: Option<f64>,
}

/// A section's daylights: its station, km and region, left and right.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DaylightStation {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub km_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
    pub left: DaylightSide,
    pub right: DaylightSide,
}

/// How many sides end each way (docs/adr/0951), the hits crossing the terrain more than once, those over the
/// sensitivity asked, and the sections not built.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DaylightCounts {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub hit: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub hinge: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub gap: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub swath: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub multiple: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub sensitive: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unbuilt: u32,
}

/// Output of `road.corridors.daylights` v1 (docs/adr/0951): the corridor, the threshold asked, each built section's
/// daylights in station order, the counts, and the most sensitive daylight (station, side, m per m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorDaylights {
    pub corridor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub sensitive: Option<f64>,
    pub stations: Vec<DaylightStation>,
    pub counts: DaylightCounts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub worst: Option<DaylightWorst>,
}

/// The most sensitive daylight: its station, km, side (`left` or `right`) and sensitivity (m per m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DaylightWorst {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub km_text: String,
    pub side: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub sensitivity: f64,
}

pub const ROAD_CORRIDORS_PROTECTED: &str = "road.corridors.protected";
pub const ROAD_CORRIDORS_PROTECTED_VERSION: u32 = 1;

/// Input of `road.corridors.protected` v1 (plan/road.md Y46, docs/adr/0952; ROAD-GRADE-06): a corridor's footprint as
/// built now against the protected areas — the area objects on `layers` (1 to 20 layer ids), each part with its holes:
/// how much of each the top surface covers, the sections whose footprint (slope end to slope end) reaches into it and
/// the longest reach.
///
/// Refusals: `not_a_road_project`, `unknown_corridor`, `invalid_protected_check` (no layer or over 20, a layer not in
/// the drawing).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsProtected {
    pub corridor: String,
    pub layers: Vec<String>,
}

/// A protected area against the footprint (docs/adr/0952): the area object's id, its part and layer, its size, how
/// much the top surface covers (m²) and its share; the sections reaching into it, their first and last station, and
/// the longest reach (a section's footprint inside the area, m) with where.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProtectedArea {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub entity: u64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub part: u32,
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub size: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub share: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub sections: u32,
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
    pub from_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to_text: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub reach: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub reach_at: Option<f64>,
    /// The area's outer ring and holes in plan, for a picture.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rings: Vec<Vec<[f64; 2]>>,
}

/// Output of `road.corridors.protected` v1 (docs/adr/0952): the corridor, its footprint's area (m², the top surface's
/// triangles in plan), the areas checked (every one, covered or not), the area covered in all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorProtected {
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub footprint: f64,
    pub areas: Vec<ProtectedArea>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
}

pub const ROAD_CORRIDORS_PREFLIGHT: &str = "road.corridors.preflight";
pub const ROAD_CORRIDORS_PREFLIGHT_VERSION: u32 = 1;

/// Input of `road.corridors.preflight` v1 (plan/road.md Y50, docs/adr/0956; ROAD-COR-13): a corridor checked before
/// its surface and volumes are trusted — where it folds (each cross's margin `1 + κ·o` against its curve, and the
/// sampled sections crossing), where its top surface lies over itself (strips that share no station overlapping in
/// plan) and where its regions do not meet (the crosses at a region's end and the next's start, code by code).
/// `margin` warns of crosses below it too (absent: only those that fold, at or below 0); `tolerance` (m) is how far
/// apart two crosses at a seam may be (absent: 1 mm). Reads only.
///
/// Refusals: `not_a_road_project`, `unknown_corridor`, `invalid_preflight` (a margin not in [0, 1), a tolerance not in
/// (0, 10] m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorsPreflight {
    pub corridor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub margin: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

/// A stretch of crosses whose margin is below the limit: region, stations and km, the least margin, where, the offset
/// of the point giving it and the curve's radius there (none on a tangent).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorTight {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub least: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub at: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub radius: Option<f64>,
}

/// A stretch of sampled sections crossing each other in plan (no surface made there): region and stations with km.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorFold {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
}

/// Two parts of the top surface over each other in plan: their stations with km, the area, and the least and the most
/// height between the two surfaces over it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorOverlap {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub first: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub second: [f64; 2],
    pub first_text: [String; 2],
    pub second_text: [String; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub least: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub most: f64,
}

/// Where one region meets the next: the station and km, the regions, the largest plan shift and height step between
/// the codes both crosses have and which code gives each, the codes only one has, and whether they meet within the
/// tolerance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorSeam {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub km: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub before: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub after: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub shift: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shift_code: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub step: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub step_code: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub only_before: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub only_after: Vec<String>,
    pub meets: bool,
}

/// Output of `road.corridors.preflight` v1: the corridor, how many crosses, strips and triangles were checked, the
/// least margin and where, the tight stretches, the folds, the overlaps and the seams, and whether all is clear.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CorridorPreflight {
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub crosses: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub strips: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub least: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_at: Option<f64>,
    pub tight: Vec<CorridorTight>,
    pub folds: Vec<CorridorFold>,
    pub overlaps: Vec<CorridorOverlap>,
    pub seams: Vec<CorridorSeam>,
    pub clear: bool,
}
