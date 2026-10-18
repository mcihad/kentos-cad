//! The semantic wall commands of the product command catalog (docs/adr/0339 §5, plan/cad.md `CAD-ARCH-05`, `09`):
//! walls along paths as drawn (`cad.walls.create`), walls from the drawing's lines (`cad.walls.convert`) and a
//! wall's own fields changed (`cad.walls.edit`). Each is one undo step.
//!
//! Their input is what the user gave: the path clicked, the width, justification, offset and style chosen, the lines
//! picked. The faces are drawn from that, never given; a style's components come from the project's table or KentOS's
//! own (`builtin_wall_styles`), taken into the project in the same step. Coordinates and lengths are metres on the wire
//! (x east (Y), y north (X)), whatever unit the project shows (docs/adr/0165 §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};
use crate::{ArchInfo, WallCaps, WallJustify, WallStyleDef};

pub const CAD_WALLS_CREATE: &str = "cad.walls.create";
pub const CAD_WALLS_CREATE_VERSION: u32 = 1;
pub const CAD_WALLS_FROM_LINES: &str = "cad.walls.convert";
pub const CAD_WALLS_FROM_LINES_VERSION: u32 = 1;
pub const CAD_WALLS_EDIT: &str = "cad.walls.edit";
pub const CAD_WALLS_EDIT_VERSION: u32 = 1;
/// The most walls one call writes.
pub const MAX_WALLS: usize = 10_000;
/// Fromlines' pairs: the widest wall two lines make, metres, when none is given.
pub const DEFAULT_PAIR_WIDTH: f64 = 0.6;
/// Fromlines' pairs: lines this many degrees apart are parallel, when none is given.
pub const DEFAULT_PAIR_ANGLE: f64 = 1.0;

/// One wall to write: its path as drawn and what the user chose for it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewWall {
    /// The path's vertices, two or more (three or more for a closed one of straight edges); no two in a row at one
    /// place.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// One per edge (DXF's bulge, tan(θ/4), counter-clockwise positive): one fewer than the points, as many on a
    /// closed path. Absent: every edge straight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    /// Kapat: the path's last vertex joins its first.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub closed: bool,
    /// Metres, above 0, at most 100. Absent: the style's new width, else 0.2 m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
    /// Where the body lies across the path. Absent: centred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub justify: Option<WallJustify>,
    /// The base line offset, metres, positive to the path's left. Absent: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: Option<f64>,
    /// A wall style by its id or its name: the project's, or KentOS's own (taken into the project). Absent: a plain
    /// wall of one body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<String>,
    /// Its ends. Absent: the style's, else closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub caps: Option<WallCaps>,
    /// Its height above its level's floor, metres. Absent: the level's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<f64>,
    /// Its level (a project level's id), phase, construction status and discipline. Absent: none said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
    /// GIS attributes, text in v1. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    /// The text shown beside it. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
}

/// Input of `cad.walls.create` v1: walls on a named layer, along their paths, written as one undo step “Duvar”. The
/// wall tool gives the path clicked and the width, justification, offset and style chosen (TODOS.md CMD-07: it reads
/// no active layer, colour or style).
///
/// A wall of a style takes the style's components, ends and fill; its width is the style's when all its components
/// have their thickness, else the input's (the style's new width when absent), the rest going to the component
/// without one. A style of KentOS's own is taken into the project's wall styles in the same step.
///
/// Refusals (`CommandError.code`), checked in this order: `no_objects` (no wall), `not_finite` (a coordinate, bulge
/// or length, at its place), `coincident_points`, `style_not_found`, `invalid_wall` (the path, the width, the offset
/// or the components), `wall_collapses` (an arc tighter than a face), `level_not_found`, `invalid_line_weight`,
/// `invalid_revision`, `revision_conflict` (status `conflict`), `layer_not_found`, `not_a_layer`, `layer_locked`; on
/// the desktop also `slots_exhausted`. Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallsCreate {
    /// The layer they go on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    pub walls: Vec<NewWall>,
    /// Their own colour (`EntityBase.color`). Absent: the layer's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Their own line weight, paper mm (0 to 100). Absent: the layer's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    /// Their own line type. Absent: the layer's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<crate::layer::LineType>,
    /// The document revision the input was prepared against, as decimal text; another one writes nothing (`conflict`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.walls.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallsCreated {
    /// The walls' persistent ids, in the input's order.
    pub uids: Vec<String>,
    /// Their slots in the open document.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    /// The ids of KentOS's wall styles taken into the project.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub styles_added: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.walls.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallsPlan {
    /// The walls as they would be stored, their `id` 0.
    pub walls: Vec<Entity>,
    /// KentOS's wall styles the project would take.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<WallStyleDef>>", optional))]
    pub styles_added: Vec<WallStyleDef>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// How lines become walls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WallsFromMode {
    /// Tek tek: a wall along each line, polyline, area's ring, arc or circle.
    #[default]
    Each,
    /// Çiftler: two parallel straight lines at most `maxWidth` apart, overlapping, a wall between them as wide as they
    /// are apart, along the first's direction over their overlap.
    Pairs,
}

/// Input of `cad.walls.convert` v1 (`CAD-ARCH-09`): walls from the drawing's line work, one undo step “Çizgiden
/// duvar”; the sources are taken away unless kept, those a wall does not cover whole are kept (`source_kept`).
///
/// Tek tek (`each`): a line, an open or closed polyline (its arcs too), an area's outer ring, an arc and a circle each
/// give one wall along it, of the width, justification and style given; another kind is left (`not_a_wall_path`).
///
/// Çiftler (`pairs`): straight lines (a line, a polyline of two vertices) are paired, the nearest first: two whose
/// directions are within `angle` degrees, at most `maxWidth` apart and whose projections overlap; the wall runs along
/// the first line's direction over the overlap, midway between them, as wide as they are apart, centred. Lines left
/// without a pair are reported (`wall_unpaired`).
///
/// Refusals: `no_entities`, `invalid_uid`, `entity_not_found`, `not_a_wall_path` (no source a wall is drawn along),
/// `wall_unpaired` (no pair in pairs mode), `not_finite`, `style_not_found`, `invalid_wall`, `wall_collapses`,
/// `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`, `layer_locked`. Warnings:
/// `not_a_wall_path`, `wall_unpaired`, `source_kept`, `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallsFromLines {
    /// The sources' persistent ids, at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    #[serde(default)]
    pub mode: WallsFromMode,
    /// Each mode's width, metres. Absent: the style's new width, else 0.2 m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
    /// Each mode's justification. Absent: centred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub justify: Option<WallJustify>,
    /// A wall style by its id or name. Absent: plain walls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<String>,
    /// Kaynağı koru: the sources stay. Absent: those a wall covers whole go.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub keep_sources: bool,
    /// Pairs: the widest wall, metres. Absent: 0.6 m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_width: Option<f64>,
    /// Pairs: degrees two lines may be apart and be parallel. Absent: 1°.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: Option<f64>,
    /// The layer the walls go on. Absent: each source's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.walls.convert` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallsFromLinesDone {
    /// The walls written, in the sources' order (a pair's at its first line's place).
    pub uids: Vec<String>,
    /// The sources taken away.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    /// The sources no wall was drawn from: another kind, or a line without a pair.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub left: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.walls.convert` would write (plan mode): the walls, the sources that would go and those left.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallsFromLinesPlan {
    pub walls: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub left: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<WallStyleDef>>", optional))]
    pub styles_added: Vec<WallStyleDef>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// A wall's own fields to change; absent ones stay (`cad.walls.edit`'s `set`). A nullable field's null takes it away.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallChange {
    /// Metres, above 0, at most 100; the component without a thickness takes the rest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub justify: Option<WallJustify>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: Option<f64>,
    /// A wall style by its id or name: its components, ends and fill; null: no style, one plain body.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub style: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub caps: Option<WallCaps>,
    /// The schematic fill (`#RRGGBB`); null: none.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub fill: Option<Option<String>>,
    /// Metres above its level's floor; null: the level's.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<Option<f64>>,
    /// Its level by the project level's id; null: none.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub level: Option<Option<String>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<crate::ArchPhase>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<crate::ArchPhase>"))]
    pub phase: Option<Option<crate::ArchPhase>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<crate::ArchStatus>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<crate::ArchStatus>"))]
    pub status: Option<Option<crate::ArchStatus>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Option<crate::ArchDiscipline>>", optional)
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<crate::ArchDiscipline>"))]
    pub discipline: Option<Option<crate::ArchDiscipline>>,
}

/// What `cad.walls.edit` does to the walls.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WallsOperation {
    /// Değiştir: the fields given, on every wall named.
    Set(WallChange),
}

/// Input of `cad.walls.edit` v1: walls named by their persistent ids changed as one undo step “Duvarı düzenle”, all
/// or none. A width a style's components do not fit, an arc a new width collapses, a level the project does not have:
/// nothing is written.
///
/// Refusals: `no_entities`, `invalid_uid`, `entity_not_found`, `not_a_wall`, `nothing_to_set`, `not_finite`,
/// `style_not_found`, `invalid_wall`, `wall_collapses`, `level_not_found`, `invalid_revision`, `revision_conflict`,
/// `layer_locked`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallsEdit {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub operation: WallsOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.walls.edit` v1 (its plan's too: the walls as they would be, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallsEdited {
    /// The walls as written, in the input's order.
    pub walls: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub styles_added: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
