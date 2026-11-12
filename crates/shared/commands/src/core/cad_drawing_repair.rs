//! `cad.drawing.repair` (Onar, docs/adr/1005 §2, docs/adr/1012): reviewed health findings fixed: a line's end moved
//! along its own line onto the inside of a target's straight edge, by at most `maxGap`; a line turned about one end
//! onto the nearest axis when it is off by at most `maxAngle`; block definitions of one content merged into one.
//! Measured or constrained geometry is refused, never moved; all or nothing, one undo step “Onar”, with what moved
//! and the recipe applied (its order, thresholds and reach) reported.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_DRAWING_REPAIR: &str = "cad.drawing.repair";
pub const CAD_DRAWING_REPAIR_VERSION: u32 = 1;
/// The undo step's name.
pub const DRAWING_REPAIR_STEP: &str = "Onar";

/// Which end of a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LineEnd {
    Start,
    End,
}

/// One T-junction fix: the line `uid`'s `end` brought onto `target`'s nearest straight edge along the line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionFix {
    pub uid: String,
    pub end: LineEnd,
    pub target: String,
}

/// One angle fix (docs/adr/1012): the line `uid` turned about its `pivot` end onto the nearest of the horizontal and
/// vertical, its length kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AngleFix {
    pub uid: String,
    pub pivot: LineEnd,
}

/// One merge of block definitions of one content (docs/adr/1012): the inserts of `merge`'s definitions, in the
/// drawing and in other definitions, take `keep`'s, and the definitions left without inserts go.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlockMerge {
    /// The definition kept, by name.
    pub keep: String,
    /// The definitions merged into it, by name: 1 to 100.
    pub merge: Vec<String>,
}

/// Input of `cad.drawing.repair` v1. Refusals, in order: `invalid_input`, `invalid_uid`, `invalid_gap`,
/// `invalid_angle_tolerance`, `invalid_revision`, `revision_conflict`, `entity_not_found`, `unknown_block`,
/// `repair_unsupported`, `layer_locked`, `repair_refused_measured`, `repair_target_missing`,
/// `repair_angle_too_large`, `repair_blocks_differ`. Fixes are applied in that order: junctions, angles, blocks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingRepair {
    /// T-junction fixes, a line end once. With `angles` and `blocks`, 1 to 1000 fixes in all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<JunctionFix>>"))]
    pub fixes: Vec<JunctionFix>,
    /// Angle fixes, a line once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<AngleFix>>"))]
    pub angles: Vec<AngleFix>,
    /// Block definition merges, a definition once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<BlockMerge>>"))]
    pub blocks: Vec<BlockMerge>,
    /// The most a line may be turned, degrees, above 0 and at most 10; absent: 0.5 (the health check's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub max_angle: Option<f64>,
    /// The most an end may move, metres, above 0 and at most 1000; absent: 0.01.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_gap: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What one fix did.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionFixed {
    pub uid: String,
    pub end: LineEnd,
    /// How far the end moved, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length_before: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length_after: f64,
}

/// What one angle fix did.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EdgeStraightened {
    pub uid: String,
    pub pivot: LineEnd,
    /// How far it was off its axis, degrees.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub deviation: f64,
    /// How far its other end moved, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub moved: f64,
}

/// What one block merge did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksMerged {
    pub keep: String,
    pub merged: Vec<String>,
    /// Inserts that took `keep`'s definition: in the drawing and inside other definitions.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub inserts: u32,
}

/// One step of the recipe applied, in its order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RepairStep {
    Junction { uid: String, end: LineEnd, target: String },
    Angle { uid: String, pivot: LineEnd },
    Blocks { keep: String, merge: Vec<String> },
}

/// The recipe applied (docs/adr/1012): its steps in order, its thresholds and its reach. The journal keeps the input;
/// this says what the defaults made of it, so the same repair can be read back and run again.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RepairRecipe {
    pub steps: Vec<RepairStep>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_gap: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub max_angle: f64,
    /// Objects changed (lines moved or turned, inserts relinked).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// Definitions removed by a merge.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub definitions_removed: u32,
}

/// Output of `cad.drawing.repair` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingRepaired {
    pub fixed: Vec<JunctionFixed>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<EdgeStraightened>>"))]
    pub straightened: Vec<EdgeStraightened>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<BlocksMerged>>"))]
    pub merged: Vec<BlocksMerged>,
    pub recipe: RepairRecipe,
    /// The largest move, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
