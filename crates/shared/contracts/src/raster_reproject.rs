//! `cad.rasters.reproject` v1 (docs/adr/0644 §3; plan/gis.md `CBS-31`): a raster file in another coordinate system
//! carried into the project's, its pixels resampled onto a north-up grid of the project's system and written as a
//! GeoTIFF, then added to the drawing as a raster linked to it, as one undo step “Rasteri yeniden izdüşür”. Raster ekle
//! refuses a file in another system; this is its way in. The values stay (an 8-bit raster is not made RGBA); a
//! bilinear mean leaves out the cells of no value.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::RasterSample;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_RASTERS_REPROJECT: &str = "cad.rasters.reproject";
pub const CAD_RASTERS_REPROJECT_VERSION: u32 = 1;

/// The most cells a reprojected raster holds.
pub const REPROJECT_MOST_CELLS: u64 = 1 << 30;

/// How a pixel takes the source's values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReprojectSample {
    /// The source pixel under the centre: classes and quality bands keep their codes.
    #[default]
    Nearest,
    /// The mean of the four round the centre by their distances, the cells of no value left out.
    Bilinear,
}

/// Input of `cad.rasters.reproject` v1. Refusals, in this order: `invalid_file_path` (the file or `output`, or
/// `output` the source itself), `invalid_input` (`cell` not a positive number), `file_not_found`, `import_unreadable`,
/// `crs_unknown` (the file names no system KentOS knows and `srid` is not given, or `srid` is not one),
/// `crs_mismatch` (the project has no system, or the raster is in the project's already), `crs_unreached` (no corner
/// of the raster reaches the project's system), `invalid_input` (the grid too large for `cell`), `file_exists`, then
/// `layer_not_found` (not a layer, or a group), `layer_locked`, `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersReproject {
    /// The raster file (GeoTIFF, a PNG or JPEG with its world file …), absolute or from the drawing's folder.
    pub file: String,
    /// The file's coordinate system when it names none (or another than it names: `crs_overridden`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// The result's square pixel, in the project's units; none: the source's middle pixel's footprint in the
    /// project's system, to six significant figures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub cell: Option<f64>,
    #[serde(default)]
    pub sample: ReprojectSample,
    /// The GeoTIFF written, absolute or from the drawing's folder; none: beside the file as `<name>-epsg<code>.tif`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub output: Option<String>,
    /// The layer the raster goes on; none: a new layer named after the file, at the top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// Attributes the raster carries (a STAC preparation's record, a source's name).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<BTreeMap<String, String>>"))]
    pub attributes: BTreeMap<String, String>,
    /// An existing `output` is written over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub overwrite: Option<bool>,
    /// The open drawing's own file, an absolute path, when it has one: relative paths are read from its folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.rasters.reproject` v1 (its plan's too: what it would write, nothing written; no `uid`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersReprojected {
    /// The raster added (none in a plan).
    pub uid: Option<String>,
    /// The layer it went on (a new one's id).
    pub layer: String,
    /// The GeoTIFF, as the input named it.
    pub file: String,
    /// The source's system.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub from: u32,
    /// The project's.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub to: u32,
    /// `[x₀, s, 0, y₀, 0, −s]`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub affine: [f64; 6],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub height: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub bands: u32,
    pub sample: RasterSample,
    /// The transformation's path and accuracy, as the transforms say it (“EPSG:5261, ±1 m”, “aynı datum”).
    pub way: String,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
