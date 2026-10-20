//! GeoPackage tile pyramids as rasters (docs/adr/0556; plan/gis.md `CBS-03`'s raster profile).
//!
//! `cad.rasters.gpkgread` reads the tile table of a GeoPackage (its tile matrix set and matrices, PNG tiles) and
//! takes one zoom level, or a part of it, as one raster: the tiles of the level's window are laid side by side and
//! written as a north-up GeoTIFF (`output`), four bands of bytes (red, green, blue and alpha, alpha 0 where the
//! file holds no tile), in the table's system; with `layer` the GeoTIFF is added to the drawing as a raster linked to
//! it, as one undo step. Without `output` it only says what the file holds.
//!
//! `cad.rasters.gpkgwrite` writes a raster of the drawing (8-bit, north up, not a palette) as a tile table: 256 ×
//! 256 PNG tiles on a pyramid whose finest zoom is the raster's own pixel, each coarser one half as many pixels
//! (a 2 × 2 mean), in a new GeoPackage or added to one that is there.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::RasterSample;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_RASTERS_GPKGREAD: &str = "cad.rasters.gpkgread";
pub const CAD_RASTERS_GPKGREAD_VERSION: u32 = 1;
pub const CAD_RASTERS_GPKGWRITE: &str = "cad.rasters.gpkgwrite";
pub const CAD_RASTERS_GPKGWRITE_VERSION: u32 = 1;

/// The most pixels a raster read from tiles, or written as tiles, may have (it is held whole).
pub const GPKG_TILES_MOST_PIXELS: u64 = 25_000_000;
/// The tile side `cad.rasters.gpkgwrite` writes, pixels.
pub const GPKG_TILE_SIDE: u32 = 256;

/// Input of `cad.rasters.gpkgread` v1. Refusals, in this order: `invalid_file_path` (the file, or `layer` without
/// `output`), `file_not_found`, `import_unreadable` (not a GeoPackage), `no_tile_table`, `table_not_found` (`table`
/// is not a tile table; a file with several needs it), `zoom_not_found`, `invalid_area`, `tile_table_empty` (no tile
/// at the zoom in the window), `gpkg_tiles_too_large`, `tile_unreadable` (a tile that is not a PNG of the matrix's
/// size), `tile_format_unsupported` (JPEG, WebP or a PNG of more than 8 bits), then with `output`: `invalid_file_path`
/// (not a `.tif`), `file_exists`, with `layer`: `layer_not_found`, `not_a_layer`, `layer_locked`, `crs_mismatch`, and
/// `invalid_revision`, `revision_conflict`. Warnings: `crs_undeclared`, `tiles_missing`, `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersGpkgRead {
    /// The GeoPackage, absolute or from the drawing's folder.
    pub path: String,
    /// The tile table; none: the file's only tile table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
    /// The zoom level; none: the highest (the finest).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub zoom: Option<u32>,
    /// The part to read, `[min x, min y, max x, max y]` in the table's system; none: the table's data extent (its
    /// tile matrix set's, when `gpkg_contents` gives none) cut to the level's grid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub area: Option<[f64; 4]>,
    /// The GeoTIFF written (`.tif`), absolute or from the drawing's folder; none: nothing is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub output: Option<String>,
    /// The layer the raster is added to (linked to `output`); none: only the file is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
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

/// A zoom level of a tile table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GpkgTileZoom {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub zoom: u32,
    /// Tiles across and down the matrix.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub matrix_width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub matrix_height: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub tile_width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub tile_height: u32,
    /// A pixel's width and height, in the system's units.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub pixel: [f64; 2],
    /// Tiles the table holds at this zoom.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub tiles: u64,
    /// The formats of its tiles (`png`, `jpeg`, `webp`, `other`), looked at in up to 64 of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub formats: Vec<String>,
}

/// A tile table of the file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GpkgTileSet {
    pub table: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub identifier: Option<String>,
    /// The file's own id of the system (`gpkg_spatial_ref_sys`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub srs_id: i32,
    /// Its EPSG code; none for an undefined system or another authority's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// The tile matrix set's box: its top-left corner is the grid's origin.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub bounds: [f64; 4],
    /// `gpkg_contents`'s box, the data's extent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub data_bounds: Option<[f64; 4]>,
    /// The zoom levels, coarsest first.
    pub zooms: Vec<GpkgTileZoom>,
    /// The file's extensions that name this table (`gpkg_webp`, …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub extensions: Vec<String>,
}

/// Output of `cad.rasters.gpkgread` v1 (its plan's too: what it would read and write, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersGpkgReadDone {
    /// Every tile table of the file.
    pub tables: Vec<GpkgTileSet>,
    /// The table and zoom level read.
    pub table: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub zoom: u32,
    /// The window read, `[min x, min y, max x, max y]`, whole pixels of the level.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub area: [f64; 4],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub height: u32,
    /// Always 4: red, green, blue and alpha.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub bands: u32,
    pub sample: RasterSample,
    /// `[x₀, a, 0, y₀, 0, d]`: north up, GDAL's geotransform.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub affine: [f64; 6],
    /// The table's EPSG code; 0: undefined or not EPSG.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    /// Tiles taken, and tiles of the window the table does not hold (transparent).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub tiles_read: u64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub tiles_missing: u64,
    /// The GeoTIFF, as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: Option<u64>,
    /// The raster added, when `layer` was given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Where `cad.rasters.gpkgwrite` puts the table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GpkgTilesMode {
    /// A new GeoPackage; a file that is there is written over only with `overwrite`.
    #[default]
    New,
    /// The table is added to the GeoPackage that is there, in one transaction; a table of the name is refused.
    Add,
}

/// Input of `cad.rasters.gpkgwrite` v1. Refusals, in this order: `invalid_uid`, `entity_not_found`, `not_a_raster`,
/// `invalid_file_path` (not a `.gpkg`, a relative path without the drawing, no folder), `invalid_table_name`,
/// `raster_unsupported` (a palette, more than 8 bits, a dataset, an address without a file),
/// `raster_not_north_up`, `gpkg_tiles_too_large`, `file_not_found` (the raster's source), `import_unreadable`,
/// then in mode `new` `file_exists`, in mode `add` `file_not_found` and `import_unreadable` (no GeoPackage there)
/// and `table_exists`. Warnings: `nodata_lost`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersGpkgWrite {
    /// The raster, by persistent id.
    pub uid: String,
    /// The GeoPackage written (`.gpkg`), absolute or from the drawing's folder.
    pub path: String,
    /// The tile table's name; none: `tiles`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
    #[serde(default)]
    pub mode: GpkgTilesMode,
    /// The table's identifier (`gpkg_contents`); none: its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub identifier: Option<String>,
    /// An existing `path` (mode `new`) is written over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub overwrite: Option<bool>,
    /// The open drawing's own file, an absolute path, when it has one: relative paths are read from its folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// Output of `cad.rasters.gpkgwrite` v1 (its plan's too: what it would write, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersGpkgWritten {
    pub path: String,
    pub table: String,
    pub mode: GpkgTilesMode,
    /// The raster's EPSG code; 0: it names none (the file's undefined cartesian system, id −1).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    /// The tiles' bands: 1 grey, 2 grey with alpha, 4 colour (RGBA).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub bands: u32,
    /// The tile matrix set's box and the data's extent, `[min x, min y, max x, max y]`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub bounds: [f64; 4],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub data_bounds: [f64; 4],
    /// The zoom levels written, coarsest first.
    pub zooms: Vec<GpkgTileZoom>,
    /// Tiles written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub tiles: u64,
    /// The file's size after the write (a plan: the tiles' bytes).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
