//! KentOS Yol's vertical alignment commands (plan/road.md Y4, docs/adr/0902): `road.profiles.create` makes a vertical
//! alignment (a profile) from PVIs on an alignment; `road.profiles.edit` changes its name, PVIs or surface, rebinds it
//! to its alignment's present revision, or removes it; and `road.profiles.query` reads its grades and curves (K, crest
//! or sag, the highest or lowest point), the design (red) and terrain (black) elevations at stations, and the terrain
//! profile along its alignment. The geometry is `kentos-road`'s (`kentos.profile/1`); a definition is a project setting
//! kept in the undo history: every write is one undo step. Only a road project (`cadWork: road`) has them. The
//! desktop's handlers are `crates/native/application/src/road_profiles*.rs`; they and the Python SDK pass the shared
//! cases in `fixtures/commands/v1/road.profiles.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::ProfilePvi;

pub const ROAD_PROFILES_CREATE: &str = "road.profiles.create";
pub const ROAD_PROFILES_CREATE_VERSION: u32 = 1;
pub const ROAD_PROFILES_EDIT: &str = "road.profiles.edit";
pub const ROAD_PROFILES_EDIT_VERSION: u32 = 1;
pub const ROAD_PROFILES_QUERY: &str = "road.profiles.query";
pub const ROAD_PROFILES_QUERY_VERSION: u32 = 1;

/// Input of `road.profiles.create` v1: a vertical alignment on an alignment (its id or name; one of its own PIs, not a
/// parallel axis), through PVIs whose stations are the alignment's own (horizontal length from its start, not km);
/// optionally read against a terrain surface (its id or name). Its id when not given is the first free of `b-1`, `b-2`
/// … It holds the alignment's present revision.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_alignment`, `unknown_terrain`, `invalid_road_profile` (the
/// definition by `road_problem`, path the field's; PVIs that make no vertical alignment: too few, an id twice, a station
/// not after the one before, a curve on the start or the end, a curve where the grade does not change; or a PVI off
/// the alignment), `profile_curves_overlap` (two curves, or a curve and an end, pass each other; the message names both
/// PVIs and by how much), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesCreate {
    pub name: String,
    pub alignment: String,
    pub pvis: Vec<ProfilePvi>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `road.profiles.edit` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProfilesEditOperation {
    /// The given fields (`name`, `pvis`, `surface`; an empty `surface` clears it) replace the profile's.
    Set,
    /// The profile takes its alignment's present revision: no longer stale. Its PVIs are not moved (docs/adr/0902 §6).
    Rebind,
    /// The profile goes.
    Remove,
}

/// Input of `road.profiles.edit` v1: the profile by its id or name.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_road_profile`, `unknown_terrain`, `invalid_road_profile`
/// (set with nothing to set, or the definition and the PVIs as `road.profiles.create`), `profile_curves_overlap`,
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesEdit {
    pub profile: String,
    pub operation: ProfilesEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pvis: Option<Vec<ProfilePvi>>,
    /// The surface's id or name; empty: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Input of `road.profiles.query` v1: every profile's report, or one's (`profile`: its id or name) with the elevations
/// at its alignment's stations (`stations`), at every round `every` metres of km (`every`), and, with `terrain`, the
/// terrain profile along its alignment on its surface.
///
/// Refusals: `not_a_road_project`, `unknown_road_profile`, `invalid_places` (a value not finite, `every` not above 0,
/// more than 100 000 places; or a question without `profile`), `station_outside` (a station off the alignment),
/// `no_terrain` (`terrain` for a profile with no surface), the geometry's when the alignment or the profile no longer
/// builds. Warnings: `profile_stale` (its alignment changed since), `profile_surface_gone` (its surface is not in the
/// project or makes no surface: no black elevation).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub every: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub terrain: bool,
}

/// A grade between two PVIs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileGradeRow {
    /// The PVIs it runs between.
    pub between: [String; 2],
    /// Their stations.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: [f64; 2],
    /// Rise over run (0.025 is 2.5 %).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub grade: f64,
}

/// A vertical curve on a PVI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileCurveRow {
    pub pvi: String,
    /// `parabola`, `asymmetric` or `circle`.
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub g_in: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub g_out: f64,
    /// Its start and end: station and elevation.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub start: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub end: [f64; 2],
    /// Its horizontal length (end − start), m.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    /// A circle's radius.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: Option<f64>,
    /// A = 100·(gOut − gIn), percent.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub a: f64,
    /// K = length / |A|, metres per percent.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/%")))]
    pub k: f64,
    /// A crest (the grade falls) or a sag.
    pub crest: bool,
    /// Its highest (crest) or lowest (sag) point when it falls inside: station and elevation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub extreme: Option<[f64; 2]>,
}

/// A profile as it builds now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileReport {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    /// The alignment's id.
    pub alignment: String,
    /// The alignment's revision it was made or rebound against, and its present one.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_now: u32,
    /// The alignment changed since.
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    /// Its first and last PVI's stations and kms.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub kms: [f64; 2],
    pub grades: Vec<ProfileGradeRow>,
    pub curves: Vec<ProfileCurveRow>,
    /// `kentos.profile/1`.
    pub algorithm: String,
}

/// A profile in the project's list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadProfileListed {
    pub id: String,
    pub name: String,
    pub alignment: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub stale: bool,
}

/// Output of `road.profiles.create` and `.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesWritten {
    /// The profile written; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<ProfileReport>,
    pub profiles: Vec<RoadProfileListed>,
    pub revision: String,
}

/// The elevations at a station of the alignment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileAt {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    /// The km written `2+345.678`.
    pub km_text: String,
    /// The design (red) elevation and grade; absent outside the profile's first and last PVI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub red: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub grade: Option<f64>,
    /// The terrain (black) elevation; absent off the surface or without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub black: Option<f64>,
    /// Red − black: fill positive, cut negative; absent without both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub difference: Option<f64>,
}

/// A point of the terrain profile: where the alignment crosses a triangle's edge, an element's end, the alignment's
/// ends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileGroundRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    /// Absent off the surface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: Option<f64>,
    /// The stretch from the point before to this one is off the surface.
    pub gap: bool,
    /// Where the alignment crosses a triangle's edge.
    pub crossing: bool,
}

/// Output of `road.profiles.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesQueried {
    /// Every profile's report, or the one asked's.
    pub profiles: Vec<ProfileReport>,
    pub at: Vec<ProfileAt>,
    pub ticks: Vec<ProfileAt>,
    pub terrain: Vec<ProfileGroundRow>,
}

pub const ROAD_PROFILES_DRAW: &str = "road.profiles.draw";
pub const ROAD_PROFILES_DRAW_VERSION: u32 = 1;

/// Input of `road.profiles.draw` v1 (plan/road.md Y4, docs/adr/0902 §8): a profile drawn on a layer as lines and texts,
/// so it goes to sheets and DXF as Netcad's and Civil 3D's profile views do, from its first to its last PVI. The frame's
/// lower left is `at` (the first PVI's station at the `datum` elevation); a metre of station is a drawing unit, a metre
/// of elevation `exaggeration` units (10 when absent). Inside: the elevation grid with its heights, the terrain (black;
/// off the surface left out), the design (red) line and each curve's sizes and K. Under it the bands: km, black and red
/// elevations and their difference at every round `every` metres of km (50 when absent) and at the ends, and the grades.
/// Texts are `height` paper millimetres high at the project's drawing scale (2.5 when absent). The datum when absent is a
/// metre under the lowest elevation shown, floored; the top a metre over the highest, ceiled.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_road_profile`, `invalid_label` (`every`, `height` or
/// `exaggeration` not above 0, more than 100 000 ticks, a datum over the lowest elevation), the layer's
/// (`layer_not_found`, `not_a_layer`, `service_layer`, `layer_locked`), `invalid_revision`, `revision_conflict`, and
/// the geometry's when the profile no longer builds. Warnings: `layer_hidden`, `profile_stale`, `profile_surface_gone`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesDraw {
    pub profile: String,
    pub layer_id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub every: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub datum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub exaggeration: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.profiles.draw` v1, and its plan: the frame's lower left and upper right, the datum and top
/// elevations, the exaggeration and the elevation grid's step, the bands' columns in station order, the objects written
/// (none in a plan).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesDrawn {
    pub profile: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub frame: [[f64; 2]; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub datum: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub top: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub exaggeration: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub grid: f64,
    pub columns: Vec<ProfileAt>,
    pub uids: Vec<String>,
    pub revision: String,
}
