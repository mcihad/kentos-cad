//! `road.terrains.stockpile` (plan/map-cad.md MAP-15, docs/adr/3140, 3940): a stockpile's volume over a base drawn from
//! its boundary, Pix4D's volume base surfaces. Reads only.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const ROAD_TERRAINS_STOCKPILE: &str = "road.terrains.stockpile";
pub const ROAD_TERRAINS_STOCKPILE_VERSION: u32 = 1;

/// The bases a stockpile's volume is measured from.
pub const STOCKPILE_BASES: [&str; 6] = ["lowest", "highest", "average", "fit", "triangulated", "elevation"];

/// Input of `road.terrains.stockpile` v1 (docs/adr/3140): the surface (its id or name), the boundary (a closed object
/// as in `road.terrains.coverage`: straight edges, one part, an area's holes out) and the base: `lowest`, `highest` or
/// `average` level at that of the boundary's outer corners' heights on the surface, `fit` the plane fitted to them
/// (the least squares of the vertical differences), `triangulated` the corners with their heights triangulated inside
/// the outer ring (docs/adr/3940: the core's constrained Delaunay triangulation, each triangle its plane),
/// `elevation` level at `elevation`. Above the base is cut (the pile), below it fill (Pix4D's words). Reads only.
///
/// Refusals, in this order: `invalid_stockpile` (a base not one of these, an elevation missing or not finite),
/// `not_a_road_project`, `unknown_terrain`, `terrain_unbuildable`, `entity_not_found`, `invalid_coverage` (a boundary
/// that is not one), `invalid_stockpile` (an outer corner off the surface for a base drawn from them, the corners on a
/// line for `fit`, the corners not triangulated). Warnings: `terrain_no_data` (the surface does not cover the whole
/// boundary).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsStockpile {
    pub surface: String,
    pub boundary: String,
    pub base: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: Option<f64>,
}

/// Output of `road.terrains.stockpile` v1: the surface's id, the base and its plane `z = a·x + b·y + c` (level: a and
/// b zero; a triangulated base has none but its triangles), the outer corners' heights on the surface (none when a
/// level base is given and a corner is off it), the boundary's plan area, the part over the surface and the rest, the
/// cut (above the base) and fill (below it) volumes and their total (Pix4D's: cut + fill), the plan areas of cut and
/// fill, and notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsStockpiled {
    pub surface: String,
    pub base: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub plane: Option<[f64; 3]>,
    /// A triangulated base's triangles (docs/adr/3940): each three corners `[x, y, z]` counterclockwise from its
    /// smallest place (x, then y), the list ordered by them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[[f64; 3]; 3]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub triangles: Vec<[[f64; 3]; 3]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub corners: Vec<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub uncovered: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub total: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub cut_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub fill_area: f64,
    pub notes: Vec<String>,
}
