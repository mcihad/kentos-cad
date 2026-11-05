//! `cad.drawing.repair` (Onar, docs/adr/1005 §2): reviewed health findings fixed, one line end at a time: the end
//! moved along its own line onto the inside of a target's straight edge, by at most `maxGap`. Measured or
//! constrained geometry is refused, never moved; all or nothing, one undo step “Onar”, with what moved reported.

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

/// Input of `cad.drawing.repair` v1. Refusals, in order: `invalid_input`, `invalid_uid`, `invalid_gap`,
/// `invalid_revision`, `revision_conflict`, `entity_not_found`, `repair_unsupported`, `layer_locked`,
/// `repair_refused_measured`, `repair_target_missing`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingRepair {
    /// 1 to 1000 fixes, a line end once.
    pub fixes: Vec<JunctionFix>,
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

/// Output of `cad.drawing.repair` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingRepaired {
    pub fixed: Vec<JunctionFixed>,
    /// The largest move, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_moved: f64,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
