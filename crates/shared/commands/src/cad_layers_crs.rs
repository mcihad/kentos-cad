//! `cad.layers.crs` and `cad.layers.reproject` v1 (docs/adr/0583 §2): a layer's source system recorded (Sistem ata:
//! nothing moves) and a layer's objects moved from a system into the project's (Yeniden izdüşür).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::cad::REVISION_TEXT;
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

/// Output of `cad.layers.reproject` v1.
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
}
