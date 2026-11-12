//! A point cloud's statistics in an area and a profile along a line, at the source's full resolution (docs/adr/0552;
//! plan/gis.md `CBS-32`).
//!
//! `cad.pointclouds.region` answers what the points inside a polygon (with holes) are: their count, bounds, heights,
//! intensities and classes, and the points per plan unit of area. `cad.pointclouds.profile` answers the same along a
//! line: the points within half a width of a path, binned by station (the distance along the path), and, asked, the
//! points themselves with their station and signed offset.
//!
//! Both read the files themselves, never a drawing's level of detail: COPC reads only the nodes the area (or a
//! segment of the corridor) touches, LAS and LAZ read the whole file when its box meets the area. The answer says
//! how many points were read. Neither writes anything.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Vec2;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_POINTCLOUDS_REGION: &str = "cad.pointclouds.region";
pub const CAD_POINTCLOUDS_REGION_VERSION: u32 = 1;
pub const CAD_POINTCLOUDS_PROFILE: &str = "cad.pointclouds.profile";
pub const CAD_POINTCLOUDS_PROFILE_VERSION: u32 = 1;

/// The most stations a profile has.
pub const PROFILE_MOST_STATIONS: u64 = 100_000;
/// The most points a profile lists.
pub const PROFILE_MOST_LISTED: u64 = 100_000;

/// Input of `cad.pointclouds.region` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_pointcloud`,
/// `invalid_cloud_area`, `too_many_cloud_points`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsRegion {
    /// The area's outer ring in plan, at least three vertices; implicitly closed (a repeated last vertex is dropped).
    /// A point on the boundary is inside.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub ring: Vec<Vec2>,
    /// Holes of the area: a point strictly inside one is outside the area (its boundary is inside).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Vec<Vec2>>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub holes: Vec<Vec<Vec2>>,
    /// The point clouds looked in; none: every visible one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    /// The open drawing's file, a relative source's folder (the host gives it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A value's smallest, largest and mean.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudRange {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub min: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub max: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub mean: f64,
}

/// What the points inside an area are.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudStats {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u64,
    /// `[x₁, y₁, z₁, x₂, y₂, z₂]` of the points inside; none when there are none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bounds: Option<[f64; 6]>,
    /// The heights (the mean summed exactly).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<CloudRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub intensity: Option<CloudRange>,
    /// Each classification code (as text) and its points.
    #[cfg_attr(feature = "ts", ts(type = "Record<string, number>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub classes: BTreeMap<String, u64>,
}

/// One point cloud's share of an area.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudRegionPart {
    pub uid: String,
    /// The points of this cloud's sources looked at.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    pub stats: CloudStats,
}

/// Output of `cad.pointclouds.region` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsRegioned {
    /// The area: the ring's, less the holes' (the polygon is taken as simple).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    /// The points per unit of area, all the clouds together; none when there are none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub density: Option<f64>,
    /// The points looked at, all the clouds together.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    /// All the clouds together.
    pub total: CloudStats,
    /// Each cloud looked in, in the asked order (or the drawing's).
    pub clouds: Vec<CloudRegionPart>,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.pointclouds.profile` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_pointcloud`,
/// `invalid_corridor`, `too_many_stations`, `too_many_cloud_points`, `too_many_listed_points`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsProfile {
    /// The path in plan, at least two vertices, no two in a row the same.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub path: Vec<Vec2>,
    /// The corridor's width: the points within half of it of the path (a rounded end), above 0.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// The stations' step along the path, above 0.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub step: f64,
    /// Also list the points (station, offset, place, class), by station.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub points: bool,
    /// The point clouds looked in; none: every visible one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// One station of a profile: the points whose station lies in [`from`, `to`) (the last includes the path's end).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileBin {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: f64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u64,
    /// The heights; none when no point falls in the station.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<CloudRange>,
}

/// A point of the corridor: its lasting id, its place along the path and its facts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilePoint {
    pub cloud: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub source: u32,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u64,
    /// The distance along the path of the point's foot on the path.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub station: f64,
    /// The distance from the path, positive on its left.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub z: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub class: u8,
}

/// Output of `cad.pointclouds.profile` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsProfiled {
    /// The path's length.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    /// The points in the corridor, all the clouds together.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u64,
    /// The points looked at.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    /// Every station, empty ones too, from the path's start.
    pub bins: Vec<ProfileBin>,
    /// With `points`: the corridor's points by station, then by cloud, source and place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProfilePoint>>", optional))]
    pub points: Vec<ProfilePoint>,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
