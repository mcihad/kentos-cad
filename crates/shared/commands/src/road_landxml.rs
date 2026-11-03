//! KentOS Yol's LandXML export (plan/road.md Y10.2, docs/adr/0908; ROAD-IO-05): `file.export.landxml` writes a road
//! project's terrain surfaces (as TINs), its horizontal alignments (lines, circular curves and clothoids, with their km
//! equations) and their vertical alignments (PVIs with their curves) to a LandXML 1.2 file, whole or not at all. It does
//! not change the drawing. The writer is `kentos-formats`' `landxml_write`; the desktop's handler is
//! `crates/native/application/src/road_landxml.rs`; it and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/file.export.landxml.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const FILE_EXPORT_LANDXML: &str = "file.export.landxml";
pub const FILE_EXPORT_LANDXML_VERSION: u32 = 1;

/// Input of `file.export.landxml` v1: the file's path (absolute, ending `.xml`); the surfaces and the alignments by id
/// or name (absent: every one; an empty list: none); `withoutProfiles` leaves the alignments' profiles out; `stamp` the
/// file's date and time (`YYYY-MM-DDThh:mm:ss`; absent: now, UTC).
///
/// Refusals, in this order: `not_a_road_project`, `invalid_file_path` (empty, relative, not `.xml`, its folder not
/// there), `file_exists` (without `overwrite`), `unknown_terrain`, `unknown_alignment`, `invalid_landxml` (a stamp that
/// does not read, nothing to write, a surface or an alignment chosen that does not build). Warnings: `export_losses`
/// (a parallel axis, an alignment or a surface that does not build when every one is written, a profile whose
/// alignment is not written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportLandxml {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surfaces: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alignments: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub without_profiles: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stamp: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub overwrite: bool,
}

/// A written surface: its name and its points and triangles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LandxmlSurfaceWritten {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles: u32,
}

/// A written alignment: its name, its elements by kind, its km equations and the profiles written on it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LandxmlAlignmentWritten {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub curves: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub spirals: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub equations: u32,
    pub profiles: Vec<String>,
}

/// Output of `file.export.landxml` v1, and its plan: what is (or would be) in the file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LandxmlExported {
    pub path: String,
    pub surfaces: Vec<LandxmlSurfaceWritten>,
    pub alignments: Vec<LandxmlAlignmentWritten>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}
