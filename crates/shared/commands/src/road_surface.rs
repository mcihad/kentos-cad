//! A terrain surface's analysis (plan/road.md Y18, docs/adr/0916; ROAD-TIN-16): `road.terrains.analysis` reads a
//! surface's elevation, slope and aspect ranges (Civil 3D's Elevations, Slopes and Directions; their plan and 3D
//! areas), its roughness and how a raster sampled from it would put its slope. The geometry is
//! `kentos_road::surface`'s; nothing is kept. The desktop's handler is
//! `crates/native/application/src/road_terrains_analysis.rs`; it and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.terrains.analysis.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const ROAD_TERRAINS_ANALYSIS: &str = "road.terrains.analysis";
pub const ROAD_TERRAINS_ANALYSIS_VERSION: u32 = 1;

/// Input of `road.terrains.analysis` v1: the surface (its id or name) and what is asked: the breaks of the elevation
/// (m), slope (percent) and aspect (degrees from north, clockwise) ranges, increasing, at least two each; `roughness`;
/// `grid` (m: a raster's cell to compare the slope by); with `pieces` each range's parts as plan polygons too. Nothing
/// is filled in: the breaks and the cell are the user's.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_analysis` (nothing asked, breaks fewer than two, not finite
/// or not increasing, a cell not positive), `unknown_terrain`, `terrain_unbuildable`, `invalid_analysis` (more grid
/// cells than four million).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsAnalysis {
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevations: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub slopes: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub aspects: Option<Vec<f64>>,
    #[serde(default)]
    pub roughness: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub grid: Option<f64>,
    #[serde(default)]
    pub pieces: bool,
}

/// A range: its bounds (in the measure's unit), its plan and 3D area, its triangles (slope and aspect; 0 for
/// elevation, whose triangles are cut) and, asked, its parts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainBand {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub surface_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Vec<[f64; 2]>>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub parts: Vec<Vec<[f64; 2]>>,
}

/// A measure's ranges, the area outside them and (aspect) the flat area.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainBands {
    pub bands: Vec<TerrainBand>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub outside_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside_triangles: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub flat_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub flat_triangles: u32,
}

/// The roughness (vector ruggedness measure, 0–1): its mean over the corners, its largest and where.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainRoughness {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub mean: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub max: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
}

/// A raster's slope against the triangles': the cell, the cells compared, the mean (raster − triangle) and mean
/// absolute difference, the largest absolute one and where; percent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainGridSlope {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub cells: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub mean: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub mean_absolute: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub max_absolute: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<[f64; 2]>,
}

/// Output of `road.terrains.analysis` v1: the surface's plan and 3D area and what was asked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsAnalysed {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub surface_area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub elevations: Option<TerrainBands>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slopes: Option<TerrainBands>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub aspects: Option<TerrainBands>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub roughness: Option<TerrainRoughness>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grid: Option<TerrainGridSlope>,
}

pub const ROAD_TERRAINS_COMPARE: &str = "road.terrains.compare";
pub const ROAD_TERRAINS_COMPARE_VERSION: u32 = 1;

/// Input of `road.terrains.compare` v1: the difference of `comparison` over `base` (each a surface's id or name;
/// Civil 3D's TIN volume surface), over the comparison's plan; with `ranges` (m, increasing, at least two) the plan
/// area of the difference between each pair.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_analysis` (ranges fewer than two, not finite or not
/// increasing), `unknown_terrain`, `invalid_analysis` (the same surface twice), `terrain_unbuildable`. Warnings:
/// `compare_uncovered` (part of the comparison is off the base and left out).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsCompare {
    pub base: String,
    pub comparison: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub ranges: Option<Vec<f64>>,
}

/// A difference's extreme: its value (comparison − base) and where.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainDifferenceAt {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub difference: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
}

/// A range of the difference and its plan area.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainDifferenceBand {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
}

/// Output of `road.terrains.compare` v1: the surfaces' ids; the cut (comparison below base) and fill volumes, the net
/// (fill − cut), their plan areas, the area both cover and the comparison's area off the base; the least and the
/// largest difference; with ranges, their areas and the area outside them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsCompared {
    pub base: String,
    pub comparison: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub net: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub cut_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub fill_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub uncovered: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub least: Option<TerrainDifferenceAt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub largest: Option<TerrainDifferenceAt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerrainDifferenceBand>>", optional))]
    pub bands: Vec<TerrainDifferenceBand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub outside_area: Option<f64>,
}
