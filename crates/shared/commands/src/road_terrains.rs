//! KentOS Yol's terrain surface commands (plan/road.md Y2.3, docs/adr/0900 §11): `road.terrains.create` makes a
//! surface from the drawing's points, breaklines, boundary and voids and keeps its definition in the road project's
//! settings; `road.terrains.edit` changes a definition, accepts a surface made again from its sources, or removes
//! one; `road.terrains.query` reads a surface's state, figures, heights at places and section along a line (Civil
//! 3D's `FindElevationAtXY` and `SampleElevations`, OpenRoads' Analyze Point and profile of a line). The surface's
//! triangles are made by `kentos-road` (`kentos.cdt/1`) from the drawing every time; a definition is a project
//! setting, as a network is: writing it makes the drawing dirty and is no undo step. Only a road project
//! (`cadWork: road`) has them. The desktop's handlers are `crates/native/application/src/road_terrains.rs`; they and
//! the Python SDK pass the shared cases in `fixtures/commands/v1/road.terrains.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{SurfaceSources, TerrainSurfaceDef};

pub const ROAD_TERRAINS_CREATE: &str = "road.terrains.create";
pub const ROAD_TERRAINS_CREATE_VERSION: u32 = 1;
pub const ROAD_TERRAINS_EDIT: &str = "road.terrains.edit";
pub const ROAD_TERRAINS_EDIT_VERSION: u32 = 1;
pub const ROAD_TERRAINS_QUERY: &str = "road.terrains.query";
pub const ROAD_TERRAINS_QUERY_VERSION: u32 = 1;

/// The most places a query names (its `at` and `along` together).
pub const MAX_TERRAIN_PLACES: usize = 100_000;

/// Input of `road.terrains.create` v1: a new surface's definition; its id when not given is the first free of `t-1`,
/// `t-2` …
///
/// Refusals, checked in this order: `not_a_road_project`, `invalid_terrain` (the definition, or the project's surfaces
/// with it, by `road_problem`; path the field's), `invalid_revision`, `revision_conflict`, `terrain_unbuildable` (no
/// surface is made of what the sources give: too few points, two heights at one place, crossing breaklines, a boundary
/// outside; the message says where). What the definition names and the drawing cannot give is the warning
/// `terrain_source_left`, one for each, by the object's or the layer's id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsCreate {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "SurfaceSources::is_empty")]
    pub points: SurfaceSources,
    #[serde(default, skip_serializing_if = "SurfaceSources::is_empty")]
    pub breaklines: SurfaceSources,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub voids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub max_edge: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `road.terrains.edit` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TerrainsEditOperation {
    /// `surface` replaces the project's definition of its id; it keeps the digest it was last accepted from, so a
    /// changed definition is out of date until accepted.
    Set,
    /// The surface of `id`, made again from its sources, is accepted: its digest becomes the sources'.
    Accept,
    /// The surface of `id` goes.
    Remove,
}

/// Input of `road.terrains.edit` v1.
///
/// Refusals, in this order: `not_a_road_project`, `no_terrain` (set without `surface`), `no_id` (accept or remove
/// without `id`), `invalid_terrain` (set: the definition, given without `built`, or the surfaces with it),
/// `invalid_revision`, `revision_conflict`, `unknown_terrain` (no surface of the id), `terrain_unbuildable` (accept).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsEdit {
    pub operation: TerrainsEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<TerrainSurfaceDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Input of `road.terrains.query` v1: every surface's state and figures, or one's (`surface`: its id or name) with its
/// heights at places (`at`) and its section along a line (`along`, two places or more).
///
/// Refusals: `not_a_road_project`, `unknown_terrain`, `no_terrain` (`at` or `along` without `surface`),
/// `invalid_places` (a place not finite, a line of one place, more than 100 000 places).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub along: Vec<[f64; 2]>,
}

/// A surface's state against its sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TerrainState {
    /// Made from the sources it was last accepted from.
    Current,
    /// Its sources, or its definition, changed since.
    Stale,
    /// Never accepted.
    New,
    /// No surface is made of what its sources give now (`problem` says why).
    Unbuildable,
}

/// What a surface names and could not use, and why: `missing` (not in the drawing), `no_height`, `arc_edge`,
/// `wrong_kind`, `not_closed`, `too_few`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainLeft {
    /// The object's or the layer's id.
    pub id: String,
    /// `points`, `breaklines`, `boundary`, `voids`.
    pub role: String,
    pub why: String,
}

/// Why no surface is made: the core's code (`too_few_points`, `height_ambiguous`, `height_conflict`,
/// `constraints_cross`, `boundary_outside` …), its words and its place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainProblem {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<[f64; 2]>,
}

/// A surface's figures: corners, kept triangles, constrained edges (breaklines' and rings' pieces), plan area and the
/// kept corners' lowest and highest heights.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainFigures {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub vertices: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub constrained: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub min_z: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub max_z: Option<f64>,
    /// Records at one place within the tolerance, made one corner.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub merged: u32,
    /// Breakline pieces split at a corner on them.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub split: u32,
    /// Outer triangles left out by the edge method.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub trimmed: u32,
}

/// Where a place is on a surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TerrainPlace {
    Inside,
    Outside,
    Void,
}

/// A place's height: none outside and in a void.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainHeight {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: Option<f64>,
    pub place: TerrainPlace,
}

/// A place of a section: its distance from the line's start, where it is, its height (none off the surface), and
/// `gap`: the line from the previous place to this one is off the surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainSectionPoint {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub gap: bool,
}

/// A surface as its sources give it now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainReport {
    pub id: String,
    pub name: String,
    pub state: TerrainState,
    /// The sources' digest now.
    pub digest: String,
    /// The digest it was last accepted from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub built: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerrainLeft>>", optional))]
    pub left: Vec<TerrainLeft>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub figures: Option<TerrainFigures>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<TerrainProblem>,
}

/// Output of `road.terrains.create` and `.edit` v1, and their plan: the project's surfaces as the write leaves them,
/// the one written or accepted as its sources give it (none for a remove), and the revision after it (a plan: before).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsWritten {
    pub surfaces: Vec<TerrainSurfaceDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<TerrainReport>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Output of `road.terrains.query` v1: the surfaces' reports (one when `surface` is given), and that one's heights
/// and section.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsQueried {
    pub surfaces: Vec<TerrainReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerrainHeight>>", optional))]
    pub heights: Vec<TerrainHeight>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TerrainSectionPoint>>", optional))]
    pub section: Vec<TerrainSectionPoint>,
}
