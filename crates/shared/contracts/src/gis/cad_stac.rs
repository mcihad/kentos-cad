//! STAC discovery and data preparation (docs/adr/0528; plan/gis.md `CBS-31`).
//!
//! `cad.stac.search` asks a STAC API for the items that cover an area in a time (WGS 84 box, datetime interval,
//! collections, most cloud cover) and answers what it read: items with their assets (roles, bands, grid, scale and
//! offset). It writes nothing; the host fetches (the desktop's cloud client), the rules are the services core's
//! (`kentos_services::stac`). `cad.stac.prepare` adds the assets of an item the caller chose to the drawing as
//! rasters linked by address (a COG read by ranges, docs/adr/0207 §1), in one new layer group, as one undo step, and
//! returns the preparation record: what was searched, which item and assets, the band mapping, scale and offset,
//! the quality (QA) band and the mask rule chosen, the acquisition time. The record also lies in each raster's
//! attributes (`stac.*`), so it travels with the drawing.
//!
//! A signed address is a temporary proof, never a place: the query string of an address that carries a signature is
//! dropped before it is shown or written (`signed` says it was there); the lasting id of an asset is
//! `catalog` + `collection` + `item` + `asset` and its address without a signature.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::RasterSample;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_STAC_SEARCH: &str = "cad.stac.search";
pub const CAD_STAC_SEARCH_VERSION: u32 = 1;
pub const CAD_STAC_PREPARE: &str = "cad.stac.prepare";
pub const CAD_STAC_PREPARE_VERSION: u32 = 1;

/// The most items a search page takes.
pub const STAC_MOST_LIMIT: u32 = 100;
/// The page size a search without `limit` asks for.
pub const STAC_DEFAULT_LIMIT: u32 = 10;
/// The most assets one preparation adds.
pub const STAC_MOST_ASSETS: usize = 32;
/// The record's format name.
pub const STAC_RECORD_FORMAT: &str = "kentos.stac-preparation";
pub const STAC_RECORD_VERSION: u32 = 1;

/// An HTTP method of a STAC request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StacMethod {
    Get,
    Post,
}

/// A `next` link of a search page (STAC API item search §paging): the next request as the server wrote it. It is
/// taken only when its address is on the endpoint's own host (`stac_next_foreign`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacNext {
    pub method: StacMethod,
    pub href: String,
    /// A POST's body; with `merge` it is laid over the search's own body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(type = "unknown", optional))]
    pub body: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub merge: bool,
}

/// Input of `cad.stac.search` v1. `endpoint` is the STAC API's root (`https://…/v1`), without a query or a login.
/// `bbox` is `[west, south, east, north]` in degrees (WGS 84; a box crossing the 180° meridian is two searches).
/// `datetime` is an RFC 3339 instant or interval (`start/end`, an open end `..`); a bare date `2024-07-01` is its
/// day (`T00:00:00Z`, and `T23:59:59Z` as an interval's end). `maxCloudCover` (percent) is asked of the server
/// (`eo:cloud_cover`) and checked again on what comes back. With `next` the page after one already read; the other
/// fields stay the search's (the filter on cloud cover still applies). Refusals, in this order:
/// `invalid_stac_endpoint`, `invalid_stac_bbox`, `invalid_stac_datetime`, `invalid_stac_cloud_cover`,
/// `invalid_stac_limit`, `invalid_stac_collection`, `stac_next_foreign`, `stac_unavailable` (the host cannot reach
/// a network), `stac_request_failed`, `stac_response_unreadable`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacSearch {
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub bbox: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub datetime: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub collections: Vec<String>,
    /// Items by id (the search narrowed to them).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub ids: Vec<String>,
    /// Percent, 0 to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub max_cloud_cover: Option<f64>,
    /// Items per page, 1 to 100; absent: 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<StacNext>,
}

/// The request a search sent, as the host sent it (a signed address never appears).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacRequestInfo {
    pub method: StacMethod,
    pub url: String,
    /// A POST's body as compact JSON, keys in alphabetical order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub body: Option<String>,
}

/// A band an asset names (STAC `eo:bands`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacBand {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// `red`, `nir`, `nir08`, `swir16` … (the eo extension's common names and the catalog's own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub common_name: Option<String>,
    /// Micrometres, as the catalog writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub center_wavelength: Option<f64>,
}

/// A band's storage (STAC `raster:bands`): how its numbers are read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacRasterBand {
    /// `uint8`, `int16`, `float32` … as the catalog writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub data_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub nodata: Option<f64>,
    /// A value is `number × scale + offset` (absent: 1 and 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub offset: Option<f64>,
    /// Metres of a pixel, the catalog's word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spatial_resolution: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
}

/// An asset of an item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacAssetInfo {
    /// The asset's key in the item (`red`, `scl`, `qa_pixel`): with the item it is the asset's lasting id.
    pub key: String,
    /// The address without any signature (a query that carries one is dropped). `s3://` and other schemes stay as the
    /// catalog wrote them; only http and https addresses can be read by a raster.
    pub href: String,
    /// The catalog's address carried a signature (or a token): it was dropped, and reading the file may need one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub signed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub title: Option<String>,
    /// The media type, as written.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(rename = "type", optional))]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub roles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<StacBand>>", optional))]
    pub bands: Vec<StacBand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<StacRasterBand>>", optional))]
    pub raster_bands: Vec<StacRasterBand>,
    /// The asset's own or its item's EPSG code (`proj:epsg`, `proj:code`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub epsg: Option<u32>,
    /// `[rows, columns]` (`proj:shape`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub shape: Option<[u32; 2]>,
    /// `[a, b, c, d, e, f]` (`proj:transform`): x = a·column + b·row + c, y = d·column + e·row + f.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub transform: Option<[f64; 6]>,
    /// The bucket's region (the STAC storage extension: `storage:region` of the asset or the item, or of the asset's
    /// first `storage:refs` scheme), for an `s3://` address (docs/adr/0644 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub region: Option<String>,
    /// Whoever reads the bucket pays for it (`storage:requester_pays`): reading takes the user's AWS credentials.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub requester_pays: bool,
}

/// An item of a search, with what a preparation needs of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacItemInfo {
    /// The endpoint (the API's root, normalized): with `collection` and `id` the item's lasting id.
    pub catalog: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub collection: Option<String>,
    pub id: String,
    /// The acquisition time (`datetime`), or the start of its interval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub datetime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end_datetime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub bbox: Option<[f64; 4]>,
    /// Percent (`eo:cloud_cover`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub cloud_cover: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub instruments: Vec<String>,
    /// Metres of a pixel (`gsd`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub gsd: Option<f64>,
    /// The item's EPSG code (`proj:epsg`, `proj:code`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub epsg: Option<u32>,
    /// The license identifier (`license` of the item).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub license: Option<String>,
    /// The address of the license text (a `license` link), without any signature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub license_href: Option<String>,
    /// The item's own address (the `self` link).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub self_href: Option<String>,
    pub assets: Vec<StacAssetInfo>,
}

/// Output of `cad.stac.search` v1, and its plan. `items` are the page's, in the server's order, without those over
/// `maxCloudCover` (`dropped` counts them, an item without a cloud cover stays); `returned` is what the server
/// sent; `matched` is its total (`numberMatched`) when it said; `next` is the page after.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacSearched {
    pub request: StacRequestInfo,
    pub items: Vec<StacItemInfo>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub returned: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub dropped: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub matched: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<StacNext>,
}

/// What a search asked, kept in the preparation record.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacSearchRecord {
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub bbox: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub datetime: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub collections: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub max_cloud_cover: Option<f64>,
}

/// Which QA values or bits mean “not usable” (clouds, shadows, no data).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StacMask {
    /// A QA pixel with one of these values is masked (Sentinel-2's SCL classes).
    Values {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
        values: Vec<u32>,
    },
    /// A QA pixel with any of these bits (0 is the lowest) set is masked (Landsat's QA_PIXEL).
    Bits {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        bits: Vec<u32>,
    },
}

/// The quality band of a preparation: which asset, and the mask rule. Without `mask` the platform's own is taken
/// (Sentinel-2 `scl`: classes 0, 1, 3, 8, 9, 10; Landsat `qa_pixel`: bits 0 to 4: fill, dilated cloud, cirrus,
/// cloud, cloud shadow); an asset of another platform needs one (`stac_qa_unknown`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacQa {
    pub asset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mask: Option<StacMask>,
}

/// Input of `cad.stac.prepare` v1: the `item` as a search returned it (only the assets used need to be in it),
/// the data assets to add (`assets`, each a single band read by address), the optional quality band, and what was
/// searched. One new group `group` (absent: `STAC: <item id>`) takes one layer per asset, named by its key; the
/// QA band is added too (marked `qa`), and is never among the data bands. The rasters' system must be the
/// project's (no reprojection). Refusals, in this order: `invalid_stac_item`, `stac_assets_empty`,
/// `stac_too_many_assets`, `stac_asset_not_found`, `stac_href_not_http`, `stac_no_grid`, `stac_unsupported_sample`,
/// `stac_multiband`, `stac_qa_unknown`, `crs_mismatch`, `invalid_group_name`, then `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacPrepare {
    pub item: StacItemInfo,
    pub assets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub qa: Option<StacQa>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub search: Option<StacSearchRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// Local copies of assets (the asset's key → its file, absolute or from the drawing's folder): the raster reads the
    /// file, not the address (docs/adr/0644 §3). The desktop downloads them itself.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(optional, as = "Option<std::collections::BTreeMap<String, String>>")
    )]
    pub local: std::collections::BTreeMap<String, String>,
    /// An asset in another coordinate system than the project's is carried into it: its local copy (`local`) is
    /// resampled onto the project's grid beside it (`cad.rasters.reproject`'s rule) and the raster reads that.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reproject: bool,
    /// The open drawing's own file, when it has one: relative `local` paths are read from its folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// An asset carried into the project's system (docs/adr/0644 §3): from and to, the transformation's way, the file
/// written and its grid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacReprojected {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub from: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub to: u32,
    /// “EPSG:5261, ±1 m”, “aynı datum”.
    pub way: String,
    pub file: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub height: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub affine: [f64; 6],
}

/// A data band of a preparation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacPreparedBand {
    /// The asset's key, with the item's id and collection its lasting id.
    pub asset: String,
    /// The band's common name when the catalog gave one (`red`, `nir`, `nir08`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub common_name: Option<String>,
    /// The address without any signature.
    pub href: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub signed: bool,
    pub sample: RasterSample,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub nodata: Option<f64>,
    /// A value is `number × scale + offset`: the catalog's, or 1 and 0 when it gave none.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub height: u32,
    /// `[x₀, a, b, y₀, c, d]` as a raster's (GDAL's geotransform order).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub affine: [f64; 6],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub epsg: u32,
    /// The local copy the raster reads (`local`), instead of the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// The asset carried into the project's system (`reproject`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reprojected: Option<StacReprojected>,
    /// The layer that holds the raster, and the raster's persistent id once written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
}

/// The quality band of a preparation and its mask rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacPreparedQa {
    pub asset: String,
    pub href: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub signed: bool,
    pub sample: RasterSample,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub nodata: Option<f64>,
    pub mask: StacMask,
    /// Whether the rule is the platform's own default.
    pub default_mask: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub height: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub affine: [f64; 6],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reprojected: Option<StacReprojected>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
}

/// Reflectance's scale and offset when every data band shares them: the numbers İşlemler's Spektral indis takes
/// as Ölçek and Öteleme (`ρ = number × scale + offset`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacReflectance {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub offset: f64,
}

/// The preparation record (`kentos.stac-preparation` 1): what was searched, the item and assets chosen, the band
/// mapping, scale and offset, the quality band and the mask rule, the acquisition time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacPreparation {
    pub format: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub search: Option<StacSearchRecord>,
    pub catalog: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub collection: Option<String>,
    pub item: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub datetime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub cloud_cover: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub license_href: Option<String>,
    pub bands: Vec<StacPreparedBand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub qa: Option<StacPreparedQa>,
    /// Present when the data bands are reflectance and share one scale and offset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reflectance: Option<StacReflectance>,
}

/// Output of `cad.stac.prepare` v1, and its plan (the layers and uids are filled when it is written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StacPrepared {
    pub preparation: StacPreparation,
    /// The group's id once written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    pub revision: String,
}

/// The attribute keys a preparation writes to each raster it adds.
pub mod stac_attrs {
    pub const CATALOG: &str = "stac.catalog";
    pub const COLLECTION: &str = "stac.collection";
    pub const ITEM: &str = "stac.item";
    pub const ASSET: &str = "stac.asset";
    pub const HREF: &str = "stac.href";
    pub const ROLE: &str = "stac.role";
    pub const BAND: &str = "stac.band";
    pub const DATETIME: &str = "stac.datetime";
    pub const CLOUD_COVER: &str = "stac.cloud_cover";
    pub const PLATFORM: &str = "stac.platform";
    pub const SCALE: &str = "stac.scale";
    pub const OFFSET: &str = "stac.offset";
    pub const NODATA: &str = "stac.nodata";
    pub const LICENSE: &str = "stac.license";
    pub const SEARCH: &str = "stac.search";
    pub const QA_MASK: &str = "stac.qa_mask";
    /// How the asset is reached (docs/adr/0644 §1): `aws` (an `s3://` address, signed when the user has AWS
    /// credentials), `planetary-computer` (a blob address the Planetary Computer signs with a SAS token).
    pub const ACCESS: &str = "stac.access";
    /// `true`: whoever reads the bucket pays.
    pub const REQUESTER_PAYS: &str = "stac.requester_pays";
    /// The local copy the raster reads.
    pub const LOCAL: &str = "stac.local";
    /// `EPSG:32636`: the asset's own system, carried into the project's.
    pub const REPROJECTED_FROM: &str = "stac.reprojected_from";
}
