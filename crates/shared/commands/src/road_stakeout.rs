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

/// A source a stakeout was computed from, as it is now (docs/adr/1038): its id and revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeoutSource {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// The terrain surface a stakeout was computed on (docs/adr/1038): its id and the digest of the sources it was last
/// accepted from (absent: never accepted).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeoutSurface {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub built: Option<String>,
}

/// An earthwork volume of the stakeout's corridor (docs/adr/1038): its id, revision and whether approved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeoutVolume {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub approved: bool,
}

/// What a stakeout was computed from, each as it is now (docs/adr/1038; ROAD-IO-12): the corridor, its alignment and
/// profile, its regions' templates in order, its superelevation schedule, its terrain surface, and its corridor's
/// earthwork volumes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeoutSources {
    pub corridor: StakeoutSource,
    pub alignment: StakeoutSource,
    pub profile: StakeoutSource,
    pub templates: Vec<StakeoutSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<StakeoutSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<StakeoutSurface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<StakeoutVolume>>", optional))]
    pub volumes: Vec<StakeoutVolume>,
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
    /// What it was computed from (docs/adr/1038).
    pub sources: StakeoutSources,
}

pub const ROAD_STAKEOUT_SEND: &str = "road.stakeout.send";
pub const ROAD_STAKEOUT_SEND_VERSION: u32 = 1;

/// The file a stakeout is sent as: a LandXML string road (the corridor's alignment, its profile and the coded points
/// as strings, what Trimble Access, Leica Captivate and Topcon MAGNET read), or the points to an instrument's
/// coordinate file (docs/adr/0169 §4's writers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StakeoutSendFormat {
    Landxml,
    Gsi16,
    Gsi8,
    Gts7,
    Jobxml,
    Nikon,
    Csv,
}

/// Input of `road.stakeout.send` v1: the stakeout table's points (`road.stakeout.query`'s corridor, range, step and
/// codes; places on the grid) written to `path` as `format` (`.xml` for LandXML, `.gsi`, `.xyz`, `.jxl`, `.raw`,
/// `.csv` for the instrument files). The points are named by number from `first` (1 without it); a code the format
/// cannot carry is left off the point (said). `job` is JobXML's job and Nikon RAW's comment (the corridor's name
/// without it); `stamp` (`YYYY-MM-DDThh:mm:ss`) the file's time (now without it).
///
/// Refusals, in this order: `not_a_road_project`, `unknown_corridor`, `invalid_stakeout` (as the query), an invalid
/// `path` (`invalid_file_path`: empty, relative, not the format's extension, no folder), `file_exists` (without
/// `overwrite`), `invalid_stakeout_send` (no point to send, a stamp that does not read, a job the format cannot
/// carry, a first number not in [1, 99 999 999]), `field_heights_unknown` (docs/adr/1031: the project does not say its
/// heights' system; after the first number and the stamp). Warnings: the query's, `stakeout_code_dropped` (codes left
/// off), `export_losses` (points the format cannot carry; strings of one point). A LandXML file says the project's
/// systems in its `CoordinateSystem`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeoutSend {
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
    pub format: StakeoutSendFormat,
    pub path: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub overwrite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub first: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub job: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stamp: Option<String>,
}

/// A point the format did not carry: its name and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeoutLeft {
    pub name: String,
    pub problem: String,
}

/// Output of `road.stakeout.send` v1: the file, its format, the corridor, the stations read, the points written and
/// those left out, the codes left off, the strings (LandXML), the alignment and the profile written (LandXML), its
/// bytes and SHA-256.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StakeoutSent {
    pub path: String,
    pub format: StakeoutSendFormat,
    pub corridor: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stations: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    pub left: Vec<StakeoutLeft>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub codes_dropped: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub strings: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alignment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}
