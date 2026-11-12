//! Reviewed existing block placements become one array without replacing any object.
use crate::{ArrayId, ArrayItem, ArrayLayout, AssocArray};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ARRAYS_ADOPT: &str = "cad.arrays.adopt";
pub const CAD_ARRAYS_ADOPT_VERSION: u32 = 1;

/// Explicit, reviewed layout and slot membership; this command does not infer a pattern.
/// Only metadata is added. Missing slots stay missing; overrides must be explicitly kept.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArraysAdopt {
    /// Existing prototype insert UUID. Must occupy slot zero and must not be kept.
    pub source: String,
    pub layout: ArrayLayout,
    /// Exactly one entry per layout place, in layout order. Empty entries are missing places.
    /// At least two entries must be present and not kept. A UID may occur only once.
    pub items: Vec<ArrayItem>,
    /// Maximum position discrepancy in model metres; absent: 0.000001 (0.001 mm).
    /// Adoption never moves an insert to hide a discrepancy. Angles compare modulo a full turn
    /// within 1e-10 rad; scales within 1e-12 relative, independently of position tolerance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ArraysAdoptPlan {
    /// Proposed existing array model, with nil ID in a read-only plan.
    pub array: AssocArray,
    /// Maximum position discrepancy among the regular items, model metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub deviation: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub missing: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u32,
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ArraysAdopted {
    pub array: ArrayId,
    /// Existing insert UUIDs, in layout order, excluding missing places. No UUID is allocated here.
    pub retained: Vec<String>,
    pub revision: String,
}
