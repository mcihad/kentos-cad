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

/// `cad.entities.raise` v1 (plan/map-cad.md MAP-06, docs/adr/3780): Referanstan kot ver, as Civil 3D's Raise/Lower by
/// Reference. The object's vertex `at` takes the reference's elevation plus the grade times its horizontal distance
/// from the reference (or plus the difference), and every other elevation of the object moves by as much: the object
/// is raised or lowered whole, its grades kept. One undo step “Referanstan kot ver”.
pub const CAD_ENTITIES_RAISE: &str = "cad.entities.raise";
pub const CAD_ENTITIES_RAISE_VERSION: u32 = 1;

/// The reference of `cad.entities.raise`: a place and its elevation (a point's, a vertex's or a typed one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RaiseReference {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
}

/// Input of `cad.entities.raise` v1: the object (a line, a polyline or an area), its vertex whose elevation is given
/// (0-based: a line's ends 0 and 1, a polyline's first part's vertices, an area's outer ring's), the reference, and
/// exactly one of the grade from the reference to the vertex (rise over run, up positive: 0.02 is 2 % up) and the
/// difference (the vertex's elevation minus the reference's, m).
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `invalid_uid`, `entity_not_found`, `not_a_path`
/// (not a line, a polyline or an area), `wrong_place` (no such vertex), `invalid_input` (both or neither of `grade`
/// and `difference`; a number not finite, the result's too), `elevation_missing` (the vertex has no elevation to move
/// from), `layer_locked`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesRaise {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub at: u32,
    pub reference: RaiseReference,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub grade: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub difference: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.raise` v1 (and its plan): the vertex's horizontal distance from the reference, its
/// elevation before and after, what every elevation of the object moved by (`after − before`, up positive), the grade
/// from the reference to the vertex after (none when the vertex stands on the reference), how many elevations moved
/// and how many vertices have none (they stay without, as Kot ver's Artır leaves them), and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesRaised {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub before: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub after: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub by: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub grade: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moved: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unset: u32,
    pub revision: String,
}

/// Yüzeyden kot ver (plan/map-cad.md MAP-06, docs/adr/4180), as Civil 3D's Elevations From Surface: each vertex of the
/// objects takes the project's terrain surface's height under it; one off the surface keeps its own. With `breaks`,
/// as its Insert Intermediate Grade Break Points, a vertex is put wherever a straight edge crosses an edge of the
/// surface's triangles. With `offset` (docs/adr/4580, its Relative Elevation to Surface's offset) the vertices take the
/// surface's height plus it; with `range` (its Partial) only one object's vertices from `from` to `to` take heights and
/// only the edges between them take break points. The heights are not kept tied to the surface. One undo step
/// “Yüzeyden kot ver”.
pub const CAD_ENTITIES_DRAPE: &str = "cad.entities.drape";
pub const CAD_ENTITIES_DRAPE_VERSION: u32 = 1;

/// The most objects one `cad.entities.drape` takes.
pub const MAX_DRAPED: usize = 10_000;

/// Input of `cad.entities.drape` v1: the objects (points, lines, polylines, areas: every part's and hole's vertices),
/// the map or road project's terrain surface (its id or name) and whether intermediate break points go in. Breaks go
/// only into straight edges of polylines and areas that carry no measures (M) nor widths: an arc is not split, a line
/// keeps its two ends. The height above (positive) or below (negative) the surface the vertices take (docs/adr/4580;
/// none: on it). The range of one object's vertices (docs/adr/4580): a line's ends, a polyline's or an area's outer ring's
/// vertices (its first part; holes and further parts keep theirs), 0-based, `from` < `to`; the closing edge is not in it.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_terrain`,
/// `invalid_input` (no object, more than 10 000, one given twice, an offset not finite, a range with more than one
/// object), `invalid_uid`, `entity_not_found`, `not_a_path` (not a point, a line, a polyline or an area; a point with a
/// range), `wrong_place` (the range not within the vertices, or `from` ≥ `to`), `terrain_unbuildable`, `off_surface` (no
/// vertex of them, or of the range, on the surface), `layer_locked`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesDrape {
    pub uids: Vec<String>,
    pub surface: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub breaks: bool,
    /// Docs/adr/4580: the height above (positive) or below (negative) the surface, m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: Option<f64>,
    /// Docs/adr/4580: only these vertices of the one object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub range: Option<DrapeRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// The vertices of one object `cad.entities.drape` gives heights to (docs/adr/4580): from `from` to `to`, 0-based.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrapeRange {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub from: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub to: u32,
}

/// One object as `cad.entities.drape` leaves it: its vertices given the surface's height, those off the surface (kept
/// as they were), the break points put in, and whether breaks were asked of an object that takes none (a point, a
/// line, a polyline with measures or widths).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrapedObject {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub set: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub inserted: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub unsplit: bool,
}

/// Output of `cad.entities.drape` v1 (and its plan): the surface's id, each object as given, the totals and the
/// drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesDraped {
    pub surface: String,
    pub objects: Vec<DrapedObject>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub set: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub inserted: u32,
    pub revision: String,
}
