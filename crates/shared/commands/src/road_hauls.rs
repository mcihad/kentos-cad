//! KentOS Yol's mass haul commands (plan/road.md Y9, docs/adr/0907): `road.hauls.edit` writes or removes a mass haul
//! (Brückner) diagram (an earthwork volume's mass ordinate against a balance line, with a free haul distance and
//! borrow and waste sites); `road.hauls.query` reads every one, or one, now: its balance points, its loops with their
//! balanced volume, haul moment, average haul and overhaul, and the volume left to waste or borrow. The geometry is
//! `kentos-road`'s; a diagram is a project setting kept in the undo history, its reading is computed on every read.
//! The desktop's handlers are `crates/native/application/src/road_hauls*.rs`; they and the Python SDK pass the shared
//! cases in `fixtures/commands/v1/road.hauls.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{HaulSiteDef, RoadEditOperation};

pub const ROAD_HAULS_EDIT: &str = "road.hauls.edit";
pub const ROAD_HAULS_EDIT_VERSION: u32 = 1;
pub const ROAD_HAULS_QUERY: &str = "road.hauls.query";
pub const ROAD_HAULS_QUERY_VERSION: u32 = 1;

/// Input of `road.hauls.edit` v1: the diagram by its id or name (absent: a new one). A new one needs `name` and
/// `volume`. A diagram follows its volume: it is not rebound to another.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_haul` (rebind), `unknown_haul` (remove one not in the
/// project), `unknown_volume`, `invalid_haul` (another volume, a new one without its fields, set with nothing to set,
/// the definition by `road_problem`: a free haul not in (0, 100 km], a site without a name or with a volume not above
/// 0; a volume that does not compute or a site off its stations), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub haul: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub balance: Option<f64>,
    /// The free haul distance; null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub free: Option<Option<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sites: Option<Vec<HaulSiteDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A loop between two balance points, or an end stretch that does not close: its stations and km, its direction, the
/// volume balanced in it, the haul moment, the average haul (none where nothing is balanced), the overhaul's volume and
/// moment beyond the free haul distance, and the volume with no destination (waste forward, borrow backward).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulLoop {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    pub forward: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub moment: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub average: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub over_volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub over_moment: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub unbalanced: f64,
}

/// A mass haul diagram as it reads now: its sources, whether its volume's corridor is stale, the ordinate's points
/// (station, m³; with `points`), the balance points, the loops, the totals; why it does not read when it does not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulReport {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub volume: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub balance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub free: Option<f64>,
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub points: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub balance_points: Vec<f64>,
    pub loops: Vec<HaulLoop>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub hauled: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub moment: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub average: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub over_volume: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume-m")))]
    pub over_moment: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub waste: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub borrow: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
}

/// A mass haul diagram in a list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulListed {
    pub id: String,
    pub name: String,
    pub volume: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// Output of `road.hauls.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsWritten {
    /// The diagram written, read; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub haul: Option<HaulReport>,
    pub hauls: Vec<HaulListed>,
    pub revision: String,
}

/// Input of `road.hauls.query` v1: every diagram read, or one (`haul`: its id or name) with its ordinate's points.
///
/// Refusals: `not_a_road_project`, `unknown_haul`, `invalid_haul` (`points` without a diagram; the one asked does not
/// read). Warnings: `corridor_stale`, `volume_gap`, `haul_unbalanced`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub haul: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub points: bool,
}

/// Output of `road.hauls.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HaulsQueried {
    pub hauls: Vec<HaulReport>,
}
