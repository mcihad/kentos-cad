//! `cad.pointclouds.register` v1 (plan/map-cad.md MAP-13, docs/adr/1800): Tarama kaydı denetimi, read only. The
//! second cloud (moving) is fitted to the first (fixed) where they overlap by point-to-plane ICP (Chen & Medioni; Low
//! 2004's linear least squares): both read at full resolution and thinned to one point per voxel (its points' mean),
//! the fixed cloud's normals from its nearest voxel points, each moving point paired with the nearest fixed one within
//! the reach. The answer is the transform's recipe (about a stated centre, and as a 4×4 matrix), the point-to-plane
//! RMS before and after, the share of the moving cloud paired, how firmly the geometry holds the six parameters (a
//! flat ground leaves the horizontal ones loose), the plan cells whose residual holds to the limit (reliable
//! coverage) and the control points' heights before and after. Nothing is written: the cloud is not moved. The
//! shared cases are `fixtures/commands/v1/cad.pointclouds.register.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{CloudRules, OverlapControlStats};

pub const CAD_POINTCLOUDS_REGISTER: &str = "cad.pointclouds.register";
pub const CAD_POINTCLOUDS_REGISTER_VERSION: u32 = 1;

/// The most voxel points a cloud may give a registration.
pub const REGISTER_MOST_POINTS: u32 = 400_000;
/// The most iterations a registration may ask for.
pub const REGISTER_MOST_ITERATIONS: u32 = 200;

/// Input of `cad.pointclouds.register` v1: the two clouds (persistent ids: the fixed first, the moving second); the
/// voxel's side (`spacing`, none: 0.5 m), the farthest pair (`reach`, none: 1 m), the most iterations (none: 30) and
/// the step that ends them (`tolerance`, none: 0.1 mm), the neighbours a normal is fitted to (none: 8), the plan cell
/// of the coverage (none: 2 m) and the pairs it needs (`least`, none: 4), the classes taken (none: all), the control
/// points' layer, the attribute naming them (none: `Ad`) and the reach around one (`radius`, none: the cell), the
/// scale's denominator (none: the project's plot scale), the rule package (none: KentOS's `lidar-uyum`) and the open
/// drawing's file (the host gives it).
///
/// Refusals: `invalid_input` (not two different clouds; a number out of its range; `scale` 0; `rules`),
/// `invalid_uid`, `entity_not_found`, `not_a_pointcloud`, `layer_not_found` (`control`), `too_many_cloud_points` (more
/// voxel points than `REGISTER_MOST_POINTS` in a cloud). Warnings: `cloud_source_unreadable`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsRegister {
    pub uids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub reach: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub iterations: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub neighbours: Option<u32>,
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

/// A coverage cell: its centre, the moving cloud's paired points in it and their RMS residual after the fit (metres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RegisterCell {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub n: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms: f64,
}

/// A control point: its name and height, and the fixed cloud's, the moving cloud's before and after the fit planes
/// minus it there (none where a cloud has too few points around it, or they lie on a line).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RegisterControl {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dz_fixed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dz_before: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dz_after: Option<f64>,
}

/// Output of `cad.pointclouds.register` v1:
/// - the package; the points read and the voxel points used per cloud;
/// - the iterations done and whether the last step was under the tolerance;
/// - the transform from the moving cloud to the fixed one: p ↦ R(p − centre) + centre + translation, R = Rz(κ)·Ry(φ)·
///   Rx(ω) with `rotation` = [ω, φ, κ] (radians), and the same as a row-major 4×4 matrix in the project's coordinates;
/// - the pairs at the end and the share of the moving voxel points they are (`overlap`); the point-to-plane RMS
///   before the first step and after the last, the mean after;
/// - `strength`: √(λmin/λmax) of the six parameters' normal matrix (the angles times the moving points' RMS radius,
///   so all are metres): near 0 a direction the geometry does not fix; `horizontal`: √ of the plan translations'
///   smaller eigenvalue over the height translation's (near 0: a flat ground, the plan shift unknown);
/// - the coverage's cell, the area of the cells with `least` pairs and of those whose RMS holds to `limit` (the
///   package's swath-to-swath value), the worst cells (at most 20, RMS down);
/// - the control points and the statistics of the fixed cloud and of the moving cloud before and after, KOH_Z's limit;
/// - notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsRegistered {
    pub rules: String,
    pub source: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: [u64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub used: [u32; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub iterations: u32,
    pub converged: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub centre: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub translation: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub matrix: [f64; 16],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pairs: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub overlap: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms_before: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms_after: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub mean_after: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub strength: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub horizontal: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub cell: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub reliable: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit: f64,
    pub worst: Vec<RegisterCell>,
    pub control: Vec<RegisterControl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub control_fixed: Option<OverlapControlStats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub control_before: Option<OverlapControlStats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub control_after: Option<OverlapControlStats>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit_z: f64,
    pub notes: Vec<String>,
}
