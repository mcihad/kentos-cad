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
/// area of the difference between each pair. `method` (docs/adr/1600): `tin` (none: the exact TIN–TIN prisms) or
/// `grid` on `cell`-metre cells whose corners are multiples of `cell` (the midpoint rule, docs/adr/0974), its result
/// beside the exact one; `cutFactor` and `fillFactor` (Civil 3D's Volumes Dashboard: the cut's swell, the fill's
/// compaction; none: 1, no adjustment, no soil's value guessed) give the adjusted volumes apart from the geometric.
/// `zones` (docs/adr/3540; Civil 3D's bounded volumes): material zones, each a closed object bounding one material with
/// its own factors (none: the comparison's, else 1); the difference inside each, outside them all, and the adjusted
/// volumes summed by material. Zones do not overlap; they are measured by the exact TIN–TIN alone.
/// `sections` (docs/adr/4340; Civil 3D's sample line volumes): parallel sections `interval` metres apart, across an axis
/// whose direction is `azimuth` (degrees from north, clockwise; none: 0), from the comparison's first place along it to
/// its last; each section's cut and fill areas are the difference's exact integral along it where both surfaces are,
/// the volumes the average end area between consecutive sections or, with `formula: prismoidal`, the prismoidal formula
/// with a middle section; the exact TIN–TIN beside (`exact`), the plan areas the exact TIN's.
/// `axis` (docs/adr/4760; Civil 3D's sample lines, Netcad's Enkesit Al): the sections stand along a drawn axis instead,
/// a line's or a single-part open polyline's persistent id (arcs allowed): from its start every `interval` metres while
/// short of its end, then its end, and with `corners` its inner corners too; each square to the axis (on an arc along
/// its radius, at a corner to the bisector), `left` metres to its left and `right` to its right; the length between two
/// is their stations' difference along the axis.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_analysis` (ranges fewer than two, not finite or not
/// increasing; grid without a cell, a cell not over 0, ranges or zones with grid; sections without an interval, an
/// interval not over 0, an azimuth not in [0, 360), ranges or zones with sections, a formula without sections; an axis
/// without sections or with an azimuth, widths or corners without an axis, a width not a length of 0 or more or both 0;
/// more than 10 000 sections, said once the comparison is made; a factor not over 0 or over 10, the
/// comparison's or a zone's; a zone without a name), `unknown_terrain`, `invalid_analysis` (the same surface twice),
/// `entity_not_found` (the axis), `invalid_analysis` (an axis not a line or a single-part open polyline, or without
/// length), `entity_not_found` (a zone's boundary), `invalid_analysis` (a boundary not a straight-edged
/// single-part closed polyline or area; two zones sharing more than 1 mm²). Then `unproven_heights` (docs/adr/1029: the two surfaces'
/// heights are not shown to rest on one vertical system; `assumeHeights` takes them as one and the difference is a
/// draft, `heights_assumed`), `terrain_unbuildable`, `invalid_analysis` (more than 4 000 000 cells). Warnings:
/// `compare_uncovered` (part of the comparison is off the base and left out; part of a zone off the two surfaces).
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<CompareMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub assume_heights: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CompareZone>>", optional))]
    pub zones: Vec<CompareZone>,
    /// The sections' spacing, m (docs/adr/4340; the section method's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub interval: Option<f64>,
    /// The axis's direction, degrees from north clockwise (a survey azimuth); none: 0, the sections run east–west.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub azimuth: Option<f64>,
    /// How the volume between two sections is reckoned; none: the average end area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub formula: Option<SectionFormula>,
    /// A drawn axis the sections stand along (docs/adr/4760): a line's or an open polyline's persistent id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub axis: Option<String>,
    /// The sections' widths left and right of the axis, m (docs/adr/4760).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right: Option<f64>,
    /// Sections at the axis's inner corners too (docs/adr/4760; Civil 3D's At Horizontal Geometry Points).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corners: Option<bool>,
}

/// The section method's formula (docs/adr/4340; Civil 3D's Average End Area and Prismoidal): `(A₀ + A₁)/2 · L`, or
/// `L/6 · (A₀ + 4·Aₘ + A₁)` with the middle section's area `Aₘ`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SectionFormula {
    #[default]
    EndArea,
    Prismoidal,
}

/// The most sections a section comparison measures.
pub const MOST_COMPARE_SECTIONS: f64 = 10_000.0;

/// A section of the section method (docs/adr/4340): its distance from the first section along the axis and its cut and
/// fill areas (m²); along a drawn axis (docs/adr/4760) the axis's place there and the section's left and right ends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainSection {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub cut_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub fill_area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub ends: Option<[[f64; 2]; 2]>,
}

/// A material zone (docs/adr/3540; Civil 3D's bounded volume): the material's name (the user's; no class or value is
/// guessed), the closed object bounding it (a straight-edged, single-part closed polyline or area, holes allowed) and
/// its cut and fill factors (none: the comparison's, else 1; more than 0, at most 10).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CompareZone {
    pub name: String,
    pub boundary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill_factor: Option<f64>,
}

/// A material zone's part of the difference (docs/adr/3540): its name and boundary, its plan area and the part of it
/// both surfaces cover, the cut, fill and net volumes and their plan areas there (the exact TIN–TIN), and the volumes
/// adjusted by its factors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainZoneVolumes {
    pub name: String,
    pub boundary: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
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
    pub adjusted: TerrainAdjusted,
}

/// The difference outside every material zone (docs/adr/3540): the area both surfaces cover there, the cut, fill and
/// net volumes and their plan areas (the exact TIN–TIN less the zones'), adjusted by the comparison's factors (none: 1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainZoneRest {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
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
    pub adjusted: TerrainAdjusted,
}

/// How two surfaces' volume is measured (docs/adr/1600, 4340): the exact TIN–TIN prisms, a grid's cells, or parallel
/// sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CompareMethod {
    #[default]
    Tin,
    Grid,
    Sections,
}

/// The most cells a grid comparison measures.
pub const MOST_COMPARE_CELLS: f64 = 4_000_000.0;

/// Cut, fill and net (fill − cut) volumes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainVolumes {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub net: f64,
}

/// The volumes adjusted by the factors (docs/adr/1600): the cut times `cutFactor` (its swell), the fill times
/// `fillFactor` (its compaction), net their difference; the geometric volumes stay as they are.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainAdjusted {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut_factor: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill_factor: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub net: f64,
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
/// (fill − cut), their plan areas, the area both cover and the comparison's area off the base, by the method asked;
/// the least and the largest difference (always the exact TIN's); with ranges, their areas and the area outside them.
/// With `sections` (docs/adr/4340): the interval, the azimuth, the formula and each section's areas (`sections`); the
/// volumes are the sections', the plan areas the exact TIN's, the exact TIN–TIN volumes beside (`exact`).
/// With `grid`: the cell and the exact TIN–TIN volumes beside (`exact`; the method's difference is theirs); with a
/// factor: the adjusted volumes. With zones (docs/adr/3540): each zone's part (`zones`), the part outside them all
/// (`rest`) and the adjusted volumes summed by material (`zoned`: the zones' with their factors, the rest's with the
/// comparison's).
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub exact: Option<TerrainVolumes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub adjusted: Option<TerrainAdjusted>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerrainZoneVolumes>>", optional))]
    pub zones: Vec<TerrainZoneVolumes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rest: Option<TerrainZoneRest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zoned: Option<TerrainVolumes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub interval: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub azimuth: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub formula: Option<SectionFormula>,
    /// The drawn axis the sections stood along and its length (docs/adr/4760).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub axis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerrainSection>>", optional))]
    pub sections: Vec<TerrainSection>,
}

pub const ROAD_TERRAINS_ZEROLINE: &str = "road.terrains.zeroline";
pub const ROAD_TERRAINS_ZEROLINE_VERSION: u32 = 1;

/// Input of `road.terrains.zeroline` v1 (docs/adr/1620; plan/map-cad.md MAP-15): the cut and fill boundary of
/// `comparison` over `base` (each a surface's id or name) written as 3D polylines on a layer: where the difference
/// `comparison − base` is zero, over the comparison's plan, each line at the base's height. One undo step “Sıfır
/// hattı”.
///
/// Refusals, in this order: `not_a_road_project`, `layer_not_found`, `not_a_layer`, `layer_locked`,
/// `invalid_revision`, `revision_conflict`, `unknown_terrain`, `invalid_analysis` (the same surface twice),
/// `terrain_unbuildable`, `no_zero_line` (the difference changes sign nowhere).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsZeroLine {
    pub base: String,
    pub comparison: String,
    /// The layer the lines go on.
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A zero line as written: whether closed, its plan length, its places (x, y, the base's height).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainZeroLine {
    pub closed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub points: Vec<[f64; 3]>,
}

/// Output of `road.terrains.zeroline` v1, and its plan: the surfaces' ids, the lines (each starting at its smallest
/// place, x then y, ordered by it), their plan length, the area where the two surfaces coincide (no line is drawn
/// there), the objects written (a plan: none) and the revision after it (a plan: before).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsZeroLined {
    pub base: String,
    pub comparison: String,
    pub lines: Vec<TerrainZeroLine>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub coincident: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::REVISION_TEXT)))]
    pub revision: String,
}

pub const ROAD_TERRAINS_QUALITY: &str = "road.terrains.quality";
pub const ROAD_TERRAINS_QUALITY_VERSION: u32 = 1;

/// The most near pairs and spikes one answer lists (their counts are whole).
pub const MOST_QUALITY_ITEMS: usize = 1000;

/// Spikes asked for: corners more than `limit` (m) off the mean height of the other corners within `radius` (m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainSpikesAsk {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub limit: f64,
}

/// Check points: the points of `layer` and those of `uids`, their class the attribute `classField`'s value (none: one
/// class).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainCheckpointsAsk {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub class_field: Option<String>,
    /// The check points' own RMSE (m; their survey's), combined with each class's (docs/adr/0945).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: Option<f64>,
}

/// Input of `road.terrains.quality` v1: a surface's survey quality (ROAD-SURVEY-04, 05): `near` (m: corners closer in
/// plan, a duplicate shot), `spikes`, `gaps` (m: triangles with a longer edge, too sparse a survey), `checkpoints`
/// (their heights against the surface by class). The limits are the user's; nothing is changed.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_analysis` (nothing asked, a limit not positive, a spike
/// limit negative, check points with neither a layer nor uids), `unknown_terrain`, `terrain_unbuildable`,
/// `layer_not_found`, `entity_not_found`, `invalid_analysis` (a check point's object not a point).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsQuality {
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub near: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub spikes: Option<TerrainSpikesAsk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub gaps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub checkpoints: Option<TerrainCheckpointsAsk>,
}

/// Two corners closer than the limit: their places and heights, the plan distance and the height difference (b − a).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainNearPair {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub a: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub b: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub dz: f64,
}

/// A spike: the corner, the mean height round it, the difference (z − mean) and the corners round it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainSpike {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub mean: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub difference: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub neighbours: u32,
}

/// The coverage gaps: triangles with an edge over the limit, their plan area, the surface's longest edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainGaps {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub longest: f64,
}

/// A check point: its object, place, height, class, the surface's height there and the difference (point − surface);
/// none off the surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainCheckpoint {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
    pub class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub surface_z: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub difference: Option<f64>,
    /// Over three NMADs from its class's median (docs/adr/0945): flagged, not left out.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub far: bool,
}

/// A class's statistics over its points on the surface: how many on and off it, the mean, the standard deviation
/// (n − 1), the RMSE, the least and the largest difference.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainCheckClass {
    pub class: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub mean: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub std: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rmse: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub min: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub max: f64,
    /// Beyond one RMSE, for two points or more (docs/adr/0945).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub robust: Option<TerrainCheckRobust>,
}

/// A class's accuracy beyond its RMSE (docs/adr/0945): the RMSE's 95 % interval (the differences normal and unbiased),
/// the median, the NMAD (1.4826 × the median absolute deviation from the median), the 68.3 % and 95 % quantiles of the
/// absolute differences, the skewness and excess kurtosis (0 when normal), the points over three NMADs from the median,
/// and with the check points' own RMSE the two combined, √(RMSE² + σ²).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainCheckRobust {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rmse_low: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rmse_high: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub median: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub nmad: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub q68: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub q95: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub skewness: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub kurtosis: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub far: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub combined: Option<f64>,
}

/// The check points: each one, each class's statistics (by name) and all of them together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainCheckpoints {
    pub points: Vec<TerrainCheckpoint>,
    pub classes: Vec<TerrainCheckClass>,
    pub all: TerrainCheckClass,
}

/// Output of `road.terrains.quality` v1: the surface's corners and triangles and what was asked; the lists hold at
/// most [`MOST_QUALITY_ITEMS`], their counts are whole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsQualityRead {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub corners: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub near_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerrainNearPair>>", optional))]
    pub near: Vec<TerrainNearPair>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub spike_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerrainSpike>>", optional))]
    pub spikes: Vec<TerrainSpike>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub gaps: Option<TerrainGaps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub checkpoints: Option<TerrainCheckpoints>,
}

pub const ROAD_TERRAINS_REPAIR: &str = "road.terrains.repair";
pub const ROAD_TERRAINS_REPAIR_VERSION: u32 = 1;

/// Input of `road.terrains.repair` v1 (ROAD-TIN-17): point records left out of a surface (`exclude`, places exactly as
/// the records': a spike, a shot twice) or taken back (`restore`), the drawing's points untouched; a breakline's
/// vertex is never left out. The plan says the difference it makes before anything is written; one undo step “Yüzeyi
/// onar”. A surface accepted before stays accepted.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_repair` (nothing asked, a place not finite, a place in both
/// lists or twice), `unknown_terrain`, `invalid_repair` (a place to restore not left out, to exclude already left
/// out), `repair_breakline`, `repair_not_a_point`, `terrain_unbuildable`, `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsRepair {
    pub surface: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub exclude: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub restore: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::cad::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.terrains.repair` v1: the surface's id, the places left out now, its corners and triangles before
/// and after, and the difference (after − before, over the repaired plan): cut and fill volumes, net, the plan area
/// that changed and the largest change and where.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsRepaired {
    pub surface: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub excluded: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub corners_before: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub corners_after: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles_before: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles_after: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub net: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub changed_area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub largest: Option<TerrainDifferenceAt>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::cad::REVISION_TEXT)))]
    pub revision: String,
}

pub const ROAD_TERRAINS_COVERAGE: &str = "road.terrains.coverage";
pub const ROAD_TERRAINS_COVERAGE_VERSION: u32 = 1;

/// Input of `road.terrains.coverage` v1 (docs/adr/0920): how much of a project boundary a terrain surface covers. The
/// boundary is the drawing's closed object `boundary` (a closed polyline or an area, straight edges, its holes counting
/// out); absent, the surface itself. Triangles with a plan edge longer than `longest` (the user's) are estimated, not
/// measured: they span too wide a gap between shots. Reads only.
///
/// Refusals, in this order: `invalid_coverage` (longest not positive), `not_a_road_project`, `unknown_terrain`,
/// `terrain_unbuildable`, `entity_not_found`, `invalid_coverage` (a boundary that is not a closed polyline or area of
/// one part with straight edges and three corners at least). Warnings: `terrain_no_data` (the boundary is not wholly
/// covered).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsCoverage {
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub longest: Option<f64>,
}

/// Output of `road.terrains.coverage` v1: areas in plan. The surface's whole area; with a boundary, the boundary's,
/// the surface inside it, the no-data rest and the surface outside, and the share covered (0–1); the estimated part of
/// what is inside (with `longest`), its share of it and its triangles; the longest plan edge of a triangle inside.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsCovered {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub surface_area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub boundary_area: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub no_data: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub outside: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub share: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub estimated: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub estimated_share: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub estimated_triangles: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub longest_edge: f64,
}

pub const ROAD_TERRAINS_LOW_CONFIDENCE_DRAW: &str = "road.terrains.lowconfidence";
pub const ROAD_TERRAINS_LOW_CONFIDENCE_DRAW_VERSION: u32 = 1;

/// Input of `road.terrains.lowconfidence` v1 (docs/adr/0920 §5): a terrain surface's estimated areas, its triangles
/// with a plan edge longer than `longest` (the user's), drawn on `layerId` as closed polylines, their outlines (the
/// low confidence areas' 2D polygons); with `outline` the surface's own outline too (where data is), with `labels` each
/// estimated area's area at its outline's centre. One undo step “Düşük güvenli alanları çiz”.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `invalid_coverage` (longest not positive),
/// `not_a_road_project`, `unknown_terrain`, `terrain_unbuildable`, `layer_not_found`, `not_a_layer`, `layer_locked`.
/// Warnings: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsCoverageDraw {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub longest: f64,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub outline: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub labels: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::cad::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.terrains.lowconfidence` v1, and its plan: the surface, how many estimated areas (outer outlines)
/// and holes in them, their area, the objects written (in the plan, none) and the revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsCoverageDrawn {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub areas: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub holes: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    pub uids: Vec<String>,
    pub revision: String,
}

pub const ROAD_TERRAINS_DENSITY: &str = "road.terrains.density";
pub const ROAD_TERRAINS_DENSITY_VERSION: u32 = 1;

/// The most cells a density grid may have.
pub const MOST_DENSITY_CELLS: u32 = 1_000_000;

/// Input of `road.terrains.density` v1 (docs/adr/0920 §6; ROAD-SURVEY-08): a terrain surface's corners counted in
/// square cells of `cell` metres (the user's; USGS's density raster takes twice the nominal point spacing), whole
/// multiples of the cell from the world's origin. A cell counts when its centre is inside `boundary` (a closed object as
/// in `road.terrains.coverage`, an area's holes out) or, without one, on the surface. With `least`, the cells with fewer
/// points are sparse; with `share` (0–1, the user's; USGS asks 0.9 of the cells to hold a point) the answer says whether
/// that share of the cells holds a point. Reads only.
///
/// Refusals, in this order: `invalid_point_density` (cell not positive, least 0, share outside (0, 1]),
/// `not_a_road_project`, `unknown_terrain`, `terrain_unbuildable`, `entity_not_found`, `invalid_coverage` (a boundary
/// that is not one), `invalid_point_density` (more than [`MOST_DENSITY_CELLS`] cells). Warnings: `terrain_density_short`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsDensity {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub least: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub share: Option<f64>,
}

/// Output of `road.terrains.density` v1: the grid (its cell, the lower left corner of column 0 row 0, columns and
/// rows), the cells that count, those holding a point and those empty, the share holding one (none without cells),
/// the sparse cells (with `least`), the points in the cells that count, their area (cells × cell²), the mean density
/// (points per m²; none without area), the most points in a cell, and whether the share asked is met (with `share`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsDensityRead {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub origin: crate::Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub columns: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub cells: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub filled: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub empty: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub share: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub sparse: Option<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1/m2")))]
    pub mean: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub most: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub meets: Option<bool>,
}

pub const ROAD_TERRAINS_SPARSE_DRAW: &str = "road.terrains.sparse";
pub const ROAD_TERRAINS_SPARSE_DRAW_VERSION: u32 = 1;

/// Input of `road.terrains.sparse` v1 (docs/adr/0920 §7): a density grid's (as in `road.terrains.density`) cells with
/// fewer than `least` points (the user's) drawn on `layerId` as closed polylines, their merged outlines (cells meeting
/// at an edge one area), with `labels` each area's area at its outline's centre. One undo step “Seyrek alanları çiz”.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `invalid_point_density`, `not_a_road_project`,
/// `unknown_terrain`, `terrain_unbuildable`, `entity_not_found`, `invalid_coverage`, `invalid_point_density` (too many cells),
/// `layer_not_found`, `not_a_layer`, `layer_locked`. Warnings: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsSparseDraw {
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub least: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary: Option<String>,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub labels: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::cad::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}
