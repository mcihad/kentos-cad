//! KentOS Yol's superelevation commands (plan/road.md Y6, docs/adr/0904): `road.superelevations.edit` writes, rebinds
//! or removes a superelevation schedule (on an alignment, the normal cross slopes, from the curves' rates, runoffs and
//! shares on the tangent or from rows of stations); `road.superelevations.query` reads every schedule's report, or one's
//! cross slopes and widening at stations. The geometry is `kentos-road`'s; a schedule is a project setting kept in the
//! undo history: every write is one undo step. The desktop's handlers are
//! `crates/native/application/src/road_superelevations*.rs`; they and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.superelevations.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{RoadEditOperation, SuperelevationMethod};

pub const ROAD_SUPERELEVATIONS_EDIT: &str = "road.superelevations.edit";
pub const ROAD_SUPERELEVATIONS_EDIT_VERSION: u32 = 1;
pub const ROAD_SUPERELEVATIONS_QUERY: &str = "road.superelevations.query";
pub const ROAD_SUPERELEVATIONS_QUERY_VERSION: u32 = 1;

/// Input of `road.superelevations.edit` v1: the schedule by its id or name (absent: a new one). A new one needs `name`,
/// `alignment`, `normalLeft`, `normalRight` and `method`; it holds its alignment's present revision. Slopes are percent,
/// outward from the axis, falling outward negative.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_superelevation` (remove or rebind one not in the project),
/// `unknown_alignment`, `invalid_superelevation` (a new one without its fields, set with nothing to set, the definition
/// by `road_problem`, a schedule that does not build: a PI without a curve, a rate, runoff, share or width out of range,
/// a rate below the normal slope, a full rate that does not fit on its curve, two curves' transitions overlapping, a
/// transition off the alignment, a PI twice, rows not in increasing station or off the alignment),
/// `superelevation_in_use` (remove while a sample line set uses it), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alignment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub normal_left: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub normal_right: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<SuperelevationMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A critical station of a curve's transition: `normalCrown`, `levelCrown`, `reverseCrown` or `full`, on the way in or
/// out (`exit`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperCritical {
    pub pi: String,
    pub mark: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    pub km_text: String,
    pub exit: bool,
}

/// A superelevation schedule as it is now: each side's break points (station, slope percent) and widening (station,
/// metres), the critical stations; why it no longer builds when it does not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationReport {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub alignment: String,
    /// The alignment changed since it was made or rebound.
    pub stale: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub normal_left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub normal_right: f64,
    /// (station, slope percent) pairs.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/%")))]
    pub left: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/%")))]
    pub right: Vec<[f64; 2]>,
    /// (station, width) pairs, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left_widening: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right_widening: Vec<[f64; 2]>,
    pub critical: Vec<SuperCritical>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
}

/// Output of `road.superelevations.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsWritten {
    /// The schedule written; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<SuperelevationReport>,
    pub superelevations: Vec<SuperelevationReport>,
    pub revision: String,
}

/// Input of `road.superelevations.query` v1: every schedule's report, or one's (`superelevation`: its id or name) with
/// its cross slopes and widening at `stations` (the alignment's own stations).
///
/// Refusals: `not_a_road_project`, `unknown_superelevation`, `invalid_places` (stations without a schedule),
/// `station_outside`, `invalid_superelevation` (the schedule no longer builds). Warnings: `superelevation_stale`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: Vec<f64>,
}

/// The cross slopes (percent) and widening (metres) of both sides at a station.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperAt {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    pub km_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub right: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left_widening: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right_widening: f64,
}

/// Output of `road.superelevations.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsQueried {
    pub superelevations: Vec<SuperelevationReport>,
    pub at: Vec<SuperAt>,
}
