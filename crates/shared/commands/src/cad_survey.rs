//! The survey windows' points as one product command (docs/adr/0070, 0071, 0622 §9, TODOS.md `UX-14`): Poligon hesabı,
//! Kutupsal alım, Önden kestirme and Geriden kestirme. The caller gives the known points (by name or by coordinates)
//! and the observations as measured; the command computes the new points with the shared core (`survey::traverse`,
//! `survey::polar`, `survey::intersection`) and writes them as named points, one undo step named after the window.
//! The windows write through it; Python and an agent ask the same.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Computes and writes survey points.
pub const CAD_SURVEY_POINTS: &str = "cad.survey.points";
pub const CAD_SURVEY_POINTS_VERSION: u32 = 1;

/// A known point: a named point of the drawing (its label or its `Ad`, big and small letters alike), or coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum KnownPoint {
    Named {
        name: String,
    },
    At {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
}

/// A traverse's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TraverseShape {
    /// Bağlı: from a known point to another, oriented at the end when `fore` is given.
    Connected,
    /// Kapalı: back to its start, oriented on its back point.
    Closed,
    /// Açık: to no known point.
    Open,
}

/// One shot of Kutupsal alım.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyShot {
    pub name: String,
    /// The horizontal reading, in the angle unit.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
    pub reading: f64,
    /// The horizontal length, or the slope length with `zenith`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
    pub zenith: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub target_height: Option<f64>,
}

/// What is computed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SurveyWork {
    /// Poligon hesabı (docs/adr/0070): from `start` oriented on `back`, the angles (at the start, at every new point
    /// and, oriented at the end, at the end) and the legs' lengths; a connected traverse ends on `end` (oriented on
    /// `fore` when given), a closed one on its start; the misclosures shared out. The new points are named `names`
    /// in order (P1, P2 … for the ones not named).
    Traverse {
        shape: TraverseShape,
        start: KnownPoint,
        back: KnownPoint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        end: Option<KnownPoint>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        fore: Option<KnownPoint>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
        angles: Vec<f64>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        distances: Vec<f64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
        names: Vec<String>,
    },
    /// Kutupsal alım (docs/adr/0070): from `station` oriented on `back` read at `backReading`, each shot a point;
    /// with a zenith angle and the station's height its height too (the project's k).
    #[serde(rename_all = "camelCase")]
    Polar {
        station: KnownPoint,
        back: KnownPoint,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<f64>"))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
        back_reading: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
        station_z: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
        instrument_height: Option<f64>,
        shots: Vec<SurveyShot>,
    },
    /// Önden kestirme (docs/adr/0071): the point right of `a` → `b` seen at `alpha` clockwise from `b` at `a` and at
    /// `beta` clockwise from it to `a` at `b`.
    Forward {
        a: KnownPoint,
        b: KnownPoint,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
        alpha: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
        beta: f64,
        name: String,
    },
    /// Geriden kestirme (docs/adr/0071): the point where `a` → `b` is seen at `alpha` and `b` → `c` at `beta`, both
    /// clockwise.
    Resection {
        a: KnownPoint,
        b: KnownPoint,
        c: KnownPoint,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
        alpha: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
        beta: f64,
        name: String,
    },
}

/// `cad.survey.points`' input.
///
/// Refusals, in this order: `not_finite`, `point_not_found`, then the computation's (`survey_refused`), then
/// `cad.entities.create`'s (`layer_locked` …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyPoints {
    pub layer_id: String,
    pub work: SurveyWork,
    /// The angles' unit; the project's when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<crate::AngleUnit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `cad.survey.points` wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurveyPointsWritten {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    pub uids: Vec<String>,
    /// Poligon hesabı: the angular misclosure in the unit and the linear one in metres, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "survey-angle")))]
    pub angle_misclosure: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub linear_misclosure: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
