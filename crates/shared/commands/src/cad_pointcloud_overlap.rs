//! `cad.pointclouds.overlap` v1 (plan/map-cad.md MAP-13, docs/adr/1500): Tarama uyumu denetimi, read only. Two scans
//! (strips, sessions) of the same place compared where they overlap, at the sources' full resolution: on a grid of
//! cells, each cloud's points in a cell fitted by a plane (so a slope does not read as a difference), the difference
//! of the two planes at the cell's centre; over the cells both fill, the RMSDz, the mean (a systematic shift), the
//! largest and the worst cells. Against control points measured on the ground (a point layer), each cloud's plane
//! around the point gives its height error and KOH_Z. The limits are a rule package kept as data (the interswath
//! RMSDz, BÖHHBÜY Article 91(2)'s KOH_Z), not constants in code. Nothing is written; no cloud is moved. The shared
//! cases are `fixtures/commands/v1/cad.pointclouds.overlap.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_POINTCLOUDS_OVERLAP: &str = "cad.pointclouds.overlap";
pub const CAD_POINTCLOUDS_OVERLAP_VERSION: u32 = 1;

/// The rule package: who issues it, the document, when it holds; the interswath RMSDz limit (metres) and KOH_Z's limit
/// in centimetres as `zFactor × (xy[0]·S² + xy[1]·S + xy[2])`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudRules {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub release: u32,
    pub jurisdiction: String,
    pub source: String,
    pub effective: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub interswath: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub xy: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub z_factor: f64,
}

/// KentOS's example package (`catalog/v1/rules/lidar-uyum.json`).
pub fn cloud_rules_default() -> Option<CloudRules> {
    serde_json::from_str(include_str!("../../../../catalog/v1/rules/lidar-uyum.json")).ok()
}

/// Input of `cad.pointclouds.overlap` v1: the two clouds (`uids`, exactly two), the cell's side (metres; none: 2), the
/// fewest points a cloud must have in a cell (none: 4), the classes taken (none: all), the layer of the control points
/// (none: no control), their name attribute (none: `Ad`) and the reach around one (metres; none: the cell's side), the
/// scale's denominator (none: the project's), the rule package (none: KentOS's example) and the drawing's file (its
/// folder resolves a relative source).
///
/// Refusals: `invalid_uid`, `entity_not_found`, `not_a_pointcloud` (`uids[i]`), `invalid_input` (`uids`: not two
/// different clouds; `cell`, `radius` not finite and over 0; `least` under 3; `scale` 0; `rules`: an empty id or
/// source, a limit not finite), `layer_not_found` (`control`), `too_many_cloud_points`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsOverlap {
    pub uids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub cell: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub least: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub classes: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub control: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<CloudRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A cell both clouds fill: its centre and the second cloud's plane minus the first's there (metres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OverlapCell {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dz: f64,
}

/// A control point: its name, its height, and each cloud's plane minus it there (none where the cloud has too few
/// points around it, or they lie on a line).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OverlapControl {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dz_a: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dz_b: Option<f64>,
}

/// A cloud's control statistics: how many control points it reached, KOH_Z and the mean (metres), and whether KOH_Z
/// holds to the limit (none without one).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OverlapControlStats {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub n: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub koh_z: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub mean: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed: Option<bool>,
}

/// Output of `cad.pointclouds.overlap` v1: the package, the cell, the points read and taken per cloud, the cells each
/// cloud fills, the cells both fill and their area, the cells left out because a plane was not defined, RMSDz, mean,
/// standard deviation and largest |dz| over the overlap and whether RMSDz holds (none without overlap), the worst
/// cells (at most 20, by |dz|), the control points with each cloud's statistics and KOH_Z's limit, and notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsOverlapped {
    pub rules: String,
    pub source: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub cell: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: [u64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub taken: [u64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub filled: [u32; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub cells: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m2")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub degenerate: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rmsd_z: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub mean: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub std: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed: Option<bool>,
    pub worst: Vec<OverlapCell>,
    pub control: Vec<OverlapControl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub control_a: Option<OverlapControlStats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub control_b: Option<OverlapControlStats>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit_z: f64,
    pub notes: Vec<String>,
}
