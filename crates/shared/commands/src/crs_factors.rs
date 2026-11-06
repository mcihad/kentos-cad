//! `cad.crs.factors` v1 (plan/map-cad.md MAP-02, docs/adr/1231): the scales a measured length passes between the
//! ground, the ellipsoid and the project's map at given points, read only: the projection's point scale, the height's
//! factor R/(R + h) along the east and their product (the combined factor), with the project's system and the height
//! used. The desktop's Ölçeklerin özeti shows the same at the screen's middle. The handler is
//! `crates/native/application/src/crs_factors.rs`; the shared cases are `fixtures/commands/v1/cad.crs.factors.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_CRS_FACTORS: &str = "cad.crs.factors";
pub const CAD_CRS_FACTORS_VERSION: u32 = 1;

/// The most points one call takes.
pub const MOST_FACTOR_POINTS: usize = 10_000;

/// Input of `cad.crs.factors` v1: points in the project's coordinates (east, north; metres) and, when given, the mean
/// ellipsoidal height to use instead of the project's (Proje ayarları › Ölçme).
///
/// Refusals, in this order: `invalid_input` (no point, more than 10 000, a coordinate or the height not finite, a height
/// outside −500…9000 m), `no_system` (a project on its own coordinates), `no_point_scale` (a geographic
/// system, the Pseudo-Mercator or an affine local plane: no one scale at a point).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsFactors {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub points: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: Option<f64>,
}

/// One point's factors: none where the system does not reach the point (warning `point_beyond_system`); the height's
/// and the combined only with a height.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointFactors {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub height_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub combined: Option<f64>,
}

/// Output of `cad.crs.factors` v1: the project's system as a sentence names it, the height used (none without one),
/// whether the survey windows take measured lengths to the grid, each point's factors in the input's order, and the
/// drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrsFactorsRead {
    pub system: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: Option<f64>,
    pub reduces_to_grid: bool,
    pub points: Vec<PointFactors>,
    pub revision: String,
}
