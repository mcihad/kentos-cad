//! `cad.topology.repair` (docs/adr/1260 §4): a repair recipe of the topology findings, its steps in order, each a
//! finding's lasting key and the fix to apply to it; the drawing as the steps before left it is checked again before
//! each, as GRASS's v.clean rebuilds its topology before each tool. The answer is the recipe as applied, every object it
//! changes with its measures before and after and how far its source vertices moved, the area it changes, the open
//! findings before and after; the plan answers the same and writes nothing. No blind “repair all”: every step names
//! its finding. The shared cases are in `fixtures/commands/v1/cad.topology.repair.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Applies a repair recipe to the topology findings (docs/adr/1260 §4).
pub const CAD_TOPOLOGY_REPAIR: &str = "cad.topology.repair";
pub const CAD_TOPOLOGY_REPAIR_VERSION: u32 = 1;

/// The most steps a recipe has.
pub const TOPOLOGY_REPAIR_STEPS_MAX: usize = 1000;

/// A step: the finding (its lasting key in the drawing as the steps before left it) and the fix (`subtractFirst`,
/// `removeVertex` …; one the finding offers).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyRepairStep {
    pub key: String,
    pub fix: String,
}

/// Input of `cad.topology.repair` v1: the recipe's steps, 1–1000, in the order they are applied; all or nothing, one
/// undo step “Topoloji düzelt”.
///
/// Refusals, in this order: `invalid_input` (no step, more than 1000), the revision (`invalid_revision`,
/// `revision_conflict`), then step by step: `finding_not_found` (`steps[i].key`: the drawing as the steps before left
/// it has no finding of that key), `repair_refused_exception` (`steps[i].key`: the finding is an exception, left on
/// purpose), `fix_unavailable` (`steps[i].fix`: the finding does not offer the fix, the fix has no answer, or it would
/// change a row outside a bound part), `layer_locked` (`steps[i]`: an object it changes is on a locked layer); last
/// `repair_beyond_acceptance` (`steps[i]`, the step that moved a source vertex farthest past the quality profile's
/// acceptance threshold). A job asked to stop answers `cancelled` and writes nothing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyRepair {
    pub steps: Vec<TopologyRepairStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A step as applied: the finding it put right, its rule, its problem and its objects' persistent ids.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyRepairApplied {
    pub key: String,
    pub fix: String,
    pub rule: String,
    pub problem: String,
    pub uids: Vec<String>,
}

/// An object the recipe changes: its measures before and after (an area's area, a line's length), `removed` when it
/// is deleted, how many of its source vertices stayed bit for bit and how many moved, the farthest and the total
/// distance they moved to the new geometry (metres), and the steps that changed it (0-based).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyRepairChange {
    pub uid: String,
    pub layer: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub removed: bool,
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
    pub moved: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub total_moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub steps: Vec<u32>,
}

/// Output of `cad.topology.repair` v1, and its plan: the recipe as applied, the objects it changes in the drawing's
/// order, the source vertices moved over them all (`moved`, `maxMoved`, `totalMoved`; `belowPrecision` of them by less
/// than the profile's storage precision, still moved and written as computed), the change of the areas' sum (square
/// metres; a deleted area's whole area), the open findings before and after (of every rule), and the revision after
/// (a plan: before).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyRepaired {
    pub recipe: Vec<TopologyRepairApplied>,
    pub changes: Vec<TopologyRepairChange>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moved: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub total_moved: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub below_precision: Option<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area_change: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub open_before: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub open_after: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
