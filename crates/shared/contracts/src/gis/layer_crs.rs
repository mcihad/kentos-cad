//! A layer's source system (docs/adr/0583 §1): the system its objects came from. The objects themselves are always
//! in the project's system; the record says where they were, its WKT2, the order EPSG gives its axes, its unit and
//! the registry's words for where it is meant for. An unknown system has no record: nothing is guessed.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The system a layer's objects came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerSourceCrs {
    /// Its EPSG code, when the registry knows it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    pub name: String,
    /// WKT2 (ISO 19162:2019).
    pub wkt: String,
    /// `en`, `ne` or `latlon`: EPSG's order of its axes (the drawing's are always east, north).
    pub axis_order: String,
    /// `metre` or `degree`.
    pub unit: String,
    /// Where the system is meant for, in the registry's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area: Option<String>,
}

/// Longest WKT kept (a definition with its operations stays far under it).
pub const MAX_WKT: usize = 16_384;

/// What is wrong with a record, in Turkish; none when it can be kept.
pub fn layer_crs_problem(c: &LayerSourceCrs) -> Option<String> {
    if c.name.trim().is_empty() {
        return Some("kaynak sisteminin adı boş".into());
    }
    if c.wkt.trim().is_empty() || c.wkt.len() > MAX_WKT {
        return Some(format!(
            "kaynak sisteminin WKT'si boş ya da {MAX_WKT} karakterden uzun"
        ));
    }
    if !matches!(c.axis_order.as_str(), "en" | "ne" | "latlon") {
        return Some(format!(
            "eksen sırası “{}” olamaz: en, ne ya da latlon",
            c.axis_order
        ));
    }
    if !matches!(c.unit.as_str(), "metre" | "degree") {
        return Some(format!("birim “{}” olamaz: metre ya da degree", c.unit));
    }
    if c.srid == Some(0) {
        return Some("yerel sistemin EPSG kodu yoktur; srid yazılmaz".into());
    }
    None
}
