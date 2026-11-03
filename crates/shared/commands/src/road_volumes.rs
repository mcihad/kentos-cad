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
use crate::{RoadEditOperation, VolumeBenchDef, VolumeFactorDef, VolumeMethod, VolumeStripDef};

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
/// project), `volume_approved` (change or remove an approved one), `unknown_corridor`, `invalid_volume` (another corridor, a new one without its fields, set with nothing to
/// set, the definition by `road_problem`: factors not positive, over 10 or over overlapping stretches; strip rows
/// overlapping or a depth not in (0, 5] m; a bench's slope not in (0, 1000] % or its height not in (0, 10] m; stripping or
/// benching between surfaces; a corridor that does not build), `invalid_revision`, `revision_conflict`.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A row of an earthwork volume between two consecutive sections: their stations and km, the cut and fill (m³; none
/// where a section is not solved), the factors over it (none: not converted), the factored cut and fill, and the
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
    /// The approval against today's figures, where approved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approval: Option<VolumeApprovalReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
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
