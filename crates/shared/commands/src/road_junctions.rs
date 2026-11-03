//! KentOS Yol's junctions (plan/road.md Y12.2, docs/adr/0910 §2; ROAD-JUNC-01, 02, 04): `road.junctions.edit` writes,
//! rebinds or removes a junction (two alignments as its legs, their edges, which crossing, at grade or separated, the
//! curb returns' radii, the project's own); `road.junctions.query` reads every one, or one, now: the crossing, the
//! legs' stations and the angle between them, and at grade each quadrant's curb return; `road.junctions.draw` draws the
//! returns in plan. The geometry is `kentos-road`'s `junction`; a junction is a project setting kept in the undo
//! history, its reading is computed on every read. The desktop's handlers are
//! `crates/native/application/src/road_junctions*.rs`; they and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.junctions.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{JunctionKind, JunctionLegDef, RoadEditOperation};

pub const ROAD_JUNCTIONS_EDIT: &str = "road.junctions.edit";
pub const ROAD_JUNCTIONS_EDIT_VERSION: u32 = 1;
pub const ROAD_JUNCTIONS_QUERY: &str = "road.junctions.query";
pub const ROAD_JUNCTIONS_QUERY_VERSION: u32 = 1;
pub const ROAD_JUNCTIONS_DRAW: &str = "road.junctions.draw";
pub const ROAD_JUNCTIONS_DRAW_VERSION: u32 = 1;

/// Input of `road.junctions.edit` v1: the junction by its id or name (absent: a new one). A new one needs `name`,
/// `kind` and `legs` (each leg's alignment by id or name), and at grade `radius`. Rebind takes the legs' alignments'
/// present revisions. `radii` null takes it off.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_junction` (rebind or remove one not in the project),
/// `unknown_alignment`, `invalid_junction` (a new one without its fields, set with nothing to set, the definition by
/// `road_problem`: two legs on two alignments, edges and radii in bounds; legs that do not cross, a crossing not there,
/// legs parallel at it), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub junction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<JunctionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub legs: Option<Vec<JunctionLegDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub at: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: Option<f64>,
    /// Each quadrant's radius; null takes them off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<Vec<f64>>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<Vec<f64>>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radii: Option<Option<Vec<f64>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A place: plan and, on a leg, its station.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionPlace {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
}

/// A leg at the crossing: its alignment, station and km there, its direction (degrees from east, counter-clockwise)
/// and whether its alignment changed since.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionLeg {
    pub alignment: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub km_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub direction: f64,
    pub stale: bool,
}

/// A quadrant's curb return: the sides of the two legs it lies outward of, its radius and centre, its tangent points
/// on the first and the second leg's edge (with their stations along the legs), its sweep (degrees, counter-clockwise
/// positive from the first tangent point) and its length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionReturn {
    pub a: String,
    pub b: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub cx: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub cy: f64,
    pub on_a: JunctionPlace,
    pub on_b: JunctionPlace,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub sweep: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
}

/// A junction as read now: its names, how its legs meet and whether they are joined (`connected`, at grade only),
/// whether an alignment changed since, the crossing's place, its legs and the angle between them (degrees, 0–180),
/// and at grade its four curb returns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionReport {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub kind: JunctionKind,
    pub connected: bool,
    pub stale: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    pub legs: Vec<JunctionLeg>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: f64,
    pub returns: Vec<JunctionReturn>,
}

/// A junction in the project's list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionListed {
    pub id: String,
    pub name: String,
    pub kind: JunctionKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// Output of `road.junctions.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionsWritten {
    /// The junction written, read; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub junction: Option<JunctionReport>,
    pub junctions: Vec<JunctionListed>,
    pub revision: String,
}

/// Input of `road.junctions.query` v1: every junction read, or one (`junction`: its id or name).
///
/// Refusals: `not_a_road_project`, `unknown_junction`, `invalid_junction` (the one asked does not read). Warnings:
/// `junction_stale`, `junction_on_curve`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub junction: Option<String>,
}

/// Output of `road.junctions.query` v1: the junctions read (one that does not read with no legs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionsQueried {
    pub junctions: Vec<JunctionReport>,
}

/// Input of `road.junctions.draw` v1: an at-grade junction's curb returns drawn on `layerId`, each an arc; with
/// `labels` a text at the crossing saying its name and the angle.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_junction`,
/// `invalid_junction` (a separated one has no returns), `layer_not_found`, `not_a_layer`, `layer_locked`. Warnings:
/// `layer_hidden`, and the read's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionsDraw {
    pub junction: String,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub labels: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.junctions.draw` v1, and its plan: the junction and the objects written (in the plan, none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionsDrawn {
    pub junction: String,
    pub uids: Vec<String>,
    pub revision: String,
}
