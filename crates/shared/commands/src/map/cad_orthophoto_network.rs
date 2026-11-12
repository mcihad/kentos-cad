//! `cad.orthophoto.network` v1 and `cad.orthophoto.footprint` v1 (plan/map-cad.md MAP-12, docs/adr/5260).
//!
//! - **Birleşme çizgisi ağı:** the mosaic elements of two or more orthophotos. Every place of the footprints' union
//!   goes to the image, among those whose footprint covers it, whose footprint's centre is nearest (the first given of
//!   equals): the area Voronoi diagram of the centres cut to the footprints. Where three or more elements meet is a
//!   junction (üçlü düğüm). Each element is written as an area on the mosaic elements' layer.
//! - **Ayak izi:** a raster's footprint from its pixels: the outline, on the pixels' edges, of those that are not
//!   NoData (a pixel is valid when any band is not NoData), outer rings and holes, or (`hull`) their convex hull, which
//!   the seamlines take as a footprint. Written as an area on the footprints' layer.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ORTHOPHOTO_NETWORK: &str = "cad.orthophoto.network";
pub const CAD_ORTHOPHOTO_NETWORK_VERSION: u32 = 1;
pub const CAD_ORTHOPHOTO_FOOTPRINT: &str = "cad.orthophoto.footprint";
pub const CAD_ORTHOPHOTO_FOOTPRINT_VERSION: u32 = 1;
/// The most images a network takes.
pub const MOST_NETWORK_IMAGES: usize = 50;
/// The footprints' layer made when none is given and the drawing has none, and its name.
pub const FOOTPRINT_LAYER: &str = "ayak-izleri";
pub const FOOTPRINT_LAYER_NAME: &str = "Ayak izleri";
/// The most pixels a footprint reads.
pub const MOST_FOOTPRINT_PIXELS: u64 = 25_000_000;

/// Input of `cad.orthophoto.network` v1: the orthophotos (2 to 50, rasters of the project's system), their footprints
/// in the same order (areas or closed polylines with straight edges, convex; none: the rasters' four corners), the
/// layer the elements are written on (none: `mozaik-elemanlari`, made when the drawing has none). Refusals:
/// `invalid_uid`, `entity_not_found`, `not_a_raster`, `crs_mismatch` (`images[k]`); `invalid_input` (fewer than two or
/// more than 50 images, one twice, footprints not one each, a footprint not convex or covering nothing, an image whose
/// footprint is wholly another's nearer one: no element); `layer_not_found`, `layer_locked`; `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoNetwork {
    pub images: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub footprints: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// An image's mosaic element: its parts (each an outer ring, counter-clockwise, from its southernmost corner, the
/// westernmost of equals; and its holes, clockwise), its area, its footprint's centre, the ids written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoNetworkElement {
    pub image: String,
    pub parts: Vec<OrthoNetworkPart>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub centre: [f64; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
}

/// A part of an element: its outer ring and holes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoNetworkPart {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub ring: Vec<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Vec<[f64; 2]>>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// A junction: a place three or more elements meet, and their images (in the input's order).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoNetworkJunction {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub at: [f64; 2],
    pub images: Vec<String>,
}

/// Output of `cad.orthophoto.network` v1: the elements in the input's order, the junctions (south to north, west to
/// east), the union's area, the layer (and whether it was made), notes and the revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoNetworkBuilt {
    pub elements: Vec<OrthoNetworkElement>,
    pub junctions: Vec<OrthoNetworkJunction>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub covered: f64,
    pub layer: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub opened: bool,
    pub notes: Vec<String>,
    pub revision: String,
}

/// Input of `cad.orthophoto.footprint` v1: the raster, whether its convex hull is drawn instead of its outline, the
/// layer (none: `ayak-izleri`, made when the drawing has none) and, for a raster whose file is a relative path, the
/// drawing's folder. Refusals: `invalid_uid`, `entity_not_found`, `not_a_raster`, `crs_mismatch` (`raster`);
/// `file_not_found`, `invalid_file_path`, `import_unreadable` (its image); `invalid_input` (too many pixels, every
/// pixel NoData); `layer_not_found`, `layer_locked`; `invalid_revision`, `revision_conflict`. A raster without a NoData
/// value has every pixel valid: its footprint is its bounds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoFootprint {
    pub raster: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub hull: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub folder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.orthophoto.footprint` v1: the footprint's parts (as the network's elements'), its area, the pixels
/// read and valid, the NoData value, whether it is the hull, the layer, the id written, notes and the revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoFootprintBuilt {
    pub parts: Vec<OrthoNetworkPart>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pixels: u64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub valid: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub nodata: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub hull: bool,
    pub layer: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub opened: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    pub notes: Vec<String>,
    pub revision: String,
}
