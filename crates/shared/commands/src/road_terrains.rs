//! KentOS Yol's terrain surface commands (plan/road.md Y2.3, docs/adr/0900 §11): `road.terrains.create` makes a
//! surface from the drawing's points, breaklines, boundary and voids and keeps its definition in the road project's
//! settings; `road.terrains.edit` changes a definition, accepts a surface made again from its sources, or removes
//! one; `road.terrains.query` reads a surface's state, figures, heights at places and section along a line (Civil
//! 3D's `FindElevationAtXY` and `SampleElevations`, OpenRoads' Analyze Point and profile of a line), with the slope and
//! aspect; `road.terrains.contours` writes its contour lines (plan/road.md Y2.5). The surface's
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
pub const ROAD_TERRAINS_CONTOURS: &str = "road.terrains.contours";
pub const ROAD_TERRAINS_CONTOURS_VERSION: u32 = 1;
pub const FILE_IMPORT_LANDXML: &str = "file.import.landxml";
pub const FILE_IMPORT_LANDXML_VERSION: u32 = 1;

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

/// Input of `road.terrains.contours` v1 (plan/road.md Y2.5): a surface's contour lines written as 3D polylines on a
/// layer, every `interval` metres from `base` (0 when absent), every `majorEvery`-th level from the base a major line
/// (thicker; 5 when absent, 0: none), as Civil 3D extracts a surface's major and minor contours. A line through a
/// corner is drawn once; a closed line is a closed polyline. One undo step “Eş yükselti eğrileri”.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_contours` (an interval not above zero, a base not finite),
/// `layer_not_found`, `not_a_layer`, `layer_locked`, `invalid_revision`, `revision_conflict`, `unknown_terrain`,
/// `terrain_unbuildable`, `invalid_contours` (more than 10 000 levels cross the surface), `no_contours` (none does).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsContours {
    /// The surface's id or name.
    pub surface: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub interval: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub base: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub major_every: Option<u32>,
    /// The layer they go on.
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A contour line as written: its level, whether major and closed, its places.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainContour {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
    pub major: bool,
    pub closed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub points: Vec<[f64; 2]>,
}

/// Output of `road.terrains.contours` v1, and its plan: the lines in their order (by level, then by their first
/// place), the objects written (a plan: none) and the revision after it (a plan: before).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsContoured {
    pub lines: Vec<TerrainContour>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
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
    /// The plane's rise over run, inside one triangle only (on an edge or a corner two planes meet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub slope: Option<f64>,
    /// The azimuth it falls towards, radians clockwise from north; none on a flat triangle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub aspect: Option<f64>,
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

/// Input of `file.import.landxml` v1 (plan/road.md Y2.5, docs/adr/0900 §14): a LandXML file's TIN surface taken into a
/// road project: its points as point objects, its breaklines as 3D polylines, its outer boundary and voids as areas on
/// one layer (one undo step “LandXML al”), and a terrain surface of them (`road.terrains.create`'s definition, a project
/// setting). The file's faces are not kept: the surface is made again from its points and breaklines, and the answer
/// says how far the two differ. Coordinates turn from the file's unit into metres, “northing easting” into (x, y).
///
/// Refusals: `not_a_road_project`, `file_not_found`, `import_unreadable` (not LandXML, a number that does not read),
/// `landxml_no_surface` (no surface, or none of `surface`'s name, or one without points), `layer_not_found`,
/// `not_a_layer`, `layer_locked`, `invalid_terrain` (the surface's name), `invalid_revision`, `revision_conflict`,
/// `terrain_unbuildable`. Warnings: `landxml_left` (faces naming a missing point, a point id twice, a second outer
/// boundary), `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportLandxml {
    /// The file's path.
    pub path: String,
    /// The layer the objects go on.
    pub layer_id: String,
    /// The file's surface by its name; absent: its first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    /// The terrain surface's name; absent: the file's surface's (or “LandXML”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `file.import.landxml` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LandxmlImported {
    /// The file's surface taken.
    pub surface: String,
    /// The file's linear unit as it writes it (empty: none said, metres taken).
    pub unit: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub breaklines: u32,
    pub boundary: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub voids: u32,
    /// The file's faces it does not ignore, and those it ignores.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub faces: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ignored: u32,
    /// The greatest height difference at the file's faces' centroids between the file's face and the surface made
    /// again (m); none without faces or with no centroid on the surface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub differs: Option<f64>,
    /// The terrain surface as its sources give it.
    pub terrain: TerrainReport,
    /// The objects written (a plan: none).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
