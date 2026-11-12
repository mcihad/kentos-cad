//! `cad.topology.clean` v1 (plan/map-cad.md MAP-07, docs/adr/1262): Topolojik temizlik as a command, with its moves
//! reported. The rules are docs/adr/0148's (the core `kentos_geometry_core::ops::topology::topology_clean`, the tool
//! `tool.topology` draws the same); the command adds what a large clean-up must say before it writes: every move
//! (which object, what kind, from where to where, how far), each changed object's source vertices kept bit for bit and
//! moved, the farthest and the total move, the areas' and lengths' change, and the moves past the quality profile's
//! acceptance threshold (docs/adr/1260 §2), which the write refuses. The plan reports and writes nothing. The shared
//! cases are `fixtures/commands/v1/cad.topology.clean.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::Vec2;

pub const CAD_TOPOLOGY_CLEAN: &str = "cad.topology.clean";
pub const CAD_TOPOLOGY_CLEAN_VERSION: u32 = 1;

/// Input of `cad.topology.clean` v1: what to correct (`uids`; none: every line, polyline, arc and area on a visible,
/// unlocked layer), the tolerance (metres, finite, at least 0.000001), the four works (Uçlar `ends` on, Köşeler
/// `vertices` off, Uzat `extend` on, Buda `trim` on when not given). The other objects on visible layers are supports:
/// never moved. A given object that cannot be corrected (on a locked or hidden layer, or of a kind the clean-up does
/// not correct) is counted in `skipped`, as the tool says it.
///
/// Refusals, in this order: `invalid_input` (`tolerance`), `invalid_revision`, `revision_conflict`, `invalid_uid` and
/// `entity_not_found` (`uids[i]`); the write (and `validate`) last `clean_beyond_acceptance` (`tolerance`: a move past
/// the quality profile's acceptance threshold). One undo step “Topolojik temizlik”; nothing to correct writes nothing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyClean {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ends: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vertices: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub extend: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trim: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A move: the object, its kind (`end` joined, `vertex` joined, `extended`, `trimmed`, `edge`: an end moved onto a
/// line), where it was, where it goes and how far (metres), in the core's order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyCleanMove {
    pub uid: String,
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
}

/// A changed object: its layer, its area (an area) or length (the others) before and after, its source vertices kept
/// bit for bit, its moves, the farthest and the total of them (metres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyCleanChange {
    pub uid: String,
    pub layer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area_before: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area_after: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length_before: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length_after: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moves: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub total_moved: f64,
}

/// The counts of docs/adr/0148 §9: ends joined, vertices joined, ends extended, trimmed, moved onto a line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyCleanCounts {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ends: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub vertices: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub extended: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub trimmed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edges: u32,
}

/// Output of `cad.topology.clean` v1, and its plan: the objects it corrects and the supports, the given ones skipped,
/// the counts, every move, the changed objects in the drawing's order, the moves over them all (`moved`, `maxMoved`,
/// `totalMoved`), the change of the areas' sum (square metres), the quality profile's acceptance threshold when one is
/// set and how many moves go past it, and the revision after (a plan: before).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyCleaned {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub supports: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    pub counts: TopologyCleanCounts,
    pub moves: Vec<TopologyCleanMove>,
    pub changes: Vec<TopologyCleanChange>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moved: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub total_moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area_change: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub acceptance: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub beyond: u32,
    pub revision: String,
}
