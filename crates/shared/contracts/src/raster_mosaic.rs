//! `cad.rasters.mosaic` v1 (docs/adr/0517; plan/gis.md `CBS-28`): many GeoTIFF files made one virtual mosaic, a
//! GDAL VRT that KentOS, GDAL and QGIS open alike; no pixel is copied. Which file lies on top where they overlap is
//! the order given, the finer pixel or the newer date; the mosaic's pixel is the finest, coarsest or mean of the
//! files' (or one given). Optionally the mosaic is added to the drawing as a raster, as one undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::RasterSample;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_RASTERS_MOSAIC: &str = "cad.rasters.mosaic";
pub const CAD_RASTERS_MOSAIC_VERSION: u32 = 1;

/// The most files a mosaic takes.
pub const MOSAIC_MOST_SOURCES: usize = 10_000;

/// Which file lies on top where files overlap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MosaicOrder {
    /// The order of `sources`: the first on top.
    #[default]
    Given,
    /// The smaller pixel on top; equal ones in the order given.
    Finest,
    /// The newer date (a TIFF's DateTime) on top; files without one at the bottom; equal ones in the order given.
    Newest,
}

/// The mosaic's pixel from the files' (each axis on its own, as gdalbuildvrt's `-resolution`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MosaicResolution {
    /// The finest file's: no detail lost.
    #[default]
    Highest,
    /// The coarsest file's.
    Lowest,
    /// The files' mean.
    Average,
}

/// Input of `cad.rasters.mosaic` v1. Refusals, in this order: `mosaic_empty`, `mosaic_too_large` (more than
/// 10 000 files), `invalid_file_path` (a source or the mosaic's path), `invalid_pixel_size`, `file_not_found`,
/// `import_unreadable`, `mosaic_unplaced`, `mosaic_rotated`, `mosaic_mismatch`, `mosaic_too_large` (the mosaic's
/// size), `file_exists`, then with `layer`: `layer_not_found`, `not_a_layer`, `layer_locked`, `crs_mismatch`, and
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersMosaic {
    /// The GeoTIFF files, absolute or from the drawing's folder; with `order` `given` the first lies on top.
    pub sources: Vec<String>,
    /// The VRT file written (`.vrt`), absolute or from the drawing's folder.
    pub path: String,
    #[serde(default)]
    pub order: MosaicOrder,
    #[serde(default)]
    pub resolution: MosaicResolution,
    /// A square pixel of this size, in the files' system's units, instead of `resolution`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub pixel_size: Option<f64>,
    /// The mosaic's nodata (what no file covers); none: the first file's (in the order given) that has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub nodata: Option<f64>,
    /// The layer the mosaic is added to as a raster (linked to `path`); none: only the file is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// An existing `path` is written over.
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

/// A file of the mosaic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MosaicSource {
    /// As the VRT holds it: from the VRT's folder when the file is under it, else absolute.
    pub file: String,
    /// Where it was read from.
    pub read: String,
    /// 1 on top.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub rank: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub height: u32,
    /// Its pixel's width and height, in the system's units.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub pixel: [f64; 2],
    /// Its TIFF's DateTime, `YYYY-MM-DDTHH:MM:SS`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    /// The rectangle of its pixels taken, `[x, y, width, height]` (the VRT's SrcRect): all of it, or cut where the
    /// mosaic's rounded size ends.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub src: [f64; 4],
    /// Its rectangle on the mosaic, `[x, y, width, height]` in the mosaic's pixels (the VRT's DstRect).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub dst: [f64; 4],
    /// Its nodata, which lets the files below show.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub nodata: Option<f64>,
}

/// Output of `cad.rasters.mosaic` v1 (its plan's too: what it would write, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersMosaicked {
    /// The VRT file, as written.
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub height: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub bands: u32,
    pub sample: RasterSample,
    /// `[x₀, a, 0, y₀, 0, d]`: north up, GDAL's geotransform.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub affine: [f64; 6],
    /// The files' coordinate system; 0: they name none.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub nodata: Option<f64>,
    /// The files, top first.
    pub sources: Vec<MosaicSource>,
    /// The VRT's size.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    /// The raster added, when `layer` was given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
