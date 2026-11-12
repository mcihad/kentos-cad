//! `cad.layers.crs` and `cad.layers.reproject` v1 (docs/adr/0583 §2): a layer's source system recorded (Koordinat
//! sistemi ▸ Tanımla: nothing moves) and a layer's objects moved from a system into the project's (▸ Dönüştür).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::cad_crs::ChainStatus;
use crate::entity::Vec2;
use crate::layer_crs::LayerSourceCrs;

pub const CAD_LAYERS_CRS: &str = "cad.layers.crs";
pub const CAD_LAYERS_CRS_VERSION: u32 = 1;
pub const CAD_LAYERS_REPROJECT: &str = "cad.layers.reproject";
pub const CAD_LAYERS_REPROJECT_VERSION: u32 = 1;

/// Input of `cad.layers.crs` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersCrs {
    /// The layer's id.
    pub layer: String,
    /// The system its objects came from (an EPSG code the registry knows); absent or null clears the record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.layers.crs` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersCrsSet {
    pub layer: String,
    /// The record now; absent when cleared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crs: Option<LayerSourceCrs>,
    /// Whether it differed (an undo step was written).
    pub changed: bool,
    /// The document's revision after the write, as decimal text.
    pub revision: String,
}

/// Input of `cad.layers.reproject` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersReproject {
    /// The layer's id.
    pub layer: String,
    /// The system the objects are in now (an EPSG code the registry knows); absent: the layer's recorded one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub from: Option<u32>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// At most this many objects' shifts are listed in an answer (all are counted): those with a note first, then the
/// rest, each in drawing order (docs/adr/2500 §1).
pub const REPROJECT_LISTED: usize = 1000;

/// What a moved object's points may fall outside of (docs/adr/2500 §3), in this order: EPSG's area of use of the
/// system the objects come from (read at their place there), of the datum transformation's EPSG operation (the same
/// place), of the project's system (where they land).
pub const REPROJECT_NOTES: [&str; 3] = [
    "outside_source_area",
    "outside_operation_area",
    "outside_project_area",
];

/// An object's shift (docs/adr/2500 §1): its persistent id and kind, how many points it has, its first point as it is
/// and as it moves, that point's shift (moved − now) and its length, the smallest and largest shift of all its points,
/// in the project's units, and what its points fall outside of (codes of [`REPROJECT_NOTES`], in that order).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReprojectShift {
    pub uid: String,
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub before: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub after: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub dx: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub dy: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub shift: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub least: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub most: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub notes: Vec<String>,
}

/// Output of `cad.layers.reproject` v1: what moved and how (docs/adr/0583 §2), and the preview of the move
/// (docs/adr/2500): every point's shift summed up (the mean shift, the smallest and largest, in the project's units;
/// none when nothing moves), the objects with a note, the chain's verdict for the way as `cad.crs.transform` gives it
/// (docs/adr/1230 §3; none when nothing moves) and the objects' shifts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersReprojected {
    pub layer: String,
    /// The system the objects came from.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub from: u32,
    /// The project's system they are in now.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub to: u32,
    /// The objects moved.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moved: u32,
    /// The way and its accuracy (“EPSG:1311, ±1 m”), as the transforms say it.
    pub via: String,
    /// The layer's record now.
    pub crs: LayerSourceCrs,
    /// The document's revision after the write, as decimal text.
    pub revision: String,
    /// Every point of the objects moved.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub mean_dx: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub mean_dy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub least: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub most: Option<f64>,
    /// The objects with a note.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub flagged: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub status: Option<ChainStatus>,
    /// Why the chain is limited, in `cad.crs.transform`'s order.
    pub reasons: Vec<String>,
    /// At most [`REPROJECT_LISTED`] objects' shifts.
    pub shifts: Vec<ReprojectShift>,
}
