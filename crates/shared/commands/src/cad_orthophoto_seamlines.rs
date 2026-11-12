//! `cad.orthophoto.seamlines` v1 (plan/map-cad.md MAP-12, docs/adr/4700): Mozaik birleşme çizgisi. Two orthophotos'
//! seamline (birleşme çizgisi) and mosaic elements (mozaik elemanı, TUCBS Ortogörüntü) built in KentOS: each image gets the part of its footprint the mosaic uses (ArcGIS Pro's mosaic
//! dataset seamlines: one polygon per footprint, no overlap left), the two meeting along a seam drawn through their
//! overlap from one end to the other.
//!
//! - **Voronoi** (Pan, Wang, Li, Li 2009, the area Voronoi diagram with overlap): a point of the overlap goes to the
//!   image whose own part (its footprint less the other's) is nearer; the seam is the set of points equally far from
//!   the two, so the stretch of each image's end (the other's footprint's boundary inside it) the distances are taken
//!   to. Where both nearest elements are edges the seam is straight (an angle's bisector), between a corner and an edge
//!   it is curved and its points are taken every `step` metres.
//! - **Radiometry** (the same paper's refinement, Chon, Kim and Lin 2010): through the overlap's cells the path between
//!   the Voronoi seam's ends whose largest difference of the two images is the smallest (the bottleneck), and among
//!   the paths under it the one of least cost; a cell's difference is the largest of its pixels' (each the largest of
//!   the bands' absolute differences, the second read at the first's pixel centres).
//!
//! The footprints are the rasters' extents or areas the user gives (convex, straight edges); their overlap must be one
//! piece whose boundary changes hands twice. Execute writes the two polygons on a layer in one undo step; plan writes
//! nothing. The shared cases are `fixtures/commands/v1/cad.orthophoto.seamlines.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ORTHOPHOTO_SEAMLINES: &str = "cad.orthophoto.seamlines";
pub const CAD_ORTHOPHOTO_SEAMLINES_VERSION: u32 = 1;

/// The layer the polygons go on when none is given: made in the same undo step when the drawing has none.
pub const SEAMLINE_LAYER: &str = "mozaik-elemanlari";
/// Its name.
pub const SEAMLINE_LAYER_NAME: &str = "Mozaik elemanları";

/// How the seam is drawn through the overlap (docs/adr/4700).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OrthoSeamMethod {
    /// Equally far from the two images' own parts (the area Voronoi diagram with overlap).
    #[default]
    Voronoi,
    /// The least-difference path between the Voronoi seam's ends (the bottleneck, then the least cost).
    Radiometry,
}

/// Input of `cad.orthophoto.seamlines` v1 (plan/map-cad.md MAP-12, docs/adr/4700): two orthophotos (rasters'
/// persistent ids; the first's polygon is written first), their footprints (areas or closed polylines of straight
/// edges, convex; none: each raster's extent), the method, and for Voronoi the spacing of a curved stretch's points
/// (metres; none: 1), for radiometry the cell's side in the first image's pixels (none: 1) and the corridor round the
/// Voronoi seam the path keeps to (metres; none: the whole overlap). `folder`: where a raster's relative file is looked
/// for (radiometry reads the pixels). `avoid` (radiometry): a layer whose areas the seam keeps out of. `layer`: the
/// layer's id the polygons are written on; none: `mozaik-elemanlari` (“Mozaik elemanları”), made in the same undo step
/// when missing.
///
/// Refusals: `invalid_uid`, `entity_not_found`, `not_a_raster`, `crs_mismatch` (`first`, `second`); `invalid_input`
/// (one raster twice; `step` not above zero; `cell` outside 1..=64; `corridor` not above zero; `cell`, `corridor` or
/// `avoid` with Voronoi; a footprint twice, not in the drawing, not an area or a closed polyline, of arcs, holes or parts, not
/// convex; no overlap, one footprint inside the other, an overlap whose boundary changes hands more than twice; for
/// radiometry no cell to search, too many pixels, no path between the ends); `layer_not_found` (`layer`, `avoid`),
/// `layer_locked`;
/// `invalid_revision`, `revision_conflict`; the files' (`invalid_file_path`, `file_not_found`, `import_unreadable`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoSeamlines {
    pub first: String,
    pub second: String,
    /// The two images' footprints' persistent ids, in the images' order; none: each raster's extent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub footprints: Option<[String; 2]>,
    #[serde(default)]
    pub method: OrthoSeamMethod,
    /// Voronoi: a curved stretch's points every `step` metres along the line from the seam's start to its end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub step: Option<f64>,
    /// Radiometry: a cell's side in the first image's pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub cell: Option<u32>,
    /// Radiometry: how far from the Voronoi seam a searched cell's centre may lie.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub corridor: Option<f64>,
    /// Radiometry: a layer whose areas (and closed polylines) the seam keeps out of, as buildings and bridges (USGS:
    /// a seam does not cut a roof): a cell with a pixel centre inside one is not searched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub avoid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub folder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// A seam's end (docs/adr/4700): where the two footprints' boundaries cross (`crossing`), or on a stretch the two
/// share (`shared`: the union's outer boundary), the point of it equally far from the two images' own parts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoSeamEnd {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub at: [f64; 2],
    pub kind: String,
}

/// An image's seamline polygon: the raster's persistent id, its ring (east, north; counter-clockwise, closed back to
/// its start by its last edge), its area, the part of the overlap it keeps (m²), and the object written (execute).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoSeamPolygon {
    pub image: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub ring: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub kept: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
}

/// The radiometric seam's differences (docs/adr/4700), in the images' values: the largest along the path (the
/// bottleneck no path between the ends goes under), the mean along it and over every searched cell; the path's cells,
/// the searched cells and a cell's side on the ground (metres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoSeamDifference {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bottleneck: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub mean: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub searched_mean: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub cells: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub searched: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub size: f64,
}

/// Output of `cad.orthophoto.seamlines` v1: the method, the overlap (its ring and area), the seam from its start to its
/// end (east, north) and its length, its two ends, the two polygons (the first image's, then the second's), for
/// radiometry the differences, the layer written on (and whether it was made), notes and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoSeamlinesBuilt {
    pub method: OrthoSeamMethod,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub overlap: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub overlap_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub seam: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    pub ends: [OrthoSeamEnd; 2],
    pub polygons: [OrthoSeamPolygon; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub difference: Option<OrthoSeamDifference>,
    pub layer: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub opened: bool,
    pub notes: Vec<String>,
    pub revision: String,
}
