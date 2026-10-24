//! Yatay ağ dengelemesi and Kot ağı dengelemesi as one product command (docs/adr/0203 §7–§8, 0622 §9, TODOS.md
//! `UX-14`).
//!
//! The caller gives the network as the windows' tables give it: the known points (fixed, or weighted with their σ;
//! a point given by its name alone is the drawing's) and the observations. The command adjusts it with the shared core
//! (`survey::adjust`), the project's a priori standard deviations and grid, and writes the new points, never the known
//! ones: Yatay ağ moves the drawing's points of the same name to their adjusted places, the vertices of lines,
//! polylines and areas on them with them, and adds the others on `layerId`; Kot ağı gives the drawing's points of the
//! same name their adjusted heights, their `Z (m)` and the vertices on them too, and says the points it has not. One
//! undo step named after the window; the windows write through it, and Python and an agent ask the same.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Adjusts a network and writes its new points.
pub const CAD_SURVEY_ADJUST: &str = "cad.survey.adjust";
pub const CAD_SURVEY_ADJUST_VERSION: u32 = 1;

/// A known point of a horizontal network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AdjustKnownPoint {
    /// Its name in the observations.
    pub name: String,
    /// Its place (x east, y north); none: the drawing's point of the name (its label, else its `Ad`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// Weighted: its standard deviation (m); none: fixed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: Option<f64>,
}

/// An observation of a horizontal network: from the station to the target, a direction reading in the input's angle
/// unit, a horizontal distance (m), either or both.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AdjustObservation {
    pub station: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
    pub direction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: Option<f64>,
}

/// A known height of a levelling network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AdjustKnownHeight {
    pub name: String,
    /// Its height (m); none: the height of the drawing's point of the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub h: Option<f64>,
    /// Weighted: its standard deviation (m); none: fixed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: Option<f64>,
}

/// A height difference from one point to another (m) over its length (m; a trigonometric height's horizontal
/// distance).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AdjustDifference {
    pub from: String,
    pub to: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub dh: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
}

/// How a levelling network's height differences were measured (docs/adr/0203 §5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LevelMethod {
    /// Geometrik nivelman: weighted by the length's kilometres.
    #[default]
    Geometric,
    /// Trigonometrik: weighted by the length's square.
    Trigonometric,
}

/// The network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AdjustWork {
    /// Yatay ağ dengelemesi: direction series and horizontal distances.
    Horizontal {
        known: Vec<AdjustKnownPoint>,
        rows: Vec<AdjustObservation>,
    },
    /// Kot ağı dengelemesi: height differences.
    Level {
        #[serde(default)]
        method: LevelMethod,
        known: Vec<AdjustKnownHeight>,
        rows: Vec<AdjustDifference>,
    },
}

/// `cad.survey.adjust`'s input.
///
/// Refusals, in this order: `not_finite` (a number of the network), `invalid_revision`, `revision_conflict`,
/// `point_not_found` (a known point by its name the drawing has not), the adjustment's (`survey_refused`), then the
/// writes': `layer_locked` (a point to move or a vertex on it on a locked layer), `no_layer` (new points and no layer),
/// then `cad.entities.create`'s for the layer (`layer_not_found`, `not_a_layer`, `layer_locked` …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyAdjust {
    pub work: AdjustWork,
    /// Yatay ağ: the layer of the new points the drawing has not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    /// The directions' unit; the project's when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<crate::AngleUnit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A new point adjusted: its place (Yatay ağ) or its height (Kot ağı), and its standard deviation (m: the position
/// error, or the height's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyAdjustedPoint {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub h: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub sigma: f64,
}

/// What `cad.survey.adjust` wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyAdjusted {
    /// The new points adjusted, in the adjustment's order.
    pub points: Vec<SurveyAdjustedPoint>,
    /// The drawing's points of their names moved (Yatay ağ) or given their heights (Kot ağı), in that order.
    pub moved: Vec<String>,
    /// Yatay ağ: the new points the drawing had not, added on `layerId`.
    pub created: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    /// Kot ağı: the new points the drawing has not; their heights are not written.
    pub absent: Vec<String>,
    /// The degrees of freedom, the a posteriori standard deviation of unit weight and the model test (none without
    /// redundancy).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub f: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub m0: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed: Option<bool>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
