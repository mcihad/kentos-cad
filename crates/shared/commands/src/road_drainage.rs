//! Surface drainage (plan/road.md Y16, docs/adr/0929; ROAD-DRAIN-03, 05): `road.drainage.query` reads a terrain
//! surface's ponds, the water drops from places, and a discharge point's catchment with its Rational peak flow and a
//! culvert's full-flow capacity against it; `road.drainage.draw` draws the catchment's outline, its flow path and the
//! ponds' bottoms on a layer. The geometry is `kentos_road::drainage`'s; nothing is kept: every read computes again.
//! The desktop's handlers are `crates/native/application/src/road_drainage*.rs`; they and the Python SDK pass the shared
//! cases in `fixtures/commands/v1/road.drainage.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const ROAD_DRAINAGE_QUERY: &str = "road.drainage.query";
pub const ROAD_DRAINAGE_QUERY_VERSION: u32 = 1;
pub const ROAD_DRAINAGE_DRAW: &str = "road.drainage.draw";
pub const ROAD_DRAINAGE_DRAW_VERSION: u32 = 1;

/// The most water drops one query reads, and the most cells of a catchment's grid.
pub const MOST_DRAINAGE_DROPS: usize = 1000;
pub const MOST_CATCHMENT_CELLS: u64 = 1_000_000;

/// The ponds asked for: at least `minDepth` deep (OpenRoads' minimum depth).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainagePondsAsk {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub min_depth: f64,
}

/// A catchment asked for: the discharge point (`at`, or a culvert's inlet), the grid's cell (m) and the radius a drop
/// must pass within (m); with `runoff` (C, 0–1) and `intensity` (mm/h) the Rational peak flow, and with `culvert` its
/// full-flow capacity against it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainageCatchmentAsk {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<[f64; 2]>,
    /// A culvert by its id or name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub culvert: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub cell: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub radius: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub runoff: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rainfall")))]
    pub intensity: Option<f64>,
}

/// Input of `road.drainage.query` v1: the terrain surface (its id or name) and what is asked of it: `ponds`, `drops`
/// (places in plan) and `catchment`. Nothing is filled in: the depth, the cell, the radius, C and i are the user's.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_drainage` (nothing asked, more than [`MOST_DRAINAGE_DROPS`]
/// drops, a value not finite or not positive, C outside 0–1, C or i without the other, a catchment without `at` or a
/// culvert), `unknown_terrain`, `unknown_culvert`, `terrain_unbuildable`, the culvert's `invalid_culvert` (its inlet is
/// read from its corridor), `invalid_drainage` (a grid of more than [`MOST_CATCHMENT_CELLS`] cells). Warnings:
/// `drainage_catchment_empty`, `culvert_no_roughness`, `culvert_over_capacity`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainageQuery {
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ponds: Option<DrainagePondsAsk>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub drops: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub catchment: Option<DrainageCatchmentAsk>,
}

/// A pond: its bottom, the level it spills at and the corner it spills over, its depth, its water's area and volume and
/// how many corners are under it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainagePond {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bottom: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub level: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub outlet: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub depth: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub corners: u32,
}

/// Why a water drop ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DrainageDropEnd {
    LowPoint,
    Flat,
    Edge,
    Outside,
    Steps,
}

/// A water drop: where it started, its path with heights, its last place, its plan length and why it ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainageDrop {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub path: Vec<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Option<[f64; 3]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    pub end: DrainageDropEnd,
}

/// A culvert's full-flow capacity by Manning: every barrel full at the culvert's slope and roughness; its discharge
/// (m³/s), its velocity (m/s), and the peak flow over it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainageCapacity {
    pub culvert: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "discharge")))]
    pub discharge: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "velocity")))]
    pub velocity: f64,
    /// The peak flow over the capacity; absent without a peak flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub ratio: Option<f64>,
}

/// A catchment: its discharge point, its cells at the grid's cell and its area, its outline rings, the flow path from
/// its most distant cell with its length, fall and mean slope (m/m), its Rational peak flow (m³/s) and a culvert's
/// capacity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainageCatchment {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub cells: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub outline: Vec<Vec<[f64; 2]>>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub path: Vec<[f64; 3]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub fall: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub slope: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "discharge")))]
    pub peak: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub capacity: Option<DrainageCapacity>,
}

/// Output of `road.drainage.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainageQueried {
    pub surface: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DrainagePond>>", optional))]
    pub ponds: Vec<DrainagePond>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DrainageDrop>>", optional))]
    pub drops: Vec<DrainageDrop>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub catchment: Option<DrainageCatchment>,
}

/// Input of `road.drainage.draw` v1: a query's catchment (its outline as closed polylines, its flow path as a 3D
/// polyline) and ponds (a point at each bottom), drawn on `layerId`; with `labels` the catchment's area and peak flow
/// and each pond's depth. One undo step “Drenaj çiz”.
///
/// Refusals: the query's, `drainage_nothing` (no catchment and no pond to draw), the layer's, `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainageDraw {
    pub query: DrainageQuery,
    pub layer_id: String,
    #[serde(default)]
    pub labels: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.drainage.draw` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrainageDrawn {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
