//! KentOS Yol's earthwork volume commands (plan/road.md Y8, docs/adr/0906): `road.volumes.edit` writes or removes an
//! earthwork volume (a corridor's, by the average end area, the prismoidal formula or between the corridor's top
//! surface and the terrain, with cut and fill factors by stations); `road.volumes.query` computes every one, or one,
//! now: its rows between consecutive sections, the cut and fill, the factored volumes and the running totals. The
//! geometry is `kentos-road`'s; a volume is a project setting kept in the undo history, its result is computed on every
//! read. The desktop's handlers are `crates/native/application/src/road_volumes*.rs`; they and the Python SDK pass the
//! shared cases in `fixtures/commands/v1/road.volumes.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{RoadEditOperation, VolumeBenchDef, VolumeFactorDef, VolumeMethod, VolumeRule, VolumeStripDef};

pub const ROAD_VOLUMES_EDIT: &str = "road.volumes.edit";
pub const ROAD_VOLUMES_EDIT_VERSION: u32 = 1;
pub const ROAD_VOLUMES_QUERY: &str = "road.volumes.query";
pub const ROAD_VOLUMES_QUERY_VERSION: u32 = 1;
pub const ROAD_VOLUMES_APPROVE: &str = "road.volumes.approve";
pub const ROAD_VOLUMES_APPROVE_VERSION: u32 = 1;

/// Input of `road.volumes.edit` v1: the earthwork volume by its id or name (absent: a new one). A new one needs `name`,
/// `corridor` and `method`. A volume is not rebound: it follows its corridor.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_volume` (rebind), `unknown_volume` (remove one not in the
/// project), `volume_approved` (change or remove an approved one), `volume_in_use` (remove one a mass haul diagram
/// uses), `unknown_corridor`, `invalid_volume` (another corridor, a new one without its fields, set with nothing to
/// set, the definition by `road_problem`: factors not positive, over 10 or over overlapping stretches; strip rows
/// overlapping or a depth not in (0, 5] m; a bench's slope not in (0, 1000] % or its height not in (0, 10] m; stripping or
/// benching between surfaces; a corridor that does not build; a Bitkisel Toprak rule without stripping, a Formasyon rule
/// between surfaces or on a template without a pavement), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corridor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<VolumeMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub factors: Option<Vec<VolumeFactorDef>>,
    /// The topsoil stripped by stations; an empty list takes it off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub strip: Option<Vec<VolumeStripDef>>,
    /// The fill's foundation benched; null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<VolumeBenchDef>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<VolumeBenchDef>"))]
    pub bench: Option<Option<VolumeBenchDef>>,
    /// Which two lines it measures between (docs/adr/0930 §2): Yol or Formasyon against Zemin or Bitkisel Toprak.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rule: Option<VolumeRule>,
    /// The volume on a curve by the sections' moments (docs/adr/0934), the average end area method only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub curve: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A row of an earthwork volume between two consecutive sections: their stations and km, the cut and fill (m³; none
/// where a section is not solved), the factors over it (none: not converted), the factored cut and fill, and the
/// The curve correction's effect on a volume (docs/adr/0934): the corrected less the plain cut and fill (m³), and the
/// stretches left plain because a section reaches past its curve's centre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeCurve {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub plain: u32,
}

/// running totals of the factored cut, the factored fill and their difference (the mass ordinate) to its end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill_factor: Option<f64>,
    /// The topsoil stripped over it (m³), where the volume strips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub strip: Option<f64>,
    /// The benches cut over it (m³), where the volume benches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub bench: Option<f64>,
    /// Each pavement layer's volume over it (m³), in the report's `layers` order; none where a section is not solved or
    /// its template has no such layer. Empty without a pavement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Option<f64>>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub layers: Vec<Option<f64>>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub total_cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub total_fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub mass: f64,
}

/// An earthwork volume as it is now: its sources, whether its corridor's alignment changed since, its rows (by
/// sections), the totals of the cut and fill and their factored volumes, the surfaces' plan areas (between surfaces),
/// the stretches without volume; why it does not compute when it does not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeReport {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub corridor: String,
    pub method: VolumeMethod,
    pub stale: bool,
    pub rows: Vec<VolumeRow>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_fill: f64,
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub uncovered: Option<f64>,
    /// The topsoil stripped in all (m³), where the volume strips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub strip: Option<f64>,
    /// The benches cut in all (m³), where the volume benches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub bench: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub gaps: Vec<[f64; 2]>,
    /// Its rule, where it has one (docs/adr/0930 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rule: Option<VolumeRule>,
    /// The pavement layers its templates have, by name in the order met, with their volumes in all (m³). Empty without a
    /// pavement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<VolumeLayer>>", optional))]
    pub layers: Vec<VolumeLayer>,
    /// The approval against today's figures, where approved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approval: Option<VolumeApprovalReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
    /// With the curve correction (docs/adr/0934): how much it changed the cut and the fill (m³; the cut and fill above
    /// are the corrected ones).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub curve: Option<VolumeCurve>,
}

/// A pavement layer's volume in all: its name, its material code and its volume (m³; docs/adr/0930 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeLayer {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: f64,
}

/// A stretch whose figures differ from the approved ones (beyond 1 cm³, 1e-6 m³), or that is new or gone: the approved cut and
/// fill and today's (none where it has none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeRowChange {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub approved_cut: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub approved_fill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: Option<f64>,
}

/// An approved volume against today's: the approval as written, whether the figures changed, today's totals less the
/// approved ones and the stretches that changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeApprovalReport {
    pub approved: crate::VolumeApprovalDef,
    pub changed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut_change: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill_change: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_cut_change: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_fill_change: f64,
    pub rows: Vec<VolumeRowChange>,
}

/// An earthwork volume in a list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeListed {
    pub id: String,
    pub name: String,
    pub corridor: String,
    pub method: VolumeMethod,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub approved: bool,
}

/// Output of `road.volumes.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesWritten {
    /// The volume written, computed; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub volume: Option<VolumeReport>,
    pub volumes: Vec<VolumeListed>,
    pub revision: String,
}

/// Input of `road.volumes.approve` v1 (docs/adr/0906 §5): the volume (id or name) approved as it computes now, on the
/// day `at` (YYYY-MM-DD) with an optional note; or, with `revoke`, its approval taken off. One undo step “Kübaj onayı”.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_volume`, `volume_not_approved` (revoke one not approved),
/// `invalid_volume` (approve without `at`, a day or note the rule refuses, a volume that does not compute),
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesApprove {
    pub volume: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub revoke: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Input of `road.volumes.query` v1: every earthwork volume computed, or one (`volume`: its id or name).
///
/// Refusals: `not_a_road_project`, `unknown_volume`, `invalid_volume` (the one asked does not compute). Warnings:
/// `corridor_stale`, `volume_gap`, `volume_uncovered`, `volume_unfactored`, `volume_changed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub volume: Option<String>,
}

/// Output of `road.volumes.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesQueried {
    pub volumes: Vec<VolumeReport>,
}

pub const ROAD_VOLUMES_UNCERTAINTY: &str = "road.volumes.uncertainty";
pub const ROAD_VOLUMES_UNCERTAINTY_VERSION: u32 = 1;

/// Input of `road.volumes.uncertainty` v1 (plan/road.md Y35, docs/adr/0941): an earthwork volume (its id or name), the
/// standard deviation of the ground's heights (`sigma`, m; the survey's: a checkpoint RMSE, a GNSS or total station
/// accuracy), an optional shift of all of them (`systematic`, m, signed), and the share of each volume its 95 % bound
/// may reach to be accepted (`accept`, percent). KentOS gives none of them.
///
/// Refusals: `not_a_road_project`, `unknown_volume`, `invalid_volume_uncertainty` (σ not above 0 or over 10 m, a shift
/// over 10 m, a share not in (0, 100]), and the volume's own (`invalid_volume`: its corridor or surface does not build).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesUncertainty {
    pub volume: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub sigma: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub systematic: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub accept: Option<f64>,
}

/// A volume's spread and bound: its value (bank, m³), its standard deviation from the heights' random error, the shift's
/// effect, and the 95 % bound `|shift| + 1.96·σ`; whether the bound is within the share accepted (none: not asked).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeSpread {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub value: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub sigma: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub bias: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub bound: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub accepted: Option<bool>,
}

/// A stretch's own spread (its heights' errors alone; stretches share ground points, so they do not add up).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StretchSpread {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut_sigma: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill_sigma: f64,
}

/// Output of `road.volumes.uncertainty` v1 (docs/adr/0941): the volume, its method, σ and the shift asked; the cut, the
/// fill and their net (cut − fill, linear in the heights: its σ exact) with their spreads; the plan areas of cut and
/// fill and the share of the area within 2σ of the zero line (over half: the cut's and fill's σ only approximate);
/// the ground points that weigh in; the other method's cut and fill (end area for the prismoidal and the surfaces,
/// prismoidal for the end area; none where it does not compute) as the method's share; the binary64 summation bound;
/// and each stretch's own spread.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeUncertaintyReport {
    pub volume: String,
    pub method: VolumeMethod,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub sigma: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub systematic: Option<f64>,
    pub cut: VolumeSpread,
    pub fill: VolumeSpread,
    pub net: VolumeSpread,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub cut_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub fill_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub near_share: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub other_method: Option<VolumeMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub other_cut: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub other_fill: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub numeric: f64,
    pub stretches: Vec<StretchSpread>,
}
