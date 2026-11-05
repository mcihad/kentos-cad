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
    /// What the road crosses (docs/adr/0946); an empty list takes them off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crossings: Option<Vec<crate::ProfileCrossingDef>>,
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
    /// Its least radius H as the standards that write a radius take it (docs/adr/0961): a parabola's 1/|z''|
    /// (100·K when symmetric; an asymmetric one's sharper half), a circle's own; absent where the grade does not change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub curve_radius: Option<f64>,
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
/// metre under the lowest elevation shown, floored; the top a metre over the highest, ceiled. `from` and `to` (internal
/// stations, m) draw a view of part of the profile, as a plan–profile sheet's (docs/adr/0908 §10): the frame, the lines,
/// the curves' texts, the bands and the grades between them only, the datum and top from what is shown there.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_road_profile`, `invalid_label` (`every`, `height` or
/// `exaggeration` not above 0, a range off the profile, backwards or empty, more than 100 000 ticks, a datum over the
/// lowest elevation), the layer's
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: Option<f64>,
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

pub const ROAD_PROFILES_SIGHT: &str = "road.profiles.sight";
pub const ROAD_PROFILES_SIGHT_VERSION: u32 = 1;

/// Which way a sight distance is read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SightSide {
    Forward,
    Backward,
    #[default]
    Both,
}

/// Input of `road.profiles.sight` v1 (docs/adr/0921; ROAD-PROFILE-08): a profile's sight distance at every `step`
/// metres from `from` to `to` (internal stations; the profile's ends when absent) and at `to`: an eye `eye` metres over
/// the profile, an object `object` metres over it, looked up to `reach` metres ahead, behind or both (`side`); with
/// `required` the stations where the profile hides the object nearer are gathered into stretches (where the profile or
/// the reach ends first, nothing is said of what lies past it). Each distance is stepped by `step` and bisected to
/// a millimetre. The heights, the reach, the step and the required distance are the user's (the design standard's);
/// KentOS gives none. The profile only: not the terrain, the plan or obstacles. Reads only.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_road_profile`, `invalid_sight` (eye or object not above 0
/// (a point on the profile grazes every line over a crest), reach, step or required not above 0, a range off the profile or backwards, more than 100 000 stations), and
/// the geometry's when the profile no longer builds. Warnings: `sight_short`, `profile_stale`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesSight {
    pub profile: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub eye: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub object: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub reach: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: Option<f64>,
    #[serde(default, skip_serializing_if = "is_both")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SightSide>", optional))]
    pub side: SightSide,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub required: Option<f64>,
}

fn is_both(s: &SightSide) -> bool {
    *s == SightSide::Both
}

/// Why a sight stops: the profile hides the object, the reach asked was seen, or the profile ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SightUntil {
    Hidden,
    Reach,
    End,
}

/// One direction's sight from a station: the distance seen, why it stops and, when hidden, the station where the
/// profile blocks the line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SightReach {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
    pub until: SightUntil,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub blocked: Option<f64>,
}

/// A station's sight: the profile's elevation there, ahead and behind (none at the profile's end that way, or not
/// asked).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SightRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub elevation: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub forward: Option<SightReach>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub backward: Option<SightReach>,
}

/// A stretch of stations, one after another, where the profile hides the object nearer than required one way: its first and last station and the
/// shortest distance in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SightShort {
    pub side: SightSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub least: f64,
}

/// The shortest sight one way where the profile hides the object: where and how far.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SightLeast {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
}

/// Output of `road.profiles.sight` v1: the profile's id, the heights, reach and step as asked, the bisection's
/// resolution (m), each station's sight, the shortest each way where the profile hides the object (none where it hides it
/// nowhere), and with `required` the stretches seen shorter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesSightRead {
    pub profile: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub eye: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub object: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub reach: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub resolution: f64,
    pub rows: Vec<SightRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub least_forward: Option<SightLeast>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub least_backward: Option<SightLeast>,
    pub short: Vec<SightShort>,
}

pub const ROAD_PROFILES_PROPOSE: &str = "road.profiles.propose";
pub const ROAD_PROFILES_PROPOSE_VERSION: u32 = 1;

/// Input of `road.profiles.propose` v1 (docs/adr/0926; ROAD-PROFILE-14): new elevations for a profile's (its id or
/// name) PVIs, their stations and parabolic curves kept, that follow its surface's terrain profile as closely as the
/// rules allow: ∫(red − black)² least over the stretches on the surface, exactly. The held PVIs keep their elevations;
/// no grade rises or falls more than `mostGrade` (a ratio); `balanced`: the area cut equal to the area filled. Reads
/// only; the proposal is written, if at all, as a new profile with `road.profiles.create`.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_road_profile`, `invalid_profile_proposal` (no surface, an
/// unknown held PVI, a most grade not above 0, a circular curve, no stretch on the surface, an elevation the ground
/// does not fix, rules that contradict; a surface that is not or does not build). Warnings: `profile_stale`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesPropose {
    pub profile: String,
    /// The PVIs (ids) that keep their elevations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub hold: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub most_grade: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub balanced: bool,
}

/// A proposed PVI: its id, station, the elevation proposed and the one it had, whether it was held.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProposedPvi {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub was: f64,
    pub held: bool,
}

/// Whether a proposed grade is at the most rise or fall.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GradeLimit {
    Rise,
    Fall,
}

/// A proposed grade between two PVIs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProposedGrade {
    pub between: [String; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub grade: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub limit: Option<GradeLimit>,
}

/// Output of `road.profiles.propose` v1: the profile's id, the PVIs and grades proposed, ∫(red − black)² (m²·m) over
/// the length on the surface (m) and its root mean square (m), the areas cut and filled (m²) along the profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileProposed {
    pub profile: String,
    pub pvis: Vec<ProposedPvi>,
    pub grades: Vec<ProposedGrade>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub squares: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub covered: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rms: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub fill: f64,
    /// Each crossing against the proposed grade line (docs/adr/0946); none where the profile has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CrossingMargin>>", optional))]
    pub crossings: Vec<CrossingMargin>,
}

/// A crossing against a grade line (docs/adr/0946): its name, station and side, the line's limit there (under a
/// structure its underside less the clearance, over one its top plus the cover), the line's elevation there (none off
/// the profile), the margin (positive clear, negative short) and, in a proposal, whether it holds the line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CrossingMargin {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub side: crate::CrossingSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub limit: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub margin: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub holds: bool,
}
