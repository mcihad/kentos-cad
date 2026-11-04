//! KentOS Yol's design checks (plan/road.md Y21, docs/adr/0922; ROAD-ALIGN-11, ROAD-ALIGN-15, ROAD-PROFILE-13): an
//! alignment and a profile checked against the user's design rules, Civil 3D's design check sets and OpenRoads' design
//! standards. Every limit is the user's (the design standard's); KentOS gives none, a rule not given is not checked.
//! The desktop's handlers are `crates/native/application/src/road_alignments_check.rs` and `road_profiles_check.rs`;
//! they and the Python SDK pass the shared cases in `fixtures/commands/v1/road.alignments.check.json` and
//! `road.profiles.check.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const ROAD_ALIGNMENTS_CHECK: &str = "road.alignments.check";
pub const ROAD_ALIGNMENTS_CHECK_VERSION: u32 = 1;
pub const ROAD_PROFILES_CHECK: &str = "road.profiles.check";
pub const ROAD_PROFILES_CHECK_VERSION: u32 = 1;

/// Input of `road.alignments.check` v1 (docs/adr/0922 §1): an alignment (its id or name; one of its own PIs, not a
/// parallel axis) and the rules: the least radius, a curve's least circular length, a clothoid's least length, the
/// least tangent between two curves, between two curves turning opposite ways (reverse; joined ones are 0) and the same
/// way (broken-back; joined ones are compound, not this), the most ratio of two joined curves' radii, and with `kinks` a
/// PI where the direction turns with no curve. Reads only.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_alignment`, `invalid_design_check` (no rule given, a least
/// not above 0, a ratio under 1, a parallel axis), and the alignment's when it no longer builds. Warnings:
/// `design_rule_broken`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsCheck {
    pub alignment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_arc: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_spiral: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_tangent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_reverse: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_broken_back: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub most_radius_ratio: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub kinks: bool,
}

/// Input of `road.profiles.check` v1 (docs/adr/0922 §2): a profile (its id or name) and the rules: the steepest and
/// the flattest grade between two PVIs (ratios, either way), a crest's and a sag's least K (metres per percent), a
/// vertical curve's least length and the least straight grade between two curves. Reads only.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_road_profile`, `invalid_design_check` (no rule given, a
/// limit not above 0), and the profile's when it no longer builds. Warnings: `design_rule_broken`, `profile_stale`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProfilesCheck {
    pub profile: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub most_grade: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub least_grade: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/%")))]
    pub least_crest_k: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/%")))]
    pub least_sag_k: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_curve: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_grade_length: Option<f64>,
}

/// Which design rule a finding breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DesignRule {
    Radius,
    Arc,
    Spiral,
    Tangent,
    Reverse,
    BrokenBack,
    RadiusRatio,
    Kink,
    Grade,
    LeastGrade,
    CrestK,
    SagK,
    Curve,
    GradeLength,
}

/// A design rule broken: the rule, the PIs or PVIs, the stations it spans (internal, m), the value found and the limit
/// (none for a kink), and what it is in words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DesignFinding {
    pub rule: DesignRule,
    pub of: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub limit: Option<f64>,
    pub message: String,
}

/// Two curves one after another: their PIs, whether they turn opposite ways, the tangent between them (ST to TS) and
/// where, whether they are joined (no tangent).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CurvePair {
    pub between: [String; 2],
    pub reverse: bool,
    pub joined: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tangent: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
}

/// Output of `road.alignments.check` v1: the alignment's id, its curve pairs and the rules broken, in station order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsChecked {
    pub alignment: String,
    pub pairs: Vec<CurvePair>,
    pub findings: Vec<DesignFinding>,
}

/// Output of `road.profiles.check` v1: the profile's id and the rules broken, in station order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProfilesChecked {
    pub profile: String,
    pub findings: Vec<DesignFinding>,
}
