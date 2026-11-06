//! `cad.entities.grade` v1 (plan/map-cad.md MAP-06, docs/adr/1290): Eğimle kot ver. A line's, a polyline's or an area's
//! outer ring's vertices from `from` to `to` take elevations along a constant grade: the start's elevation (given, else
//! the vertex's own), then the end's elevation or the grade (rise over run); each between by its horizontal distance
//! along the path (an arc edge along its arc). One undo step “Eğimle kot ver”. The core is
//! `kentos_geometry_core::ops::grade`; the shared cases are `fixtures/commands/v1/cad.entities.grade.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_ENTITIES_GRADE: &str = "cad.entities.grade";
pub const CAD_ENTITIES_GRADE_VERSION: u32 = 1;

/// Input of `cad.entities.grade` v1: the object, the range's first and last vertex (0-based, `from` < `to`; an area's
/// outer ring), the start's elevation (none: the vertex's own), and exactly one of the end's elevation and the grade.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `invalid_uid`, `entity_not_found`, `not_a_path`
/// (not a line, a polyline or an area), `wrong_place` (the range not within the vertices, or `from` ≥ `to`),
/// `invalid_input` (both or neither of `end` and `grade`, a number not finite, a range of no length),
/// `elevation_missing` (no start given and the vertex has none), `layer_locked`.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.grade` v1 (and its plan): the range's horizontal length, its start's and end's elevations,
/// the grade (rise over run), each vertex's new elevation from `from` to `to`, and the drawing's revision.
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
    pub revision: String,
}
