//! Read-only CAD boundary and hatch diagnosis. Repairs remain separate, reviewable commands.
use crate::Vec2;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_DRAWING_INSPECT: &str = "cad.drawing.inspect";
pub const CAD_DRAWING_INSPECT_VERSION: u32 = 1;
pub const INSPECTION_SCOPE_MAX: usize = 50_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum InspectionGroup {
    Boundaries,
    Hatches,
    Annotations,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum InspectionSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DrawingInspect {
    /// Absent: all visible objects. Empty: no objects. Persistent UUIDs; duplicates count once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    /// Absent/empty: boundary, hatch and annotation checks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<InspectionGroup>>", optional))]
    pub checks: Vec<InspectionGroup>,
    /// Greatest gap to report, metres. Default 0.01; 0 disables near-gap pairing, not open-end checks.
    /// This is a diagnostic threshold; no source geometry is moved or healed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_gap: Option<f64>,
    /// Maximum findings, 1..1000; default 200. Work/geometry/result caps return explicit partial results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub include_hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct InspectionFinding {
    pub group: InspectionGroup,
    pub severity: InspectionSeverity,
    pub code: String,
    pub uids: Vec<String>,
    /// Exact world positions used for screen markers; two points may describe a gap.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub points: Vec<Vec2>,
    pub message: String,
    pub repair: String,
    /// A gap's measured length in metres, independent of the project's display unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum HatchLinkState {
    Associated,
    Unassociated,
    Broken,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HatchInspection {
    pub uid: String,
    pub state: HatchLinkState,
    pub sources: Vec<String>,
    pub missing: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pattern_origin: Option<Vec2>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DrawingInspected {
    pub findings: Vec<InspectionFinding>,
    pub hatches: Vec<HatchInspection>,
    /// Real measured values, displayed text, overrides and association state of inspected dimensions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::DimensionRead>>", optional))]
    pub dimensions: Vec<crate::DimensionRead>,
    /// Curves excluded from exact line/circular-arc topology diagnosis; never reported as clean.
    pub unsupported: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub examined: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub comparisons: u64,
    pub truncated: bool,
    pub revision: String,
}
