//! KentOS Yol's profile from a drawn profile (plan/road.md Y10.6b, docs/adr/0908 §13; ROAD-IO-14): `road.profiles.fit`
//! reads a vertical alignment off a line or an open polyline drawn in a profile view (a road project's profile drawn in
//! DWG or DGN): its grades, its PVIs and its vertical curves; the line is kept, the profile's deviation from it said.
//! The desktop's handler is `crates/native/application/src/road_profiles_fit.rs`; it and the Python SDK pass the shared
//! cases in `fixtures/commands/v1/road.profiles.fit.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ProfilePvi;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const ROAD_PROFILES_FIT: &str = "road.profiles.fit";
pub const ROAD_PROFILES_FIT_VERSION: u32 = 1;

/// Input of `road.profiles.fit` v1: the line or open polyline by its persistent id, the alignment the profile lies on,
/// and the view it is drawn in: the drawing point `at` stands at `station` (the distance along the alignment, 0 without
/// it) and at the `datum` elevation, a metre of station a drawing unit, a metre of elevation `exaggeration` units (1
/// without it), as `road.profiles.draw` draws. A corner between two grades takes a symmetric parabola of `length` (none
/// without it); an arc is a vertical curve between its ends. The profile is named `name` (“‹n›. boykesit” without it)
/// and read against `surface`. A `tolerance` (m) warns where the profile lies farther from the line.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_alignment`,
/// `unknown_terrain`, `entity_not_found`, `invalid_profile_fit` (not a line or an open polyline, a datum not finite, an
/// exaggeration, a length or a tolerance not above 0, fewer than two stations, a station that does not grow, two arcs
/// in a row, an arc whose tangents do not meet, a PVI off the alignment, curves that do not fit, a name taken).
/// Warnings: `profile_fit_deviation`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesFit {
    pub uid: String,
    pub alignment: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub datum: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub exaggeration: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.profiles.fit` v1, and its plan: the profile made (its id, name, alignment and PVIs), its grades and
/// curves, the stations it spans, the line's id, and the largest vertical distance of the line's vertices (and its
/// arcs' middles) from the profile with the station and elevation where it is; the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileFitted {
    pub id: String,
    pub name: String,
    pub alignment: String,
    pub source: String,
    pub pvis: Vec<ProfilePvi>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub grades: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub curves: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub deviation: f64,
    /// The station and the elevation.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub worst: [f64; 2],
    pub revision: String,
}
