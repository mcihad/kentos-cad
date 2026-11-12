//! `cad.levels.assign` v1 (“Kat ver”, docs/adr/0717): architectural objects given one of the project's levels as one
//! undo step, their places unchanged (Archicad's Relink Home Story: the element stays where it is). Without objects
//! named it gives the level to every object Denetim says lacks one (`arch_level_missing`): a wall, a space, a column, a
//! stair, a ramp or a room separator on no level or on a level the project does not have. A room's boundary is found
//! again among its new level's walls in the same step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};

/// The command's name.
pub const CAD_LEVELS_ASSIGN: &str = "cad.levels.assign";
/// Its version.
pub const CAD_LEVELS_ASSIGN_VERSION: u32 = 1;

/// Input of `cad.levels.assign` v1 (“Kat ver”, docs/adr/0717): `level` given to the objects named by `uids` (walls,
/// spaces, grids, columns, stairs, ramps, room separators), or, without `uids`, to every wall, space, column, stair,
/// ramp and room separator on no level or on a level the project does not have. An object on `level` already is left
/// as it is; one on a locked layer is left out and counted (`layer_locked` warning), all of them locked is refused.
///
/// Refusals, in this order: `no_entities` (`uids` empty), `invalid_uid`, `entity_not_found`, `not_levelled` (an
/// opening, a tag, a line… has no level of its own), `level_not_found`, `invalid_revision`, `revision_conflict`,
/// `layer_locked`, `nothing_to_assign`. Warnings: `layer_locked`, `arch_space_broken` (a room its new level's walls do
/// not enclose, or one whose walls left its level: its last boundary kept).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LevelsAssign {
    /// The level given, by its id among the project's levels.
    pub level: String,
    /// The objects given it, by their persistent ids; absent: every object without a level of the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// One object given the level.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LevelAssigned {
    /// Its persistent id.
    pub uid: String,
    /// Its id in the drawing.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub id: u32,
    /// Its kind: `wall`, `space`, `grid`, `column`, `stair`, `ramp` or `polyline` (a room separator).
    pub kind: String,
    /// The level it was on, by its id; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
}

/// Output of `cad.levels.assign` v1, its plan's too: the objects given the level, in the drawing's order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LevelsAssigned {
    /// The level given.
    pub level: String,
    pub assigned: Vec<LevelAssigned>,
    pub revision: String,
}
