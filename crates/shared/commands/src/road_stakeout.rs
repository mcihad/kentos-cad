//! KentOS Yol's stakeout table (plan/road.md Y10, docs/adr/0908; ROAD-SEC-09): `road.stakeout.query` reads a corridor's
//! coded section points at its stations (or every `step` metres): each point's station, its km, its code, its side and
//! its offset from the axis, its plan coordinates and its elevation; on the grid, or with the offsets as ground lengths
//! (the project's scale and mean height). It writes nothing. The geometry is `kentos-road`'s; the desktop's handler is
//! `crates/native/application/src/road_stakeout.rs`; it and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.stakeout.query.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const ROAD_STAKEOUT_QUERY: &str = "road.stakeout.query";
pub const ROAD_STAKEOUT_QUERY_VERSION: u32 = 1;

/// How the table gives the offsets: as the plan (grid) lengths, or as ground lengths (each over the line's scale and
/// the project's mean height's factor).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StakeoutDistances {
    #[default]
    Grid,
    Ground,
}

/// A point's side of the axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StakeSide {
    Left,
    Axis,
    Right,
}

/// Input of `road.stakeout.query` v1: the corridor by its id or name; its stations between `from` and `to` (its
/// regions' ends without them), or every `step` metres there (the multiples of `step` and the range's ends, in each
/// region); only the points whose code is in `codes` (a whole code such as `left.daylight`, or a code's word such as
/// `daylight` for both sides); the offsets as plan or ground lengths.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_corridor`, `invalid_stakeout` (a range not on the corridor or
/// backwards, a step not in [0.1, 10000] m, a code no template gives, more than 20 000 stations or 200 000 points; a
/// corridor that does not build), `stakeout_no_ground` (ground lengths without the project's mean height, a system or
/// one scale at a point). Warnings: the corridor's own (`corridor_stale`, `corridor_gap`, `corridor_unsolved`,
/// `corridor_folded`, …), `stakeout_unsolved` (stations with no section, which give no points).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeoutQuery {
    pub corridor: String,
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
    pub codes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "is_grid")]
    #[cfg_attr(feature = "ts", ts(as = "Option<StakeoutDistances>", optional))]
    pub distances: StakeoutDistances,
}

fn is_grid(d: &StakeoutDistances) -> bool {
    *d == StakeoutDistances::Grid
}

/// A stakeout point: its station and km, its region, its code and side, its offset from the axis (right positive; on
/// the grid), its plan place and elevation, and its offset as a ground length where the table asks for it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub km_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub region: u32,
    pub code: String,
    pub side: StakeSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub z: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub ground_offset: Option<f64>,
}

/// Output of `road.stakeout.query` v1: the corridor, how the offsets are given, the stations read and the points in
/// station order (a station shared by two regions once for each), left to right.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeoutQueried {
    pub corridor: String,
    pub name: String,
    pub distances: StakeoutDistances,
    /// How the offsets are given, in words.
    pub method: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stations: u32,
    pub rows: Vec<StakeRow>,
}
