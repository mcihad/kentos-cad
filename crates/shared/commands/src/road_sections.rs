//! KentOS Yol's cross section commands (plan/road.md Y5, docs/adr/0903): `road.templates.edit` writes or removes a
//! template (lanes, shoulders, ditch, cut and fill slopes); `road.sections.edit` writes, rebinds or removes a sample line
//! set (an alignment, its profile, a template and a surface, the swath widths and how the stations are had); and
//! `road.sections.query` reads a set's sections: the terrain along each sample line, the template's platform on it, its
//! slopes tied to the terrain (daylight) and the cut and fill areas. The geometry is `kentos-road`'s; a definition is a
//! project setting kept in the undo history: every write is one undo step. Only a road project has them. The desktop's
//! handlers are `crates/native/application/src/road_sections*.rs`; they and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.templates.edit.json` and `road.sections.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{SectionStations, TemplateDef, TemplateSideDef};

pub const ROAD_TEMPLATES_EDIT: &str = "road.templates.edit";
pub const ROAD_TEMPLATES_EDIT_VERSION: u32 = 1;
pub const ROAD_SECTIONS_EDIT: &str = "road.sections.edit";
pub const ROAD_SECTIONS_EDIT_VERSION: u32 = 1;
pub const ROAD_SECTIONS_QUERY: &str = "road.sections.query";
pub const ROAD_SECTIONS_QUERY_VERSION: u32 = 1;

/// What an edit of a template or a set does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoadEditOperation {
    /// Writes it: a new one when none of the id or name is in the project (its id the given one, else the first free),
    /// else the given fields replace its.
    Set,
    /// A set takes its alignment's present revision (no longer stale).
    Rebind,
    /// It goes.
    Remove,
}

/// Input of `road.templates.edit` v1: the template by its id or name (absent: a new one).
///
/// Refusals, in this order: `not_a_road_project`, `unknown_road_template` (remove or rebind one not in the project),
/// `invalid_road_template` (a new one without `name` or `left`, set with nothing to set, rebind, or the definition by
/// `road_problem`, path the field's), `road_template_in_use` (remove while a sample line set uses it), `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplatesEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub left: Option<TemplateSideDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub right: Option<TemplateSideDef>,
    /// The right side mirrors the left (its own goes).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mirror: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A template in the project's list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplateListed {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    /// The sample line sets that use it.
    pub used_by: Vec<String>,
}

/// Output of `road.templates.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplatesWritten {
    /// The template written; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub template: Option<TemplateDef>,
    pub templates: Vec<TemplateListed>,
    pub revision: String,
}

/// Input of `road.sections.edit` v1: the set by its id or name (absent: a new one). A new set needs `name`,
/// `alignment`, `left`, `right` and `stations`; it holds its alignment's present revision. `profile`, `template` and
/// `surface` take an id or a name; empty clears.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_section_set` (remove or rebind one not in the project),
/// `unknown_alignment`, `unknown_road_profile`, `unknown_road_template`, `unknown_terrain`, `invalid_section_set` (a new one
/// without its fields, set with nothing to set, the definition by `road_problem`; a station off the alignment, a range
/// not on it, a line that is not a line or polyline or does not cross the alignment, no station at all),
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alignment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stations: Option<SectionStations>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A sample line set as it is now: its definition's names and its stations (none when they no longer come out: its line
/// erased or no longer crossing, its alignment shortened; the set's own query says why).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionSetReport {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub alignment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    /// The alignment changed since it was made or rebound.
    pub stale: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: Vec<f64>,
}

/// Output of `road.sections.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionsWritten {
    /// The set written; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set: Option<SectionSetReport>,
    pub sets: Vec<SectionSetReport>,
    pub revision: String,
}

/// Input of `road.sections.query` v1: every set's report, or one's (`set`: its id or name) with its sections; only
/// those at `stations` (among the set's) when given.
///
/// Refusals: `not_a_road_project`, `unknown_section_set`, `invalid_section_set` (the set's stations no longer come out),
/// `invalid_places` (a station not among the set's, or a question
/// without `set`), the geometry's when the alignment or the profile no longer builds. Warnings: `section_set_stale`
/// (the alignment changed since), `section_surface_gone` (the surface is not in the project or makes no surface),
/// `section_open` (a side whose slope meets the terrain nowhere on the swath).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: Vec<f64>,
}

/// A place of the terrain along a sample line: offset (right positive), height (none off the surface), whether the
/// stretch before it is off the surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionGround {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: Option<f64>,
    pub gap: bool,
}

/// A side of a section as built: `cut`, `fill` or `open`, its hinge, its daylight point and its points outward from
/// the axis (offset, elevation).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionSide {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub hinge: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub daylight: Option<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub points: Vec<[f64; 2]>,
}

/// A cross section: where it is, the terrain along it and, with a profile, a template and a surface, the platform
/// built on it and its cut and fill areas (m²).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    pub km_text: String,
    /// The sample line's left and right ends.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub ends: [[f64; 2]; 2],
    /// The axis' design elevation; absent without a profile or outside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub axis: Option<f64>,
    pub ground: Vec<SectionGround>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub left: Option<SectionSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub right: Option<SectionSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub cut: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub fill: Option<f64>,
}

/// Output of `road.sections.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionsQueried {
    pub sets: Vec<SectionSetReport>,
    pub sections: Vec<SectionRow>,
}
