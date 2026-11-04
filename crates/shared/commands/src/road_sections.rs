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
use crate::{PavementDef, SectionStations, TemplateDef, TemplateSideDef};

pub const ROAD_TEMPLATES_EDIT: &str = "road.templates.edit";
pub const ROAD_TEMPLATES_EDIT_VERSION: u32 = 1;
pub const ROAD_SECTIONS_EDIT: &str = "road.sections.edit";
pub const ROAD_SECTIONS_EDIT_VERSION: u32 = 1;
pub const ROAD_SECTIONS_QUERY: &str = "road.sections.query";
pub const ROAD_SECTIONS_QUERY_VERSION: u32 = 1;
pub const ROAD_SECTIONS_DRAW: &str = "road.sections.draw";
pub const ROAD_SECTIONS_DRAW_VERSION: u32 = 1;

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
    /// The pavement under the finish grade (docs/adr/0930 §2), replacing the template's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pavement: Option<PavementDef>,
    /// The template's pavement goes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub remove_pavement: bool,
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
    /// The superelevation schedule on the same alignment the lanes take their cross slopes from (docs/adr/0904 §3);
    /// its id or name, empty clears.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<String>,
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
    /// The template's pavement on the section, where it has one and it builds (docs/adr/0930 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pavement: Option<SectionPavement>,
}

/// A pavement on a section (docs/adr/0930 §1): each layer's name and area, the pavement under and above the ground,
/// the formation line and each layer's bottom (offset, elevation; left to right), and the earthwork from the formation
/// (Formasyon–Zemin): the road's cut plus the pavement under the ground, its fill less the pavement above it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionPavement {
    pub layers: Vec<SectionLayer>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub below: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub above: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub formation: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bottoms: Vec<Vec<[f64; 2]>>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub formation_cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub formation_fill: f64,
}

/// A pavement layer on a section: its name and area (m²).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionLayer {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
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

/// Input of `road.sections.draw` v1: a sample line set's sections (`set`: its id or name; only those at `stations`,
/// among the set's, when given) drawn on a layer as section views in a grid, lines and texts that go to sheets and
/// DXF (docs/adr/0903 §7). The views lie in rows of `columns` (5 when absent) from `at`, the sheet's upper left, left to
/// right then down; a view is the swath wide (a metre of offset a drawing unit) and the set's tallest elevation range
/// high (a metre of elevation `exaggeration` units, 1 when absent), its own datum a metre under its lowest elevation,
/// floored. Each view: its frame, the elevation and offset grids with their numbers, the terrain (black, off the
/// surface left out), the platform and slopes (red), the axis, the km over it and the axis elevation, cut and fill
/// areas under it. Texts are `height` paper millimetres high (2.5 when absent).
///
/// Refusals, in this order: `not_a_road_project`, `unknown_section_set`, `invalid_label` (`columns` not 1–100,
/// `height` or `exaggeration` not above 0), `invalid_section_set` (its stations no longer come out), `invalid_places`
/// (a station not among the set's), `no_terrain` (the set has no surface or it makes none), the layer's
/// (`layer_not_found`, `not_a_layer`, `service_layer`, `layer_locked`), `invalid_revision`, `revision_conflict`.
/// Warnings: `layer_hidden`, `section_set_stale`, `section_open`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionsDraw {
    pub set: String,
    pub layer_id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub columns: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub exaggeration: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A section view drawn: its station and km, its frame's lower left and upper right, its datum and top elevations, its
/// cut and fill areas (m²) when the platform was built.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionView {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    pub km_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub frame: [[f64; 2]; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub datum: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub top: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub cut: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub fill: Option<f64>,
}

/// Output of `road.sections.draw` v1, and its plan: the views in order, the elevation grid's and the offset grid's
/// steps, the exaggeration, the objects written (none in a plan).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionsDrawn {
    pub set: String,
    pub views: Vec<SectionView>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub grid: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offsets: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub exaggeration: f64,
    pub uids: Vec<String>,
    pub revision: String,
}
