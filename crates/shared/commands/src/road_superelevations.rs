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
use crate::{RoadEditOperation, SuperelevationMethod, SuperelevationPivot};

pub const ROAD_SUPERELEVATIONS_EDIT: &str = "road.superelevations.edit";
pub const ROAD_SUPERELEVATIONS_EDIT_VERSION: u32 = 1;
pub const ROAD_SUPERELEVATIONS_QUERY: &str = "road.superelevations.query";
pub const ROAD_SUPERELEVATIONS_QUERY_VERSION: u32 = 1;

/// Input of `road.superelevations.edit` v1: the schedule by its id or name (absent: a new one). A new one needs `name`,
/// `alignment`, `normalLeft`, `normalRight` and `method`; it holds its alignment's present revision. Slopes are grades
/// (rise over run, −0.025 is −2.5 %; docs/adr/0977), outward from the axis, falling outward negative.
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub normal_left: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub normal_right: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<SuperelevationMethod>,
    /// The axis of rotation (docs/adr/0931 §2); `centerline` takes an edge's off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pivot: Option<SuperelevationPivot>,
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

/// A superelevation schedule as it is now: each side's break points (station, grade) and widening (station,
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub normal_left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub normal_right: f64,
    /// (station, grade) pairs.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "station-grade")))]
    pub left: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "station-grade")))]
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

/// The cross slopes (grades) and widening (metres) of both sides at a station.
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
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

pub const ROAD_SUPERELEVATIONS_CHECK: &str = "road.superelevations.check";
pub const ROAD_SUPERELEVATIONS_CHECK_VERSION: u32 = 1;

/// Input of `road.superelevations.check` v1 (docs/adr/0931 §2): a schedule (its id or name) on a template's traveled
/// way (its lanes' widths; the template by id or name), and the limits, all the design standard's and none given by
/// KentOS: the design speed (km/h) with the most side friction (per one), and the most relative gradient (per one,
/// 0.005 is 1:200). A schedule without a limit is still read: its relative gradients and, with a speed, each curve's
/// friction.
///
/// Refusals: `not_a_road_project`, `unknown_superelevation`, `unknown_road_template`, `invalid_superelevation_check`
/// (a speed, a friction or a gradient not above 0, the friction without a speed, a step not in (0, 100] m, a template
/// without a lane on a side).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsCheck {
    pub superelevation: String,
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub most_friction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub most_relative: Option<f64>,
    /// How often a clothoid's friction is read (m); absent: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: Option<f64>,
}

/// A stretch's relative gradients (per one): each side's traveled way edge over the axis of rotation, the largest on it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RelativeGradient {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub right: f64,
}

/// A curve's side friction at the speed: where it asks for the most, the radius and the superelevation toward the
/// centre there (per one).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CurveFrictionRow {
    pub pi: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub friction: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub at: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub e: f64,
}

/// A limit broken: `relative` (a side's edge) or `friction` (a curve), where, the value, the limit and the words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationFinding {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    /// The relative gradient or the friction, per one.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub value: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub limit: f64,
    pub message: String,
}

/// Output of `road.superelevations.check` v1: the schedule, its axis of rotation, the relative gradients by stretch,
/// each curve's friction (with a speed), the limits broken in station order and the clothoid reading's step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsChecked {
    pub superelevation: String,
    pub pivot: SuperelevationPivot,
    pub gradients: Vec<RelativeGradient>,
    pub curves: Vec<CurveFrictionRow>,
    pub findings: Vec<SuperelevationFinding>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
}

pub const ROAD_SUPERELEVATIONS_WIDEN: &str = "road.superelevations.widen";
pub const ROAD_SUPERELEVATIONS_WIDEN_VERSION: u32 = 1;
pub const ROAD_SUPERELEVATIONS_TRANSITIONS: &str = "road.superelevations.transitions";
pub const ROAD_SUPERELEVATIONS_TRANSITIONS_VERSION: u32 = 1;

/// AASHTO's two forms of the width on a curve (docs/adr/0932 §1): `standard` `N(U + C) + (N − 1)·F_A + Z`, `simple`
/// `N(U + C)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WideningMethod {
    #[default]
    Standard,
    Simple,
}

/// Input of `road.superelevations.widen` v1 (docs/adr/0932 §3): each curve's traveled way widening for a design
/// vehicle, suggested for a schedule (its id or name) on a template's lanes (their number N and width on the tangent
/// W_n; the template by id or name). The vehicle is `road.swept.query`'s (its wheelbase, front overhang, width and
/// trailer); `track` is its track width out to out of the tires (absent: its width). `clearance` (C, per lane), the
/// design `speed` (km/h; absent: no extra width Z), the rounding `step` and the `least` widening worth building are the
/// standard's: KentOS gives none. `drive`: also drive the vehicle along each curve (the tractrix of `road.swept.query`,
/// its front axle on the alignment) and say the largest off-tracking it reaches, which a curve too short for the steady
/// state keeps under the formula's.
///
/// Refusals: `not_a_road_project`, `unknown_superelevation`, `unknown_road_template`, `invalid_widening` (a vehicle
/// size, the track, clearance, speed, step or least not right, a template without a lane).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsWiden {
    pub superelevation: String,
    pub template: String,
    pub vehicle: crate::SweptVehicleDef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub track: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub clearance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<WideningMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub drive: bool,
}

/// A curve's widening: its PI, TS and ST (m), radius, the vehicle's steady-state off-tracking, the track U, the front
/// overhang's width F_A, the extra width Z, the width on the curve W_c, the widening W_c − W_n (negative: the tangent's
/// width is enough), the suggestion (rounded; 0 below `least`), the schedule's widening on the curve now and, with
/// `drive`, the largest off-tracking of the drive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WideningRow {
    pub pi: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offtracking: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub track: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub overhang: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub allowance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub width: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub widening: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub suggested: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub current: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub driven: Option<f64>,
}

/// Output of `road.superelevations.widen` v1: the schedule, the template's lanes and width on the tangent, the method,
/// the curves' widenings in station order and the PIs whose radius the vehicle cannot turn steadily on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsWidened {
    pub superelevation: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lanes: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tangent: f64,
    pub method: WideningMethod,
    pub curves: Vec<WideningRow>,
    pub unturnable: Vec<String>,
}

/// Input of `road.superelevations.transitions` v1 (plan/road.md Y34, docs/adr/0940): each curve's transition proposed
/// for a schedule made from its curves, on a template's traveled way (its lanes' widths). The limits are the design
/// standard's and none is given by KentOS: the most relative gradient (per one, 0.005 is 1:200) gives the shortest
/// runoff; the design `speed` (km/h) with the `comfort` rate (m/s³) the clothoid by comfort (Shortt); the least and the
/// most `shift` (m) the clothoid by its shift; `onTangent` the runoff's share before a curve without clothoids; `step`
/// rounds the runoff up (m).
///
/// Refusals: `not_a_road_project`, `unknown_superelevation`, `unknown_road_template`,
/// `invalid_superelevation_transition` (a limit not above 0 or not under its bound, the comfort without a speed, a share
/// not in [0, 1], a schedule written by station, a template without a lane on a side).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsTransitions {
    pub superelevation: String,
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub most_relative: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/s3")))]
    pub comfort: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub shift: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub most_shift: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub on_tangent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: Option<f64>,
}

/// A curve's transition: its PI, TS and ST, radius, rate (a grade) and clothoids now; its runoff, share on the tangent
/// and largest relative gradient now (none where the curve alone does not build); and what is proposed: the runoff by
/// the relative gradient, the clothoids by comfort, by the least and the most shift and the shortest of them, the
/// runoff and share (on a clothoid the runoff is at least the clothoid and lies on it), its runout and the length held
/// at full superelevation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperTransitionRow {
    pub pi: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub rate: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_in: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_out: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub runoff_now: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub on_tangent_now: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub gradient_now: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub runoff_gradient: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_comfort: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_shift: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_most: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub runoff: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub on_tangent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub runout: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub full: Option<f64>,
}

/// What the proposal finds (docs/adr/0940): a clothoid shorter than comfort and its shift ask (`spiralShort`) or longer
/// than the most shift allows (`spiralLong`), a curve too short to reach full superelevation with the runoff proposed
/// (`full`), two curves whose transitions do not fit the tangent between them (`tangent`, its PI the pair); the value
/// and the limit (m) and in words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperTransitionFinding {
    pub kind: String,
    pub pi: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub value: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub limit: f64,
    pub message: String,
}

/// Output of `road.superelevations.transitions` v1: each curve's transition in station order and what is found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SuperelevationsTransitioned {
    pub superelevation: String,
    pub rows: Vec<SuperTransitionRow>,
    pub findings: Vec<SuperTransitionFinding>,
}
