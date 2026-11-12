//! Çizimi temizle's semantic command of the product command catalog (docs/adr/0305; AutoCAD's OVERKILL): what the
//! drawing carries twice, empty, on one line or with vertices that add nothing, found within a tolerance and cleaned as
//! one undo step, with a report of what went and how far it moved.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Entity;
#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};

/// Cleans duplicates, empty objects, lines on one line and needless vertices.
pub const CAD_ENTITIES_CLEANUP: &str = "cad.entities.cleanup";
pub const CAD_ENTITIES_CLEANUP_VERSION: u32 = 1;

/// The tolerance when none is given, metres (AutoCAD's OVERKILL's).
pub const CLEANUP_TOLERANCE: f64 = 1e-6;
/// The largest tolerance, metres.
pub const MAX_CLEANUP_TOLERANCE: f64 = 10.0;

/// A property two objects may differ in and still be the same (AutoCAD's OVERKILL “Ignore object property”).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CleanupIgnore {
    Layer,
    Color,
    LineType,
    LineTypeScale,
    LineWeight,
    Transparency,
    Symbol,
}

/// Input of `cad.entities.cleanup` v1, one undo step “Çizimi temizle”. Within `tolerance` metres:
///
/// - an empty object goes: a line, a path or an arc no longer than it, a circle no wider, a text without words;
/// - a duplicate goes, the first stays: one of the same kind whose points (a point, a line's ends either way, a path's
///   vertices in order, reversed or, closed, from any start, its arcs alike; a circle's centre and radius; an arc's
///   centre, radius and ends) lie within it, its other fields equal, its own properties equal but the ones in
///   `ignore`, and its data (attributes, label, attachments) equal;
/// - with `overlaps`, lines of one such group that lie on one line and overlap go into the first (with `endToEnd`
///   also lines meeting end to end): it reaches the farthest ends; lines with an elevation or a measure stay;
/// - a path's vertices that add nothing go: a repeat, and with `straightVertices` one on the straight line between its
///   neighbours; a vertex with its own elevation or measure, or where the widths change, stays.
///
/// The scope is `uids`, or with none every object on a visible, unlocked layer; a named object on a locked layer is
/// left out with a warning. Refusals: `invalid_uid`, `invalid_tolerance`, `invalid_revision`, `revision_conflict`
/// (status `conflict`), `entity_not_found`, `layer_locked` (every named object locked).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCleanup {
    /// The objects looked at: their persistent ids. Absent: every object on a visible, unlocked layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Option<Vec<String>>,
    /// How far points may lie apart and be one, metres, 0 to 10; absent: `CLEANUP_TOLERANCE`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
    /// Properties duplicates may differ in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CleanupIgnore>>", optional))]
    pub ignore: Vec<CleanupIgnore>,
    /// Lines on one line that overlap merge; absent: true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub overlaps: Option<bool>,
    /// Lines on one line that meet end to end merge too; absent: false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end_to_end: Option<bool>,
    /// A path's vertices on the straight line between their neighbours go; absent: true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub straight_vertices: Option<bool>,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What Çizimi temizle found.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CleanupReport {
    /// Empty objects removed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub empty: u32,
    /// Duplicates removed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub duplicates: u32,
    /// Lines merged into others (removed).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub merged: u32,
    /// Lines that took others (changed).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub merging: u32,
    /// Vertices dropped from paths.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub vertices: u32,
    /// Paths that lost vertices (changed).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub paths: u32,
    /// The farthest a point that went lies from what stays, metres: what the tolerance cost.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub loss: f64,
}

/// Output of `cad.entities.cleanup` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCleaned {
    /// The objects changed (lines merged into, paths thinned), in the scope's order.
    pub changed: Vec<String>,
    /// The objects removed, in the scope's order.
    pub removed: Vec<String>,
    pub report: CleanupReport,
    /// The document's revision after the write, as decimal text (the same when nothing changed).
    pub revision: String,
}

/// What `cad.entities.cleanup` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCleanupPlan {
    /// The objects that would change, as they would be written, in the scope's order.
    pub changed: Vec<Entity>,
    /// The objects that would go: their persistent ids, in the scope's order.
    pub removed: Vec<String>,
    pub report: CleanupReport,
    /// The document revision the plan was made against.
    pub revision: String,
}
