//! A point cloud's source metadata and its points by a lasting id (docs/adr/0536; plan/gis.md `CBS-32`).
//!
//! `cad.pointclouds.describe` reads each source file's header of the drawing's point clouds and says what it holds:
//! - the file's kind and version, its point format and count, bounds, scale and offset;
//! - its systems: the horizontal one and the vertical one as the file names them;
//! - its acquisition (the header's creation date, which GPS time the points carry), the system and software that
//!   wrote it;
//! - the dimensions every point has (the format's and the extra bytes' with their types), the points by return;
//! - with `classes`, every point's classification counted, the whole file read.
//!
//! It writes nothing. A source that cannot be read (not found, an address, broken) is said with why; the rest are
//! read.
//!
//! `cad.pointclouds.query` finds the point nearest a place in plan within a reach, read at the source's full
//! resolution (never a drawing's level of detail). It answers the point by a lasting id: the cloud's persistent id,
//! the source's place in the cloud and the point's place in the source's order (LAS and LAZ: the file's; COPC: its
//! nodes by key, each node's points in order). The same file gives the same id whenever it is read.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Vec2;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_POINTCLOUDS_DESCRIBE: &str = "cad.pointclouds.describe";
pub const CAD_POINTCLOUDS_DESCRIBE_VERSION: u32 = 1;
pub const CAD_POINTCLOUDS_QUERY: &str = "cad.pointclouds.query";
pub const CAD_POINTCLOUDS_QUERY_VERSION: u32 = 1;

/// The most points `classes` counts in one call, all the sources together.
pub const POINTCLOUD_MOST_COUNTED: u64 = 200_000_000;

/// Input of `cad.pointclouds.describe` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_pointcloud`,
/// `too_many_points`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsDescribe {
    /// The point clouds (persistent ids); none: every point cloud of the drawing, in the drawing's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    /// Count every point's classification (each source read whole).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub classes: bool,
    /// The open drawing's file, a relative source's folder (the host gives it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// An extra dimension of a source's points (LAS's extra bytes, LASF_Spec 4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudExtra {
    pub name: String,
    /// `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `f32`, `f64`, or `bytes` (an undocumented one).
    pub kind: String,
    /// Its bytes in each point.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub size: u32,
}

/// One source of a point cloud as its header says it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudSourceDescribed {
    /// The source as the cloud names it: its file, address or library asset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// Whether its header was read; when not, `why`.
    pub read: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub why: Option<String>,
    /// `las`, `laz`, `copc`, or `text` (XYZ, PTS, TXT, CSV: a text cloud has no header, its count and bounds are the
    /// cloud's own record).
    pub kind: String,
    /// `1.2`, `1.4`; none for a text cloud.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub format: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub record_len: Option<u16>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u64,
    /// `[x₁, y₁, z₁, x₂, y₂, z₂]` in the file's system.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub bounds: [f64; 6],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub scale: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub offset: Option<[f64; 3]>,
    /// The horizontal system's EPSG code, when the file names one KentOS can tell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub epsg: Option<u32>,
    /// The horizontal system's name as the file writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crs_name: Option<String>,
    /// The vertical system as the file names it: its name (a WKT's `VERT_CS`/`VERTCRS`) or `EPSG:n` (a GeoTIFF key);
    /// none when the file names none (the heights' datum then unknown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vertical: Option<String>,
    /// The header's creation day as `YYYY-MM-DD`; none when the file leaves it 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created: Option<String>,
    /// Which GPS time the points carry: `standard` (adjusted standard GPS time) or `week` (seconds of the GPS week);
    /// none when the format has no time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub gps_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub software: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub source_id: Option<u16>,
    /// The project's GUID as the header holds it (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`); none when it is zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub guid: Option<String>,
    /// The dimensions every point has, the format's in its order (`x`, `y`, `z`, `intensity`, …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub dimensions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CloudExtra>>", optional))]
    pub extra: Vec<CloudExtra>,
    /// The points by return number (the header's), up to the last that has any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(type = "Array<number>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub by_return: Vec<u64>,
    /// A COPC's nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub nodes: Option<u32>,
    /// With `classes`: each classification code (as text) and its points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, number>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub classes: Option<BTreeMap<String, u64>>,
}

/// A point cloud described.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudDescribed {
    pub uid: String,
    pub layer: String,
    /// The cloud's system in the project (0: none).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u64,
    pub sources: Vec<CloudSourceDescribed>,
}

/// Output of `cad.pointclouds.describe` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsDescribed {
    pub clouds: Vec<CloudDescribed>,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.pointclouds.query` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_pointcloud`,
/// `invalid_radius`, `invalid_point`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsQuery {
    /// The place in plan.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// The most distance in plan to the point, above 0.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub radius: f64,
    /// The point clouds looked in; none: every visible one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// The point found: its lasting id and its record's facts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudPoint {
    /// The cloud's persistent id.
    pub cloud: String,
    /// The source's place in the cloud's sources (0 and up).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub source: u32,
    /// The point's place in the source's order (LAS, LAZ: the file's; COPC: nodes by key).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub z: f64,
    /// Its distance in plan from `at`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub class: u8,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub intensity: u16,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub return_number: u8,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub number_of_returns: u8,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub point_source_id: u16,
    /// The point's GPS time, as the source's `gpsTime` says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub gps_time: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub rgb: Option<[u16; 3]>,
}

/// Output of `cad.pointclouds.query` v1: the nearest point, or none within the reach; the points looked at.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsQueried {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub point: Option<CloudPoint>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
