//! `cad.scenarios.impact` v1 (plan/map-cad.md MAP-10, docs/adr/1440): Senaryo ölçüleri (“Alternatif etkisi” until
//! 2026-11-07), read only. A scenario (a design alternative: a group whose layers stand for base layers, docs/adr/0210 §9) set beside the field state it
//! replaces: each measure with its unit and its method, on both sides and their difference; the areas by use; every
//! decision that changed, matched by its key (added, removed, its geometry, its decision, both), with its area and
//! capacity before and after; the gaps in the data that leave a measure short; the assumptions. No single score picks
//! a “best” plan. The base layers' stamp says what the comparison was made against: given the stamp of an earlier
//! comparison, the answer says whether the field state has changed since (the alternative is then stale, not
//! updated). The shared cases are `fixtures/commands/v1/cad.scenarios.impact.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_SCENARIOS_IMPACT: &str = "cad.scenarios.impact";
pub const CAD_SCENARIOS_IMPACT_VERSION: u32 = 1;

/// Input of `cad.scenarios.impact` v1: the scenario group's id; the field that names a decision's use (none:
/// `Kullanim`), the key that matches a decision across the two sides (none: `KararKimlik`), and the stamp of an
/// earlier comparison (`since`) to say whether the base has changed since. Two measures more, each only when asked,
/// their values the user's (docs/adr/2040; KentOS gives none): `impervious`, the impervious share (0–1) of each use's
/// area; `access`, the area of the `from` uses whose centroid lies within `distance` metres of an area of the `to` uses.
///
/// Refusals: `invalid_input` (`impervious.<use>`: a share not between 0 and 1; `access.distance`: not above zero;
/// `access.from`, `access.to`: no use given), `scenario_not_found` (`scenario`: no such node, or not a scenario group),
/// `nothing_to_compare` (`scenario`: none of its layers stands for a base layer).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenariosImpact {
    pub scenario: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub since: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub impervious: Option<std::collections::BTreeMap<String, f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub access: Option<ImpactAccess>,
}

/// Erişim (docs/adr/2040): the uses reached from (`from`, e.g. `konut`), the uses reached (`to`, e.g. `park`, `yesil`)
/// and the straight distance, metres, from a `from` area's centroid to the nearest `to` area (0 inside it).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactAccess {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
    pub from: Vec<String>,
    pub to: Vec<String>,
}

/// A base layer and the scenario's layer standing for it, by id and name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactLayer {
    pub base: String,
    pub layer: String,
    pub name: String,
}

/// A measure: its id (`objects`, `area`, `length`, `capacity`), what it is called, its unit (`count`, `m2`, `m`), its
/// value on the field state's side and on the alternative's, the difference (alternative − base), and how it is
/// computed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactMeasure {
    pub id: String,
    pub label: String,
    pub unit: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub base: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub alternative: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub difference: f64,
    pub method: String,
}

/// The areas of one use (the field's value; empty for none) on both sides, square metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactUse {
    pub code: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub base: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub alternative: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub difference: f64,
}

/// A decision that differs: its key, what changed (`added`, `removed`, `geometry`, `decision`, `both`), its use, area
/// and capacity on each side that has it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactChange {
    pub key: String,
    pub change: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base_use: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alternative_use: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub base_area: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub alternative_area: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub base_capacity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub alternative_capacity: Option<f64>,
}

/// Output of `cad.scenarios.impact` v1: the scenario, its pairs, the field and key used, the measures, the uses by
/// code, the decisions that changed (by key), the data's gaps, the assumptions, the base's stamp now and whether it
/// changed since the stamp given (none when none was given), and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenariosImpacted {
    pub scenario: String,
    pub name: String,
    pub layers: Vec<ImpactLayer>,
    pub field: String,
    pub key: String,
    pub measures: Vec<ImpactMeasure>,
    pub uses: Vec<ImpactUse>,
    pub changes: Vec<ImpactChange>,
    pub gaps: Vec<String>,
    pub assumptions: Vec<String>,
    pub stamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stale: Option<bool>,
    pub revision: String,
}
