//! A terrain's points from point clouds (plan/road.md Y15, docs/adr/0913; ROAD-PC-04, 10, 11): `road.terrains.cloud`
//! reads the chosen clouds once, keeps the points of the classes and returns asked (in an area, if one is given), thins
//! them on a grid (`kentos_road::cloud_ground`), writes the kept ones on a layer as multi-point objects with their
//! heights and makes that layer a terrain surface's point source (a new surface, or one of the project's), recording on
//! the surface how they were chosen and what the thinning cost. The desktop's handler is
//! `crates/native/application/src/road_terrain_cloud.rs`; it and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.terrains.cloud.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{CloudReturns, SurfaceCloudDef, SurfaceSources, TerrainReport};

pub const ROAD_TERRAINS_FROM_CLOUD: &str = "road.terrains.cloud";
pub const ROAD_TERRAINS_FROM_CLOUD_VERSION: u32 = 1;

/// The most points one call keeps; a cloud of more is thinned more or taken in pieces.
pub const MOST_CLOUD_TERRAIN_POINTS: u64 = 2_000_000;
/// The points a written multi-point object holds at most.
pub const CLOUD_TERRAIN_PIECE: usize = 10_000;

/// Input of `road.terrains.cloud` v1: the clouds (their objects' ids), the LAS classes taken (2 is the ASPRS
/// ground), the returns, an optional area (a closed polyline's or an area's id), the thinning (`cell` m; `zTolerance`
/// m: a steeper cell keeps its lowest and highest; `near` m: every point within it of the surface's breaklines is
/// kept), the layer the points go on, and either `surface` (a project's surface by its id or name: the layer joins its
/// points) or `name` (a new surface of the layer, with `breaklines` and `boundary` if given). Nothing is filled in: the
/// classes, the cell and the tolerances are the user's.
///
/// Refusals, in this order: `not_a_road_project`, `not_a_pointcloud`, `invalid_cloud_terrain` (no cloud or class, both
/// or neither of surface and name, a thinning value not positive, an area not a closed polyline or area, more kept
/// points than [`MOST_CLOUD_TERRAIN_POINTS`]), `cloud_class_not_ground` (a bridge deck, 17, or an overhead structure,
/// 19: they are not the ground and are not merged into it), `unknown_terrain`, the layer's, `too_many_cloud_points`,
/// `cloud_ground_empty` (no point is of the classes and returns in the area), `invalid_terrain`, `terrain_unbuildable`,
/// `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsFromCloud {
    pub clouds: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub classes: Vec<u8>,
    #[serde(default)]
    pub returns: CloudReturns,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub cell: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub z_tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub near: Option<f64>,
    pub layer_id: String,
    /// A project's surface by its id or name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    /// A new surface's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// A new surface's breaklines.
    #[serde(default, skip_serializing_if = "SurfaceSources::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SurfaceSources>", optional))]
    pub breaklines: SurfaceSources,
    /// A new surface's boundary: a closed polyline's or an area's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary: Option<String>,
    /// The open drawing's file, a relative source's folder (the host gives it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.terrains.cloud` v1: the points read by class (what the clouds held of each class asked), the
/// record written on the surface (read, kept, coverage, density, error), the kept points by why (a cell's, a steep
/// cell's, beside a breakline), the objects written, and the surface made now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsFromCloudWritten {
    pub cloud: SurfaceCloudDef,
    /// The points of the clouds read (every class).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub scanned: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub by_cell: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub by_slope: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub by_line: u64,
    /// The area's cells and those holding a ground point.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub cells: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub occupied: u64,
    /// The area's plan area, m².
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    pub uids: Vec<String>,
    pub surface: TerrainReport,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
