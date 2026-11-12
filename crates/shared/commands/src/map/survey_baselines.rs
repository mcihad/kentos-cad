//! `cad.survey.baselines` v1 (plan/map-cad.md MAP-05, docs/adr/1380): GNSS baz ağı dengelemesi, read only. Stations in
//! ECEF X, Y, Z (m), baselines ΔX, ΔY, ΔZ with their full 3×3 covariance, known stations fixed or weighted, a minimal
//! constraint when asked; the answer is the adjusted stations with their standard deviations in X, Y, Z and in local
//! east, north, up, each baseline's residual vector and test, and the network's m0 and χ² test. Nothing is written:
//! the analysis comes first, the decision to take its coordinates is the user's (a later step). The core is
//! `kentos_geometry_core::survey::adjust::gnss`; the shared cases are `fixtures/commands/v1/cad.survey.baselines.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_SURVEY_BASELINES: &str = "cad.survey.baselines";
pub const CAD_SURVEY_BASELINES_VERSION: u32 = 1;

/// A station: its name; its X, Y, Z when known (m); held fixed; or weighted by its standard deviations (m).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BaselineStation {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub xyz: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub fixed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: Option<[f64; 3]>,
}

/// A baseline: its ends, ΔX, ΔY, ΔZ (m) and its covariance's upper triangle xx, xy, xz, yy, yz, zz (m²).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BaselineVector {
    pub from: String,
    pub to: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub d: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m2")))]
    pub cov: [f64; 6],
}

/// How the datum is held: `given` by the fixed and weighted stations; `minimal` holds the first station with
/// coordinates when none is fixed or weighted (a minimally constrained adjustment).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BaselineDatum {
    #[default]
    Given,
    Minimal,
}

impl BaselineDatum {
    fn is_given(&self) -> bool {
        *self == BaselineDatum::Given
    }
}

/// Robust estimation's weight function (docs/adr/4960): `huber` c / |w|, `danish` exp(−((|w| − c) / c)²); 1 when
/// |w| ≤ c.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RobustMethod {
    #[default]
    Huber,
    Danish,
}

/// Robust estimation (docs/adr/4960): the baselines reweighted round by round by the weight function of their
/// standardised residual (the largest component's |v| / √q_vv, a priori σ0 = 1); `c` the constant (default Huber 1.5,
/// Danish 2.0). Nothing is removed; a baseline whose factor fell below 1 is to be checked.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BaselineRobust {
    pub method: RobustMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub c: Option<f64>,
}

/// How the robust rounds went: the weight function, its constant, the rounds, whether the factors settled (no factor
/// moved by more than 10⁻⁹; at most 100 rounds), how many baselines' factors are below 1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RobustRounds {
    pub method: RobustMethod,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub c: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rounds: u32,
    pub converged: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub downweighted: u32,
}

/// Input of `cad.survey.baselines` v1: the stations and baselines, or the same as `text` (lines `K;ad;X;Y;Z[;sX;sY;sZ]`
/// a known station, fixed without standard deviations; `N;ad[;X;Y;Z]` a new one; `B;from;to;dX;dY;dZ;xx;xy;xz;yy;yz;zz`
/// a baseline; `#` a remark; `;`, `,` or tabs between the values, a point before decimals); the variance factor the
/// covariances are multiplied by (default 1); the datum; robust estimation when asked (docs/adr/4960).
///
/// Refusals, in this order: `invalid_input` (text and lists both, a value not a number, a factor not above 0, a station
/// fixed and weighted, robust estimation's c not above 0 …), `no_baselines`, `duplicate_station`, `unknown_station`, `invalid_covariance` (not positive
/// definite), `datum_missing`, `network_too_large` (the normal matrix's envelope above 30 million entries, docs/adr/4560), `network_disconnected`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyBaselines {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BaselineStation>>", optional))]
    pub stations: Vec<BaselineStation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BaselineVector>>", optional))]
    pub baselines: Vec<BaselineVector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "BaselineDatum::is_given")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BaselineDatum>", optional))]
    pub datum: BaselineDatum,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub robust: Option<BaselineRobust>,
}

/// A station adjusted: X, Y, Z (m), their standard deviations and those in east, north, up (m, scaled by m0); held:
/// fixed or held by the minimal constraint (its standard deviations 0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AdjustedGnssStation {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub xyz: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sxyz: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub senu: [f64; 3],
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub held: bool,
}

/// A baseline's residual vector (computed − observed, m), its largest component test value (Baarda, decorrelated) and
/// what it says (`ok`, `blunder`, `uncontrolled`); with robust estimation its weight factor in the solution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BaselineResult {
    pub from: String,
    pub to: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub v: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub w: Option<f64>,
    pub flag: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub factor: Option<f64>,
}

/// Output of `cad.survey.baselines` v1: the stations in the input's order, the baselines in theirs, the station the
/// minimal constraint held, the counts (observations: three per baseline and per weighted station; unknowns; degrees of
/// freedom), vᵀPv, m0, the χ² quantile (95 %), whether the model test passed, the baseline with the largest flagged
/// test value, how the robust rounds went (when asked; the statistics are then the reweighted model's), and the
/// drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyBaselinesAdjusted {
    pub stations: Vec<AdjustedGnssStation>,
    pub baselines: Vec<BaselineResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub held: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub n: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub u: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub f: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub omega: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub m0: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub chi2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub worst: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub robust: Option<RobustRounds>,
    pub revision: String,
}
