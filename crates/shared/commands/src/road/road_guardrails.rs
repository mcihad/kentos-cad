//! KentOS Yol's guardrails (plan/road.md Y11.1, docs/adr/0909 §1; ROAD-JUNC-12): `road.guardrails.edit` writes, rebinds
//! or removes a guardrail (a corridor, the project's rule table with its source, the step its crosses are read at, the
//! rail's place outward from the hinge); `road.guardrails.query` reads every one, or one, now: the runs on each side
//! with their km, where the barrier is warranted within them, the highest fill and steepest slope they guard;
//! `road.guardrails.draw` draws the runs in plan. The rule is `kentos-road`'s; a guardrail is a project setting kept in
//! the undo history, its runs are computed on every read. The desktop's handlers are
//! `crates/native/application/src/road_guardrails*.rs`; they and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.guardrails.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{GuardrailRuleDef, RoadEditOperation};

pub const ROAD_GUARDRAILS_EDIT: &str = "road.guardrails.edit";
pub const ROAD_GUARDRAILS_EDIT_VERSION: u32 = 1;
pub const ROAD_GUARDRAILS_QUERY: &str = "road.guardrails.query";
pub const ROAD_GUARDRAILS_QUERY_VERSION: u32 = 1;
pub const ROAD_GUARDRAILS_DRAW: &str = "road.guardrails.draw";
pub const ROAD_GUARDRAILS_DRAW_VERSION: u32 = 1;

/// Input of `road.guardrails.edit` v1: the guardrail by its id or name (absent: a new one). A new one needs `name`,
/// `corridor`, `rule` and `step`. Rebind takes the corridor's present revision.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_guardrail` (rebind or remove one not in the project),
/// `unknown_corridor`, `invalid_guardrail` (a new one without its fields, set with nothing to set, the definition by
/// `road_problem`: a rule without its source or rows, a height, slope or length out of bounds, a step or an offset out
/// of bounds; a corridor that does not build), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailsEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub guardrail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corridor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rule: Option<GuardrailRuleDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A guardrail run on a side (`left` or `right`): where it starts and ends (stations and km), its length, the stretch
/// where the barrier is warranted within it, and the highest fill and steepest slope (1:n) it guards.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailRun {
    pub side: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub warranted: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub slope: f64,
}

/// A guardrail as read now: its definition's names, whether its corridor changed since (`stale`), the rule's source,
/// the crosses read, its runs left then right in station order, and each side's total length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailReport {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub stale: bool,
    pub source: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub crosses: u32,
    pub runs: Vec<GuardrailRun>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right: f64,
}

/// A guardrail in the project's list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailListed {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

/// Output of `road.guardrails.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailsWritten {
    /// The guardrail written, read; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub guardrail: Option<GuardrailReport>,
    pub guardrails: Vec<GuardrailListed>,
    pub revision: String,
}

/// Input of `road.guardrails.query` v1: every guardrail read, or one (`guardrail`: its id or name).
///
/// Refusals: `not_a_road_project`, `unknown_guardrail`, `invalid_guardrail` (its corridor does not build). Warnings:
/// `guardrail_stale`, and the corridor's own (`corridor_stale`, `corridor_surface_gone`, `corridor_unsolved`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub guardrail: Option<String>,
}

/// Output of `road.guardrails.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailsQueried {
    pub guardrails: Vec<GuardrailReport>,
}

/// Input of `road.guardrails.draw` v1: the guardrail's runs drawn on `layerId` as polylines at the rail's place (each
/// cross's hinge plus the guardrail's offset outward, its ends between crosses where the run starts and ends), with
/// `labels` a text at each run's middle saying its km and length.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_guardrail`,
/// `invalid_guardrail` (its corridor does not build), `layer_not_found`, `not_a_layer`, `layer_locked`. Warnings:
/// `layer_hidden`, `guardrail_stale`, `corridor_stale`, `corridor_unsolved`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailsDraw {
    pub guardrail: String,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub labels: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.guardrails.draw` v1, and its plan: the runs drawn, their total length and the objects written (in
/// the plan, none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailsDrawn {
    pub guardrail: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub runs: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    pub uids: Vec<String>,
    pub revision: String,
}
