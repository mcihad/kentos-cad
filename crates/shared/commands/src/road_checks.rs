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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub most_grade: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub least_grade: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/%")))]
    pub least_crest_k: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/%")))]
    pub least_sag_k: Option<f64>,
    /// The least crest radius H (docs/adr/0961): as RAA (Kuppenhalbmesser) and the standards that write a radius give
    /// the rule; a parabola's 1/|z''| = 100·K.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_crest_radius: Option<f64>,
    /// The least sag radius H (Wannenhalbmesser).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least_sag_radius: Option<f64>,
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
    /// A vertical curve's radius H short (docs/adr/0961).
    CrestRadius,
    SagRadius,
    Curve,
    GradeLength,
    /// A crossing's clearance or cover short (docs/adr/0946).
    Clearance,
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

pub const ROAD_PROFILES_LENGTHS: &str = "road.profiles.lengths";
pub const ROAD_PROFILES_LENGTHS_VERSION: u32 = 1;

/// Input of `road.profiles.lengths` v1 (docs/adr/0935 §2): each vertical curve's least length for a design speed
/// (km/h) by the criteria given, every value the standard's or the user's (KentOS fills in none):
///
/// - stopping sight (crest from an eye to an object, sag by headlights): the reaction `time` (s) and the `decel`
///   (m/s²), with `grades` the approach grades' (each direction's, the longer distance kept);
/// - `eye`, `object` (m): the crest's stopping sight; `passing` (m): a passing sight distance the crest must give,
///   from the eye to an eye-high object;
/// - `headlight` (m) and `beam` (degrees): the sag's headlight sight over the stopping distance;
/// - `accel` (m/s²): the sag's vertical acceleration; `external` (m): the least bisector, either kind;
/// - `step` (m): the suggestion rounded up to it.
///
/// Refusals: `invalid_profile_lengths` (no criterion, a value not right, a crest or headlight criterion without the
/// stopping sight's time and deceleration), `not_a_road_project`, `unknown_road_profile`. Warnings:
/// `vertical_curve_short` (a curve shorter than its least length), `profile_stale`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProfilesLengths {
    pub profile: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub speed: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub time: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/s2")))]
    pub decel: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub grades: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub eye: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub object: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub passing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub headlight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub beam: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/s2")))]
    pub accel: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub external: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: Option<f64>,
}

/// A criterion's least length: `stopping` (a crest's stopping sight), `passing`, `headlight`, `comfort`, `bisector`;
/// the sight distance it asks (m, the sight criteria) and the length (m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CurveCriterion {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sight: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
}

/// A vertical curve's lengths: its PVI, crest or sag, the grade change A (percent, signed), its length now (a circle's
/// as the parabola of the same K, R·|A|/100; none without a curve), each criterion's, the governing one's (its kind and
/// length), the suggestion (rounded up) and whether the curve is shorter than the governing length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CurveLengths {
    pub pvi: String,
    pub crest: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub a: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: Option<f64>,
    pub criteria: Vec<CurveCriterion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub governing: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub required: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub suggested: f64,
    pub short: bool,
}

/// Output of `road.profiles.lengths` v1: the profile, the stopping sight distance on the level (none without its time
/// and deceleration) and each curve's lengths in station order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProfileLengths {
    pub profile: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stopping: Option<f64>,
    pub curves: Vec<CurveLengths>,
}
