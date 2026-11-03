//! KentOS Yol's culverts (plan/road.md Y11.2, docs/adr/0909 §2; ROAD-DRAIN-01, 02, 11): `road.culverts.edit` writes,
//! rebinds or removes a culvert (a pipe or box barrel across a corridor at a station, its skew, flow line, fall and
//! sizes, the project's own); `road.culverts.query` reads every one, or one, now: its barrel between the fill's toes,
//! its ends' places and flow lines, the least cover under the road and its quantities; `road.culverts.draw` draws the
//! barrel in plan. The geometry is `kentos-road`'s; a culvert is a project setting kept in the undo history, its reading
//! is computed on every read. The desktop's handlers are `crates/native/application/src/road_culverts*.rs`; they and the
//! Python SDK pass the shared cases in `fixtures/commands/v1/road.culverts.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{CulvertFlow, CulvertShapeDef, RoadEditOperation};

pub const ROAD_CULVERTS_EDIT: &str = "road.culverts.edit";
pub const ROAD_CULVERTS_EDIT_VERSION: u32 = 1;
pub const ROAD_CULVERTS_QUERY: &str = "road.culverts.query";
pub const ROAD_CULVERTS_QUERY_VERSION: u32 = 1;
pub const ROAD_CULVERTS_DRAW: &str = "road.culverts.draw";
pub const ROAD_CULVERTS_DRAW_VERSION: u32 = 1;

/// Input of `road.culverts.edit` v1: the culvert by its id or name (absent: a new one). A new one needs `name`,
/// `corridor`, `station`, `shape`, `invert`, `slope` and `flow`. Rebind takes the corridor's present revision. `cover`,
/// `material` and `roughness` null take them off.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_culvert` (rebind or remove one not in the project),
/// `unknown_corridor`, `invalid_culvert` (a new one without its fields, set with nothing to set, the definition by
/// `road_problem`: a skew not within ±90°, sizes, counts, fall, length beyond the toes or cover out of bounds; a
/// station off the corridor or without a section, a side in cut or open, a corridor that does not build),
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub culvert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corridor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub skew: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<CulvertShapeDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub invert: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub slope: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub flow: Option<CulvertFlow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub extend: Option<f64>,
    /// The least cover asked; null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cover: Option<Option<f64>>,
    /// The barrel's material; null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub material: Option<Option<String>>,
    /// Its Manning roughness; null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub roughness: Option<Option<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// An end of a culvert's barrel (`left` or `right` of the road): its distance along the barrel from the axis (signed,
/// right positive), its offset in the cross, its plan place and its flow line's elevation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertEnd {
    pub side: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub invert: f64,
}

/// A culvert as read now: its names, whether its corridor changed since (`stale`), its station and km, its barrel's
/// length and outer width, its ends (left then right) and the side the water enters (`inlet`), the least cover over it
/// and that cover's offset in the cross, and its quantities: the pipes' total length and the barrel's concrete.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertReport {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub stale: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub km_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub width: f64,
    pub ends: Vec<CulvertEnd>,
    pub inlet: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cover: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cover_offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub pipes: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub concrete: f64,
}

/// A culvert in the project's list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertListed {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// Output of `road.culverts.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertsWritten {
    /// The culvert written, read; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub culvert: Option<CulvertReport>,
    pub culverts: Vec<CulvertListed>,
    pub revision: String,
}

/// Input of `road.culverts.query` v1: every culvert read, or one (`culvert`: its id or name).
///
/// Refusals: `not_a_road_project`, `unknown_culvert`, `invalid_culvert` (the one asked does not read). Warnings:
/// `culvert_stale`, `culvert_cover`, and the corridor's own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub culvert: Option<String>,
}

/// Output of `road.culverts.query` v1: the culverts read (one that does not read with no ends: `length` 0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertsQueried {
    pub culverts: Vec<CulvertReport>,
}

/// Input of `road.culverts.draw` v1: the culvert's barrel drawn on `layerId`: its two outer edges, its line and its
/// ends; with `labels` a text along its line saying its km, cross and length.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_culvert`,
/// `invalid_culvert`, `layer_not_found`, `not_a_layer`, `layer_locked`. Warnings: `layer_hidden`, and the read's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertsDraw {
    pub culvert: String,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub labels: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.culverts.draw` v1, and its plan: the culvert, its barrel's length and the objects written (in the
/// plan, none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertsDrawn {
    pub culvert: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    pub uids: Vec<String>,
    pub revision: String,
}
