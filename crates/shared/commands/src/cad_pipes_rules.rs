//! The rule package `cad.pipes.check` may take its limits from (plan/map-cad.md MAP-16, docs/adr/4780): a gravity
//! line's least and greatest slope by the pipe's diameter class, the least cover and the greatest span between two
//! structures, for each system (sewer, storm water). KentOS's is the two 2017 regulations of the Ministry (Atıksu
//! Toplama ve Uzaklaştırma Sistemleri Hakkında Yönetmelik EK-1; Yağmursuyu Toplama, Depolama ve Deşarj Sistemleri
//! Hakkında Yönetmelik EK-1 Çizelge 1.4), `catalog/v1/rules/kanal-csb-2017.json`; none of its numbers is a constant in
//! code, and the user may give another package (the administration's specification) or limits of their own, which go
//! first. The regulations' words are “olabilir” and “alınabilir”: what the package finds is a finding, never a verdict
//! that a line conforms.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A diameter class of a system's slopes and its slopes, written 1:A as the regulations write them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeSlopeClass {
    /// The class's name as the table writes it (“Tali kanal”, “φ400”).
    pub name: String,
    /// The least diameter it holds, inclusive: millimetres as the table writes φ, compared with the pipe's diameter
    /// attribute read in millimetres. At least one of `from`, `over` and `to` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub from: Option<f64>,
    /// The diameter it holds those over, exclusive (millimetres; “> φ1000”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub over: Option<f64>,
    /// The greatest diameter it holds, inclusive (millimetres).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub to: Option<f64>,
    /// The least slope 1:A, by its A (300 is 1:300).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub least: f64,
    /// The exceptional least slope 1:A, by its A: a slope between it and the least is allowed only by exception
    /// (`exceptionalSlope`), one under it is too low. None: the table has no such column for the class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub exceptional: Option<f64>,
    /// The greatest slope 1:A, by its A (15 is 1:15).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub greatest: f64,
}

/// The structures' spacing for a range of diameters: one every `least` to `greatest` metres. Only the greatest is
/// checked: a pipe between two structures longer than it is said (`longSpan`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeSpanClass {
    /// The diameters below it, exclusive (millimetres; “< φ1200”). At least one of `below` and `over` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub below: Option<f64>,
    /// The diameters over it, exclusive (millimetres; “> φ1200”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub over: Option<f64>,
    /// The least spacing the regulation names (metres), shown, not checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least: Option<f64>,
    /// The greatest spacing (metres).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub greatest: f64,
}

/// A system's rules. The first slope class holding a diameter is its; a diameter in none has no slope limit from the
/// package (`noRule`), and one in no spacing class no greatest span.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeRuleSystem {
    /// The system's id, asked by `rules.system` (`atiksu`, `yagmursuyu` in KentOS's package).
    pub id: String,
    /// Its name in words.
    pub name: String,
    /// The articles and tables its numbers come from.
    pub source: String,
    /// When its document holds (ISO date).
    pub effective: String,
    /// The least cover (metres, over the pipe's top as `cad.pipes.check` measures it); none: the package gives none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub min_cover: Option<f64>,
    /// The regulation's words about the least cover (when it may be less).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cover_note: Option<String>,
    /// The slope classes, by diameter.
    pub slopes: Vec<PipeSlopeClass>,
    /// The structures' spacing, by diameter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PipeSpanClass>>", optional))]
    pub spans: Vec<PipeSpanClass>,
    /// The regulation's words about the spacing (when it may be more).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub span_note: Option<String>,
}

/// A gravity line rule package.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeRules {
    /// The package's id.
    pub id: String,
    /// Its release.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub release: u32,
    /// Whose rules they are.
    pub jurisdiction: String,
    /// The documents, and when they were read.
    pub source: String,
    /// When the oldest of them holds (ISO date).
    pub effective: String,
    /// Its systems.
    pub systems: Vec<PipeRuleSystem>,
}

/// KentOS's package: the Ministry's 2017 sewer and storm water regulations' annexes.
pub fn pipe_rules_default() -> Option<PipeRules> {
    serde_json::from_str(include_str!("../../../../catalog/v1/rules/kanal-csb-2017.json")).ok()
}

/// The rules a check takes its limits from (docs/adr/4780). The limits the user gives (`minCover`, `minSlope`,
/// `maxSlope`) go before the package's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeRulesAsk {
    /// The system's id in the package (`atiksu` or `yagmursuyu` in KentOS's).
    pub system: String,
    /// The package; none: KentOS's (`catalog/v1/rules/kanal-csb-2017.json`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub package: Option<PipeRules>,
}
