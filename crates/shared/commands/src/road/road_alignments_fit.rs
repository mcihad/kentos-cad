//! KentOS Yol's alignment from a drawn line (plan/road.md Y10.6, docs/adr/0908 §12; ROAD-IO-14): `road.alignments.fit`
//! makes an alignment of a line or an open polyline of the drawing (a road project's axis drawn in DWG or DGN), element
//! by element or by best fit, as Civil 3D's Create Alignment from Objects and Create Best Fit Alignment do; the line is
//! kept, the alignment's deviation from it said. The desktop's handler is
//! `crates/native/application/src/road_alignments_fit.rs`; it and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.alignments.fit.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::AlignmentPi;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const ROAD_ALIGNMENTS_FIT: &str = "road.alignments.fit";
pub const ROAD_ALIGNMENTS_FIT_VERSION: u32 = 1;

/// How a line becomes an alignment: its segments as they are (straight ones tangents, arcs circular curves), or a best
/// fit of its vertices (tangents by least squares, curves where the vertices bend tighter than `maxRadius`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AlignmentFitMethod {
    Elements,
    BestFit,
}

/// Input of `road.alignments.fit` v1: the line or open polyline by its persistent id, the method; with `elements` a
/// `radius` puts that curve where two tangents meet without an arc; with `bestFit` `maxRadius` (required) parts curves
/// from tangents. `reverse` runs the alignment from the line's end. The alignment is named `name` (“‹n›. aks” without
/// it) and its km starts at `start` (0 without it). A `tolerance` (m) warns where the alignment lies farther from the
/// line's vertices.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `entity_not_found`,
/// `invalid_alignment_fit` (not a line or an open polyline, fewer than two points, a radius, a maximum radius or a
/// tolerance not above 0, a maximum radius missing, compound or reverse arcs or tangents that do not meet, curves that do
/// not fit between their tangents, a name taken). Warnings: `alignment_fit_deviation`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsFit {
    pub uid: String,
    pub method: AlignmentFitMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub max_radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reverse: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub start: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.alignments.fit` v1, and its plan: the alignment made (its id, name and PIs), its tangents, curves and
/// length, the line's id, and the largest distance of the line's vertices (and its arcs' middles) from the alignment
/// with where it is; the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentFitted {
    pub id: String,
    pub name: String,
    pub source: String,
    pub method: AlignmentFitMethod,
    pub pis: Vec<AlignmentPi>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub tangents: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub curves: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub deviation: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub worst: [f64; 2],
    pub revision: String,
}
