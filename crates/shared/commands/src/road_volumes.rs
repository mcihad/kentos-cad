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
/// overlapping or a depth not in (0, 5] m; a bench's slope not in (0, 10] (a grade) or its height not in (0, 10] m; stripping or
/// benching between surfaces or on the grid; a corridor that does not build; a Bitkisel Toprak rule without stripping, a
/// Formasyon rule between surfaces, on the grid or on a template without a pavement; the grid without a cell of 0.01 to
/// 1000 m, a cell on another method), `invalid_revision`, `revision_conflict`.
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
    /// Which two lines it measures between (docs/adr/0975 §2): Yol or Formasyon against Zemin or Bitkisel Toprak.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rule: Option<VolumeRule>,
    /// The volume on a curve by the sections' moments (docs/adr/0934), the average end area method only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub curve: Option<bool>,
    /// The grid method's cell (m, docs/adr/0974), 0.01 to 1000; a method other than the grid takes it off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: Option<f64>,
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
    /// The sampling step it used (docs/adr/0974): the grid's cell, else the largest interval between two consecutive
    /// sections (m); absent where there is no interval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: Option<f64>,
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
    /// Its rule, where it has one (docs/adr/0975 §2).
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
    /// What it measures and how (docs/adr/1037; Netpro's rules, ROAD-VOL-19): only in `road.volumes.query`'s answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub basis: Option<VolumeBasis>,
    /// What it is computed from, each as it is now (docs/adr/1040; ROAD-VOL-01): only in `road.volumes.query`'s
    /// answer, where it computes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sources: Option<crate::VolumeSources>,
}

/// A pavement layer's volume in all: its name, its material code and its volume (m³; docs/adr/0975 §2).
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
/// day `at` (YYYY-MM-DD) with an optional note; or, with `revoke`, its approval taken off (`at` and `note` then the day
/// and why, both optional). One undo step “Kübaj onayı”. An approval that ends — revoked, or replaced by a new one on
/// its day — goes to the volume's `history` as it was; each approval is numbered from 1 (docs/adr/0950).
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

/// The version of the rules' meaning (docs/adr/1037): a rule's lines, sides and states as `VolumeBasis` says them.
pub const VOLUME_RULE_VERSION: &str = "kentos.volume-rule/1";

/// A line a volume is measured between (docs/adr/1037): the road's top (finished grade), the formation under its
/// pavement, the ground, or the ground under its stripped topsoil.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum VolumeLine {
    Road,
    Formation,
    Ground,
    Topsoil,
}

/// Where a volume's sides are (docs/adr/1037): each section's daylight points (the end area and prismoidal methods),
/// or the corridor's top surface's edge (the surfaces and grid methods).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum VolumeSides {
    Daylights,
    Corridor,
}

/// A material's state as a volume measures it (docs/adr/1037).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MaterialState {
    /// In place (yerinde): the cut before it is dug.
    InPlace,
    /// Compacted (sıkışmış): the fill's body.
    Compacted,
}

/// What a volume measures (docs/adr/1037): its rule by Netpro's name, whether the volume names it (`given`) or it is
/// the default (the road's top against the ground, against the stripped ground where it strips), the rule's meaning's
/// version, the two lines, the sides, the stations it covers and the states its cut and fill are in; and how it is
/// computed (docs/adr/1040): its method's algorithm by name and version (`kentos.end-area/1`,
/// `kentos.end-area-curved/1`, `kentos.prismoidal/1`, `kentos.tin-overlay/1`, `kentos.grid-midpoint/1`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeBasis {
    pub name: String,
    pub given: bool,
    pub version: String,
    pub top: VolumeLine,
    pub bottom: VolumeLine,
    pub sides: VolumeSides,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub cut: MaterialState,
    pub fill: MaterialState,
    pub algorithm: String,
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

pub const ROAD_VOLUMES_PROGRESS: &str = "road.volumes.progress";
pub const ROAD_VOLUMES_PROGRESS_VERSION: u32 = 1;

/// Input of `road.volumes.progress` v1 (plan/road.md Y36, docs/adr/0942): an earthwork volume (its id or name), a
/// survey of the site now (`survey`: a terrain surface's id or name) and, for a period's work, the survey before it
/// (`previous`). The original ground is the volume's corridor's surface; the design its top surface.
///
/// Refusals: `not_a_road_project`, `unknown_volume`, `unknown_terrain` (a survey not in the project),
/// `invalid_volume_progress` (a survey that is the original ground, the previous survey the same as the survey, a
/// survey that does not build), and the volume's own (`invalid_volume`: its corridor or surface does not build),
/// `unproven_heights` (docs/adr/1029: the surfaces' heights are not shown to rest on one vertical system).
/// `assumeHeights` takes them as one: the report is then a draft (`heights_assumed`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesProgress {
    pub volume: String,
    pub survey: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub previous: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub assume_heights: bool,
}

/// The cut's and the fill's work against the design (bank, m³): done, left, and over (dug below the design in cut,
/// placed above it in fill). In a period, the later survey's less the earlier's: negative where the work went back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgressAmounts {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut_done: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut_left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut_over: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill_done: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill_left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill_over: f64,
}

/// A stretch's own progress by the survey now (stretches between consecutive sections of the corridor), and its plan
/// area the original ground or the survey does not cover (m²; its work there is not counted).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StretchProgress {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    pub amounts: ProgressAmounts,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub uncovered: f64,
}

/// Output of `road.volumes.progress` v1 (docs/adr/0942): the volume; the original ground (the corridor's surface), the
/// survey and the previous survey with their sources' digests (what each was made of when read); the work by the survey
/// now and by the previous one, and the period's (now less before); the share of the design's cut and fill done
/// (percent; none where the design has none); the plan area all three cover and the design's area not covered by the
/// original or the survey; each stretch's own work by the survey now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeProgressReport {
    pub volume: String,
    pub original: String,
    pub original_digest: String,
    pub survey: String,
    pub survey_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub previous: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub previous_digest: Option<String>,
    pub now: ProgressAmounts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub before: Option<ProgressAmounts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub period: Option<ProgressAmounts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub cut_share: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub fill_share: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub uncovered: f64,
    pub stretches: Vec<StretchProgress>,
}

pub const ROAD_VOLUMES_COMPARE: &str = "road.volumes.compare";
pub const ROAD_VOLUMES_COMPARE_VERSION: u32 = 1;

/// Input of `road.volumes.compare` v1 (plan/road.md Y37, docs/adr/0943): two to six earthwork volumes (ids or names),
/// the first the base; the plan tolerance (m, default 0.01) within which two alignments are the same line, and the
/// height tolerance (m, default 0.01) within which two profiles are the same.
///
/// Refusals: `not_a_road_project`, `unknown_volume`, `invalid_volume_compare` (fewer than two or more than six volumes,
/// one named twice, a tolerance not above 0 or over 10 m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesCompare {
    pub volumes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<f64>,
}

/// A stretch where an alternative departs from the base: its stations and km on the alternative, the base's nearest
/// stations and km at its ends (none beyond the base's ends), and its largest departure (m; not where: on a stretch
/// where two profiles run parallel every station is the largest, so the place is not one).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DepartureRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base_from_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base_to_text: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub largest: f64,
}

/// One alternative: its volume, corridor, alignment, profile and surface with their revisions and the surface's
/// sources' digest; its alignment's length; its volumes (bank and factored); its approval's changes (none when not
/// approved); against the base (none for the base itself): the volumes' changes and the stretches its alignment and
/// profile depart. A volume that does not compute says why and has no figures.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlternativeReport {
    pub volume: String,
    pub name: String,
    pub corridor: String,
    pub alignment: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    pub profile: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub profile_revision: u32,
    pub surface: String,
    pub surface_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub figures: Option<AlternativeFigures>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approved: Option<AlternativeApproval>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub against: Option<AgainstBase>,
}

/// An alternative's volumes (m³): bank cut and fill, factored cut and fill.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlternativeFigures {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_fill: f64,
}

/// An alternative's approval: the day, and its bank cut's and fill's change since (m³), and whether anything changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlternativeApproval {
    pub at: String,
    pub changed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut_change: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill_change: f64,
}

/// An alternative against the base: its length's and volumes' changes (none where either does not compute), and the
/// stretches its alignment departs in plan and its profile in height.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AgainstBase {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length_change: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub figures_change: Option<AlternativeFigures>,
    pub plan: Vec<DepartureRow>,
    pub height: Vec<DepartureRow>,
}

/// Output of `road.volumes.compare` v1 (docs/adr/0943): the tolerances used and the alternatives, the base first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesCompared {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: f64,
    pub alternatives: Vec<AlternativeReport>,
}

pub const ROAD_VOLUMES_LEDGER: &str = "road.volumes.ledger";
pub const ROAD_VOLUMES_LEDGER_VERSION: u32 = 1;

/// How a ledger groups a volume's stretches (docs/adr/0949): by km, `step` metres on its multiples; by the corridor's
/// regions; or by the volume's factor rows (ROAD-VOL-21: which swell and shrink factor applied to which stretches,
/// those under none apart).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LedgerBy {
    Stations {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        step: f64,
    },
    Regions,
    Factors,
}

/// Input of `road.volumes.ledger` v1 (plan/road.md Y43, docs/adr/0949; ROAD-VOL-16, 18): an earthwork volume's total
/// taken apart — its stretches grouped by km or by region, each material's column; the `top` largest stretches of the
/// cut and of the fill (1 to 100; absent: 10) with why they are large; how the cut and the fill crowd along the road;
/// and the figures rounded to `decimals` (0 to 3; absent: 2), the rounded stretches' sum against the rounded total.
///
/// Refusals: `not_a_road_project`, `unknown_volume`, `invalid_volume_ledger` (a step out of 1–100000 m, a top out of
/// 1–100, decimals over 3), and the volume's own (`invalid_volume`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesLedger {
    pub volume: String,
    pub by: LedgerBy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub top: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: Option<u32>,
}

/// A ledger's column (a material): `cut`, `fill`, `factoredCut`, `factoredFill`, `strip`, `bench` or `layer:<name>`; its
/// words; its compensated total; its stretches rounded and summed, its total rounded, and their difference.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LedgerColumn {
    pub key: String,
    pub words: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub total: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub rows_rounded: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub total_rounded: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub residue: f64,
}

/// A group of stretches: its words, its range between sections, how many stretches, each column's sum and share of the
/// column's total (none where the total is 0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LedgerGroup {
    pub label: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stretches: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub values: Vec<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub shares: Vec<Option<f64>>,
}

/// One of the largest stretches: its range, volume, share and running share of the column's total, its length and
/// mean area, each over the stretches' median, and which is larger: `section` or `length`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LedgerStretch {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub share: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cumulative: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub mean_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub length_factor: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub area_factor: f64,
    pub reason: String,
}

/// A column's largest stretches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LedgerTop {
    pub column: String,
    pub stretches: Vec<LedgerStretch>,
}

/// How a column crowds along the road: its Gini index, the shares of the length holding half and four fifths of it,
/// and its densest-first curve (length share, volume share).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LedgerConcentration {
    pub column: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub gini: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub half: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub most: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub curve: Vec<[f64; 2]>,
}

/// Output of `road.volumes.ledger` v1 (docs/adr/0949): the volume, the rounding's decimals, the columns, the groups,
/// each column's groups' sum less its total (0 but for the last bits), the largest stretches and the concentration of
/// the cut and of the fill, and the stretches without volume.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeLedgerRead {
    pub volume: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: u32,
    pub columns: Vec<LedgerColumn>,
    pub groups: Vec<LedgerGroup>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub residual: Vec<f64>,
    pub top: Vec<LedgerTop>,
    pub concentration: Vec<LedgerConcentration>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub gaps: Vec<[f64; 2]>,
}

pub const ROAD_VOLUMES_HISTORY: &str = "road.volumes.history";
pub const ROAD_VOLUMES_HISTORY_VERSION: u32 = 1;

/// Input of `road.volumes.history` v1 (plan/road.md Y44, docs/adr/0950; ROAD-VOL-17): an earthwork volume's
/// approvals, oldest first, each against the one before it, and today's figures against the last.
///
/// Refusals: `not_a_road_project`, `unknown_volume`. A volume that does not compute today still has its history; the
/// answer then says why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesHistory {
    pub volume: String,
}

/// One set of a volume's figures against another (docs/adr/0950): whether they differ (beyond 1e-6 m³), the totals'
/// changes and the stretches that changed, are new or gone (`approved*` the earlier's, the others the later's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeChange {
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

/// An approval in a volume's history: as it was written (with how it ended), whether it stands, and against the
/// approval before it (none for the first).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeHistoryEntry {
    pub approval: crate::VolumeApprovalDef,
    pub standing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub against: Option<VolumeChange>,
}

/// A volume's figures today: its cut, fill and their factored volumes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeToday {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub factored_fill: f64,
}

/// Output of `road.volumes.history` v1 (docs/adr/0950): the volume, its approvals oldest first, today's figures (none
/// where it does not compute, then why) and today against the last approval (none without one).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeHistoryRead {
    pub volume: String,
    pub name: String,
    pub approvals: Vec<VolumeHistoryEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub today: Option<VolumeToday>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub since: Option<VolumeChange>,
}

pub const ROAD_VOLUMES_STATES: &str = "road.volumes.states";
pub const ROAD_VOLUMES_STATES_VERSION: u32 = 1;

/// Input of `road.volumes.states` v1 (plan/road.md Y51, docs/adr/0957; ROAD-VOL-10): a volume's earthwork in its three
/// states, row by row of its factor table: the cut as measured in place (bank, yerinde) and loosened (as a truck
/// carries it), the fill as measured compacted and loosened (what it asks), and where the row gives the soil's
/// densities, the bank the fill asks, the compacted volume the cut makes and both masses (t). What a factor or a
/// density says that the soil cannot do is told. Reads only.
///
/// Refusals: `not_a_road_project`, `unknown_volume`, `invalid_volume` (one that does not compute).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumesStates {
    pub volume: String,
}

/// A factor row's earthwork in its states.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeStateRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub densities: Option<crate::VolumeDensities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill_factor: Option<f64>,
    /// The cut in place and the fill compacted, as measured (m³).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    /// Loosened: the cut times its factor, what the fill asks times its factor (m³; the measured where none).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut_loose: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill_loose: f64,
    /// With densities: the bank the fill asks, the compacted volume the cut makes (m³) and the masses (t).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill_bank: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut_compacted: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "t")))]
    pub cut_mass: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "t")))]
    pub fill_mass: Option<f64>,
    /// What its factors or densities say that the soil cannot do, in words.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub checks: Vec<String>,
}

/// Output of `road.volumes.states` v1: the volume, each factor row in its states, the stretches no row covers (their
/// cut and fill as measured), and the totals: in place and compacted as measured, loosened (the mass ordinate's end is
/// their difference), and where every stretch has densities, the masses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VolumeStatesRead {
    pub volume: String,
    pub rows: Vec<VolumeStateRow>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub unfactored_cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub unfactored_fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut_loose: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill_loose: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub balance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "t")))]
    pub cut_mass: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "t")))]
    pub fill_mass: Option<f64>,
}
