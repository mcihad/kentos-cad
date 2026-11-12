//! `cad.crs.factors` v1 (plan/map-cad.md MAP-02, docs/adr/1231): the scales a measured length passes between the
//! ground, the ellipsoid and the project's map at given points, read only: the projection's point scale, the height's
//! factor R/(R + h) along the east and their product (the combined factor), with the project's system and the height
//! used. The desktop's Ölçeklerin özeti shows the same at the screen's middle. Each point may have its own height (an
//! ellipsoidal height, or the drawing's height made ellipsoidal by the project's heights), and lines between the points
//! get the factors a length measured along them passes, the survey windows' (docs/adr/2900). The handler is
//! `crates/native/application/src/crs_factors.rs`; the shared cases are `fixtures/commands/v1/cad.crs.factors.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_CRS_FACTORS: &str = "cad.crs.factors";
pub const CAD_CRS_FACTORS_VERSION: u32 = 1;

/// The most points one call takes.
pub const MOST_FACTOR_POINTS: usize = 10_000;

/// The most lines one call takes.
pub const MOST_FACTOR_LINES: usize = 10_000;

/// Input of `cad.crs.factors` v1: points in the project's coordinates (east, north; metres) and, when given, the mean
/// ellipsoidal height to use instead of the project's (Proje ayarları › Ölçme), or each point's own height (docs/adr/2900
/// §1–2): `heights`, ellipsoidal heights, or `elevations`, the drawing's heights made ellipsoidal by what the project
/// says its heights are (as they are when ellipsoidal; plus the project's geoid model's N at the point when
/// orthometric). `lines` asks for the factors of lines between two of the points.
///
/// Refusals, in this order: `invalid_input` (no point, more than 10 000, a coordinate or the height not finite, a height
/// outside −500…9000 m; more than one of `height`, `heights`, `elevations`; not one per point; a line's end not a
/// point, its two ends at one place, more than 10 000 lines; heights not in metres), `no_system` (a project on its own
/// coordinates), `no_point_scale` (a geographic system, the Pseudo-Mercator or an affine local plane: no one scale at a
/// point); with `elevations`: `heights_unknown` (the project does not say what its heights are), `geoid_required`
/// (orthometric without a geoid model), `geoid_missing` (the model's grid is not on this device). A point outside the
/// geoid's grid gets no height (warning `points_outside`).
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
    /// Each point's ellipsoidal height, one per point: a point's height factor at its own height, a line's at its ends'
    /// mean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub heights: Option<Vec<f64>>,
    /// Each point's height as the drawing keeps it (its Z), one per point, made ellipsoidal by the project's heights.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevations: Option<Vec<f64>>,
    /// Lines between two of the points, by their indexes from 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub lines: Option<Vec<[u32; 2]>>,
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
    /// The point's own ellipsoidal height (with `heights` or `elevations`; none outside the geoid's grid).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: Option<f64>,
    /// The geoid's height above the ellipsoid at the point, when its elevation was orthometric.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub undulation: Option<f64>,
}

/// One line's factors (docs/adr/2900 §1): the projection's scale along it (Simpson's rule on its ends' and middle's
/// scales), the height's factor R/(R + h) with Euler's radius at the geodesic's middle in its direction and h its ends'
/// mean height (or the one height), their product, and the height used; none where the system does not reach an end,
/// the height's and the combined only with a height.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineFactors {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: Option<f64>,
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
    /// Each line's factors, in the input's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<LineFactors>>"))]
    pub lines: Vec<LineFactors>,
    /// The geoid model the elevations were made ellipsoidal with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geoid: Option<String>,
    pub revision: String,
}
