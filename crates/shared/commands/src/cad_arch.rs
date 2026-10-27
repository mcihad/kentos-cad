//! The semantic commands of grids, columns, stairs and ramps (docs/adr/0343 §4, plan/cad.md `CAD-ARCH-31`–`36`): each
//! kind made (`cad.grids.create`, `cad.columns.create`, `cad.stairs.create`, `cad.ramps.create`) and changed
//! (`cad.arch.edit`). Each is one undo step.
//!
//! Their input is what the user gave: the points clicked, the grids picked, the sizes and counts chosen; the drawing is
//! derived from these. A stair's and a ramp's checks are warnings, never a refusal, and never say a regulation is met.
//! Lengths are metres on the wire, whatever unit the project shows (docs/adr/0165 §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};
use crate::{ArchInfo, ArchPhase, ColumnShape, GridBubbles, Railings};

pub const CAD_GRIDS_CREATE: &str = "cad.grids.create";
pub const CAD_GRIDS_CREATE_VERSION: u32 = 1;
pub const CAD_COLUMNS_CREATE: &str = "cad.columns.create";
pub const CAD_COLUMNS_CREATE_VERSION: u32 = 1;
pub const CAD_STAIRS_CREATE: &str = "cad.stairs.create";
pub const CAD_STAIRS_CREATE_VERSION: u32 = 1;
pub const CAD_RAMPS_CREATE: &str = "cad.ramps.create";
pub const CAD_RAMPS_CREATE_VERSION: u32 = 1;
pub const CAD_ARCH_EDIT: &str = "cad.arch.edit";
pub const CAD_ARCH_EDIT_VERSION: u32 = 1;
/// The most objects one call writes.
pub const MAX_ARCH_ELEMENTS: usize = 10_000;

/// One grid to draw.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewGrid {
    /// Its start and end.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// Two bulges, the first its arc's; absent: straight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    /// Absent: the one after the grid before it in this call, else after the drawing's last grid, else “A”.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bubbles: Option<GridBubbles>,
    /// The bubble's diameter on paper, millimetres; absent: 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub bubble_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
}

/// Parallel grids: the first from `from` to `to`, each next `spacings[i]` further to the `side`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GridSeries {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Vec2,
    /// The gaps between them, metres, each greater than 0.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub spacings: Vec<f64>,
    /// Which side of `from` → `to` the next grids go; absent: the left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<SeriesSide>,
    /// The first's label; absent: as a grid with none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bubbles: Option<GridBubbles>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub bubble_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
}

/// The side of a line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SeriesSide {
    #[default]
    Left,
    Right,
}

/// How far a grid system's grids reach past the outermost grids they cross when no extension is given, metres
/// (docs/adr/0707): KentOS's value.
pub const DEFAULT_GRID_EXTENSION: f64 = 1.5;

/// A grid system (docs/adr/0707): two families of straight grids crossing at right angles. Those across its x axis
/// (standing along its y axis) are `x` apart, the first through `origin`; those across its y axis `y` apart; the whole
/// turned `angle` about `origin`. Each grid reaches `extension` past the outermost grids it crosses; a grid across x
/// runs up its y axis, one across y along its x axis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GridSystem {
    /// Where its first two grids cross.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub origin: Vec2,
    /// Its turn, radians counter-clockwise; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub angle: Option<f64>,
    /// The gaps between the grids across its x axis, the first's from `origin` on, each above 0; empty: one grid.
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: Vec<f64>,
    /// The gaps between the grids across its y axis.
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: Vec<f64>,
    /// The first label of the grids across x, the next ones following on; absent: “1”.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub x_start: Option<String>,
    /// The first label of the grids across y; absent: “A”.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub y_start: Option<String>,
    /// How far each grid reaches past the outermost grids it crosses, metres, 0 or more; absent: 1.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub extension: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bubbles: Option<GridBubbles>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub bubble_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
    /// A column at every crossing, tied to its two grids, row by row from `origin` (the first grid across y's first);
    /// absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub columns: Option<SystemColumns>,
}

/// The columns a grid system puts at its crossings (docs/adr/0707).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SystemColumns {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<ColumnShape>,
    /// Across, metres (a round column's diameter).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// Along, metres; absent: its width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
    /// Their turn, radians counter-clockwise; absent: the system's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: Option<f64>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<bool>,
    /// The first column's tag, the next ones' following on (S1, S2 …); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tag_start: Option<String>,
}

/// Input of `cad.grids.create` v1 (“Aks”): `grids`, then the `series`, then the grid `system` and its columns
/// (docs/adr/0707), on a named layer, as one undo step.
///
/// Refusals, in this order: `no_objects`, `not_finite`, `invalid_grid`, `invalid_column`, `level_not_found`, `invalid_line_weight`, `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`,
/// `layer_locked`, `invalid_attribute`, `attribute_required`; on the desktop also `slots_exhausted`.
/// Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GridsCreate {
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<NewGrid>>", optional))]
    pub grids: Vec<NewGrid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub series: Option<GridSeries>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub system: Option<GridSystem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.grids.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GridsCreated {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.grids.create` would write (plan mode): the objects as they would be stored, their `id` 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GridsPlan {
    pub grids: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// One column to put.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewColumn {
    /// Its centre; with `grids`, the crossing nearest it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// The two grids it stands where they cross, by their persistent ids; it follows them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub grids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<ColumnShape>,
    /// Across, metres (a round column's diameter).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// Along, metres; absent: its width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
    /// Its turn, radians counter-clockwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: Option<f64>,
    /// Its type's name.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    /// Absent: from `tagStart` on, else none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
}

/// Columns at every crossing of the grids given, pair by pair in their order, within both grids.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ColumnCrossings {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub grids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<ColumnShape>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: Option<f64>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
}

/// Input of `cad.columns.create` v1 (“Kolon”): `columns`, then those at the `crossings`, on a named layer, as one undo
/// step. A column with grids stands at their crossing and the document keeps it there (docs/adr/0343 §3).
///
/// Refusals, in this order: `no_objects`, `not_finite`, `invalid_column`, `invalid_uid`, `entity_not_found`,
/// `not_a_grid`, `grids_parallel`, `level_not_found`, `invalid_line_weight`, `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`,
/// `layer_locked`, `invalid_attribute`, `attribute_required`; on the desktop also `slots_exhausted`.
/// Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ColumnsCreate {
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<NewColumn>>", optional))]
    pub columns: Vec<NewColumn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crossings: Option<ColumnCrossings>,
    /// The first untagged column's tag, the next ones' following on (S1, S2 …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tag_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.columns.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ColumnsCreated {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.columns.create` would write (plan mode): the objects as they would be stored, their `id` 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ColumnsPlan {
    pub columns: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// The riser height a stair's count keeps under when it is drawn from its `height` and no `maxRiser` is given
/// (docs/adr/0704 §1): KentOS's default, not a regulation's value.
pub const DEFAULT_MAX_RISER: f64 = 0.175;

/// One stair to draw. Its risers are given (`risers`, `riserHeight`) or drawn from its `height` (docs/adr/0704):
/// `risers` given or as many as keep each riser under `maxRiser`, each `height / risers` high.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewStair {
    /// Its walking line, up from its first point: two points straight, three an L, four a U.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// Its risers; 0 or absent with `height`: as many as keep each under `maxRiser`.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub risers: u32,
    /// A riser's height; 0 or absent with `height` (which gives it: `height / risers`).
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub riser_height: f64,
    /// A tread's depth along the walking line.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tread: f64,
    /// Its whole rise, the risers drawn from it (docs/adr/0704 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: Option<f64>,
    /// The riser height its count keeps under, with `height` only; absent: `DEFAULT_MAX_RISER`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_riser: Option<f64>,
    /// The plan's cut height above its foot: its cut line after the last riser wholly under it (not with `cut`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub cut_height: Option<f64>,
    /// True: its last point moves along its last segment so that its walking line ends at its last riser.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fit: Option<bool>,
    /// A landing's length at a turn; absent: the width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub landing: Option<f64>,
    /// The riser its cut line stands at; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub cut: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub numbering: Option<bool>,
    /// Its railings' sides; absent: none (docs/adr/0705 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub railings: Option<Railings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
}

/// Input of `cad.stairs.create` v1 (“Merdiven”): stairs on a named layer, as one undo step. How their fields fit
/// (`stair_short`, `stair_long`, `stair_step_rule`, `stair_rise_level`, `stair_riser_high`) are warnings.
///
/// Refusals, in this order: `no_objects`, `not_finite`, `invalid_stair` (the fields, the design from the height, the
/// cut height, the fit), `level_not_found`, `invalid_line_weight`, `invalid_revision`, `revision_conflict`,
/// `layer_not_found`, `not_a_layer`, `layer_locked`, `invalid_attribute`, `attribute_required`; on the desktop also
/// `slots_exhausted`.
/// Warnings: `layer_hidden`, `stair_short`, `stair_long`, `stair_step_rule`, `stair_rise_level`, `stair_riser_high`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StairsCreate {
    pub layer_id: String,
    pub stairs: Vec<NewStair>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.stairs.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StairsCreated {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.stairs.create` would write (plan mode): the objects as they would be stored, their `id` 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StairsPlan {
    pub stairs: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// One ramp to draw.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewRamp {
    /// Its centre line, up from its first point.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// How much higher its end is than its start.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub rise: f64,
    /// Its slope, per cent; absent: its rise over its sloped length.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub slope: Option<f64>,
    /// Its start's elevation; absent: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub base: Option<f64>,
    /// A landing's length at a turn; absent: the width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub landing: Option<f64>,
    /// True: its last point moves along its last segment so that its sloped length is its rise over its slope
    /// (docs/adr/0705 §1); needs a slope and a rise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fit: Option<bool>,
    /// Its railings' sides; absent: none (docs/adr/0705 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub railings: Option<Railings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
}

/// Input of `cad.ramps.create` v1 (“Rampa”): ramps on a named layer, as one undo step. A slope that does not fit
/// (`ramp_slope_mismatch`) and landings with no sloped run left (`ramp_no_run`) are warnings.
///
/// Refusals, in this order: `no_objects`, `not_finite`, `invalid_ramp` (the fields, then the fit),
/// `level_not_found`, `invalid_line_weight`, `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`,
/// `layer_locked`, `invalid_attribute`, `attribute_required`; on the desktop also `slots_exhausted`.
/// Warnings: `layer_hidden`, `ramp_slope_mismatch`, `ramp_no_run`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RampsCreate {
    pub layer_id: String,
    pub ramps: Vec<NewRamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.ramps.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RampsCreated {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.ramps.create` would write (plan mode): the objects as they would be stored, their `id` 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RampsPlan {
    pub ramps: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// The fields of a grid, column, stair or ramp to change; absent ones stay, a nullable one's null takes it away. A field
/// the object's kind has not refuses the whole call (`field_not_for_kind`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchChange {
    /// A grid's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// A grid's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bubbles: Option<GridBubbles>,
    /// A grid's; null: 8 mm.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub bubble_size: Option<Option<f64>>,
    /// A column's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<ColumnShape>,
    /// A column's, a stair's or a ramp's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
    /// A column's; null: its width.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<Option<f64>>,
    /// A column's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: Option<f64>,
    /// A column's type.
    #[serde(
        rename = "type",
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub kind: Option<Option<String>>,
    /// A column's.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub tag: Option<Option<String>>,
    /// A column's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<bool>,
    /// A stair's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub risers: Option<u32>,
    /// A stair's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub riser_height: Option<f64>,
    /// A stair's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tread: Option<f64>,
    /// A stair's or a ramp's; null: the width.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub landing: Option<Option<f64>>,
    /// A stair's.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<u32>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub cut: Option<Option<u32>>,
    /// A stair's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub numbering: Option<bool>,
    /// A stair's or a ramp's railings (docs/adr/0705 §2); null: none.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<Railings>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<Railings>"))]
    pub railings: Option<Option<Railings>>,
    /// A ramp's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub rise: Option<f64>,
    /// A ramp's.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub slope: Option<Option<f64>>,
    /// A ramp's; null: 0.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub base: Option<Option<f64>>,
    /// Any kind's.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub level: Option<Option<String>>,
    /// Any kind's.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<ArchPhase>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<ArchPhase>"))]
    pub phase: Option<Option<ArchPhase>>,
}

/// What `cad.arch.edit` does (docs/adr/0343 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArchOperation {
    /// Değiştir: the fields given, on every object named.
    Set(Box<ArchChange>),
    /// Numarala: the grids' labels or the columns' tags `start` and on, in the order given (all of one kind).
    Renumber { start: String },
    /// Akslara bağla: each column to the crossing of these two grids (nearest its centre).
    Attach {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        grids: Vec<String>,
    },
    /// Bağı kopar: each column off its grids, staying where it is.
    Detach,
}

/// Input of `cad.arch.edit` v1: grids, columns, stairs and ramps named by their persistent ids changed as one undo step
/// “Mimari nesneyi düzenle”, all or none.
///
/// Refusals: `no_entities`, `invalid_uid`, `entity_not_found`, `not_arch_element`, `nothing_to_set`,
/// `field_not_for_kind`, `mixed_kinds`, `not_finite`, `invalid_grid`, `invalid_column`, `invalid_stair`, `invalid_ramp`,
/// `not_a_grid`, `grids_parallel`, `level_not_found`, `invalid_revision`, `revision_conflict`, `layer_locked`.
/// Warnings: those of the stairs and ramps written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchEdit {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub operation: ArchOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.arch.edit` v1 (its plan's too: the objects as they would be, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchEdited {
    /// The objects as written, in their order.
    pub elements: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
