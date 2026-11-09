//! `cad.entities.grade` v1 (plan/map-cad.md MAP-06, docs/adr/1290): Eğimle kot ver. A line's, a polyline's or an area's
//! outer ring's vertices from `from` to `to` take elevations along a constant grade: the start's elevation (given, else
//! the vertex's own), then the end's elevation or the grade (rise over run); each between by its horizontal distance
//! along the path (an arc edge along its arc). One undo step “Eğimle kot ver”. The core is
//! `kentos_geometry_core::ops::grade`; the shared cases are `fixtures/commands/v1/cad.entities.grade.json`.
//!
//! docs/adr/3380 adds the high or low point, as Civil 3D's Insert High/Low Elevation Point: `ahead` (the grade from the
//! start) and `back` (the grade from the end towards the start) instead of `grade`; where they meet a vertex goes (an
//! arc's halves keep its circle; a line becomes a polyline, as Köşe ekle makes it), unless an existing vertex lies
//! there; the vertices up to it take the grade ahead, those after it the grade back. The end's elevation is `end`, else
//! the end vertex's own. Extra fields: the command's version stays.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_ENTITIES_GRADE: &str = "cad.entities.grade";
pub const CAD_ENTITIES_GRADE_VERSION: u32 = 1;

/// Input of `cad.entities.grade` v1: the object, the range's first and last vertex (0-based, `from` < `to`; an area's
/// outer ring), the start's elevation (none: the vertex's own), and exactly one of the end's elevation and the grade;
/// or, for the high or low point (docs/adr/3380), both `ahead` and `back` without `grade`, the end's elevation being
/// `end` or else the end vertex's own.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `invalid_uid`, `entity_not_found`, `not_a_path`
/// (not a line, a polyline or an area), `wrong_place` (the range not within the vertices, or `from` ≥ `to`),
/// `invalid_input` (both or neither of `end` and `grade`; `grade` with `ahead` or `back`; one of `ahead` and `back`
/// alone), `elevation_missing` (no start given and the start vertex has none; for the high or low point, no end given
/// and the end vertex has none), `invalid_input` (a number not finite, a range of no length), `no_intersection` (the
/// two grades parallel, or meeting outside the range or at its start or end), `layer_locked`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesGrade {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub from: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub to: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub start: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub end: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub grade: Option<f64>,
    /// The high or low point's grade ahead: from the start into the range, rise over run (0.02 is 2 % up).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub ahead: Option<f64>,
    /// The high or low point's grade back: from the end towards the start, rise over run (0.02 is 2 % up from the end).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub back: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.grade` v1 (and its plan): the range's horizontal length, its start's and end's elevations,
/// the grade (rise over run; for the high or low point the grade ahead, and `back` the grade back), each vertex's new
/// elevation from `from` to `to` (an added vertex among them), the high or low point (`point`, docs/adr/3380) and the
/// drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesGraded {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub start: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub end: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub grade: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub zs: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub back: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub point: Option<GradedPoint>,
    pub revision: String,
}

/// The high or low point of `cad.entities.grade` (docs/adr/3380): its vertex after the write (0-based), whether that
/// vertex was added (else an existing one lay there), its horizontal distance from the start along the path, its place
/// and elevation, and which it is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GradedPoint {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u32,
    pub inserted: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub along: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
    pub kind: GradePointKind,
}

/// Which the point is: `high` (both grades rise towards it), `low` (both fall towards it) or `break` (neither: one
/// grade gives way to another).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GradePointKind {
    High,
    Low,
    Break,
}
