//! KentOS Yol's graded pads (plan/road.md Y14, docs/adr/0927; ROAD-GRADE-01…03, 08…10): `road.platforms.edit` writes or
//! removes a pad (a plane inside a closed boundary, its cut and fill slopes run out to a terrain surface);
//! `road.platforms.query` reads every one, or one, now: its slope lines and their side, the daylight line, the open
//! lines, the pad's and the footprint's areas, cut and fill against the terrain, and on asking the vertical offset that
//! balances them; `road.platforms.draw` draws its boundary, daylight line and slope lines in plan. The geometry is
//! `kentos_road::grading`'s; a pad is a project setting kept in the undo history, its reading is computed on every
//! read. The desktop's handlers are `crates/native/application/src/road_platforms*.rs`; they and the Python SDK pass
//! the shared cases in `fixtures/commands/v1/road.platforms.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::RoadEditOperation;

pub const ROAD_PLATFORMS_EDIT: &str = "road.platforms.edit";
pub const ROAD_PLATFORMS_EDIT_VERSION: u32 = 1;
pub const ROAD_PLATFORMS_QUERY: &str = "road.platforms.query";
pub const ROAD_PLATFORMS_QUERY_VERSION: u32 = 1;
pub const ROAD_PLATFORMS_DRAW: &str = "road.platforms.draw";
pub const ROAD_PLATFORMS_DRAW_VERSION: u32 = 1;

/// Input of `road.platforms.edit` v1: the pad by its id or name (absent: a new one). A new one needs `name`, `corners`,
/// `base`, `cut`, `fill`, `step`, `fan`, `reach` and `surface`; `grade` and `direction` are 0 unless given. Nothing is
/// filled in: the slopes, the spacing, the fan and the reach are the user's (ROAD-GRADE-02).
///
/// Refusals, in this order: `not_a_road_project`, `unknown_platform` (remove one not in the project),
/// `unknown_terrain`, `invalid_platform` (a new one without its fields, a set with nothing to set, rebind (a pad
/// has nothing to rebind), the definition by `road_problem`, a boundary crossing itself or with two corners at one
/// place), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub corners: Option<Vec<[f64; 2]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub base: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub grade: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub direction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub fan: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub reach: Option<f64>,
    /// The terrain surface by its id or name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Which way a slope line goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PlatformSide {
    Cut,
    Fill,
    /// Off the terrain, or no meeting within the reach.
    Open,
}

/// A slope line: where it starts on the pad's edge (x, y, z), where it meets the terrain (none when open), its side,
/// and the corner it starts at or after.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformSlopeLine {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Option<[f64; 3]>,
    pub side: PlatformSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub corner: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub at_corner: bool,
}

/// A pad as read now: its names and surface, its boundary counterclockwise with its heights, its slope lines, the
/// daylight ring, how many lines are open, the pad's and the footprint's plan areas, and cut and fill (m³) with their
/// plan areas against the terrain; the footprint and the volumes none while a line is open, `why` saying so.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformReport {
    pub id: String,
    pub name: String,
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub top: Vec<[f64; 3]>,
    pub lines: Vec<PlatformSlopeLine>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub daylight: Vec<[f64; 3]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub open: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub top_area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub footprint: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub cut_area: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub fill_area: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub why: Option<String>,
}

/// A pad in the project's list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformListed {
    pub id: String,
    pub name: String,
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// Output of `road.platforms.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformsWritten {
    /// The pad written, read; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub platform: Option<PlatformReport>,
    pub platforms: Vec<PlatformListed>,
    pub revision: String,
}

/// The bracket a balance is searched in: the pad's vertical offset from `low` to `high` (m), until cut and fill meet
/// within `tolerance` m³.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformBalanceAsk {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub low: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub high: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub tolerance: f64,
}

/// A balance found: the offset, the base's elevation it gives, the net (cut − fill) there, the bisection's steps and
/// whether it converged. Not written: an alternative the user takes or leaves (ROAD-GRADE-08).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformBalanced {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub net: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub iterations: u32,
    pub converged: bool,
}

/// Input of `road.platforms.query` v1: every pad read, or one (`platform`: its id or name); with `balance` (one pad
/// only) the offset that balances its cut and fill.
///
/// Refusals: `not_a_road_project`, `unknown_platform`, `invalid_platform` (the one asked does not grade; a balance
/// without one pad, its bracket empty, or a tolerance not positive). Warnings: `platform_open`,
/// `platform_no_balance` (the net keeps its sign over the bracket, or an end does not grade whole: no balance given).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub balance: Option<PlatformBalanceAsk>,
}

/// Output of `road.platforms.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformsQueried {
    pub platforms: Vec<PlatformReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub balance: Option<PlatformBalanced>,
}

/// Input of `road.platforms.draw` v1: the pad drawn on `layerId`: its boundary and daylight line as closed 3D
/// polylines and each slope line as a line; with `labels` a text at its middle saying its elevation, cut and fill.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_platform`,
/// `invalid_platform`, `layer_not_found`, `not_a_layer`, `layer_locked`. Warnings: `layer_hidden`, `platform_open` (the
/// open lines drawn to their reach's end are not drawn; the daylight line is not drawn).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformsDraw {
    pub platform: String,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub labels: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.platforms.draw` v1, and its plan: the pad and the objects written (in the plan, none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformsDrawn {
    pub platform: String,
    pub uids: Vec<String>,
    pub revision: String,
}
