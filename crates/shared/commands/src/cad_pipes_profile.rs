//! `cad.pipes.profile` v1 (plan/map-cad.md MAP-16, docs/adr/1860): Hat profili, a gravity line's longitudinal profile
//! (kanal boykesiti) drawn on a layer. From a pipe of a utility network the run follows the flow through every node one
//! pipe leaves, to the outlet, a split or the pipe asked to end at; along it the ground (a terrain surface at every
//! triangle edge the pipes cross, or the structures' cover levels), each pipe's invert and crown (and, its wall known,
//! its outer walls, docs/adr/4780), the structures, and
//! under the frame the bands: the structure's name, the distance, the ground, the inverts in and out, the depth, and
//! each pipe's slope, diameter and length. The values are read as `cad.pipes.check` reads them (its inverts, diameter
//! and ground). The drawing follows `road.profiles.draw`'s rules (docs/adr/0902 §8): `at` is the frame's lower left
//! (the run's start at the datum), a metre of distance a drawing unit, a metre of elevation `exaggeration` units (10
//! when absent: a sewer profile's vertical scale is usually ten times its horizontal), texts `height` paper
//! millimetres at the drawing's scale (2.5), a band 2.4 heights, the bands' names 14 heights wide; the datum a metre
//! under the lowest elevation, floored. One undo step “Hat profili”. The shared cases are
//! `fixtures/commands/v1/cad.pipes.profile.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{PipeDiameter, PipeGround, PipeInverts};

pub const CAD_PIPES_PROFILE: &str = "cad.pipes.profile";
pub const CAD_PIPES_PROFILE_VERSION: u32 = 1;

/// The most pipes one profile follows.
pub const PIPE_PROFILE_LIMIT: usize = 10_000;

/// The bands under a pipe profile, top down: the structures' rows, then the pipes'.
pub const PIPE_PROFILE_BANDS: [&str; 9] = [
    "Baca",
    "Mesafe (m)",
    "Zemin kotu (m)",
    "Akar giriş (m)",
    "Akar çıkış (m)",
    "Derinlik (m)",
    "Eğim (‰)",
    "Çap (mm)",
    "Boy (m)",
];

/// Input of `cad.pipes.profile` v1: the run from `start` (a pipe's uid) along the flow, to `end` (a pipe's uid; absent:
/// where the flow leaves the network or splits), of `network`'s pipes, its values read as `cad.pipes.check` reads them,
/// drawn on `layerId` with its frame's lower left at `at`; `crossing`, another network whose pipes crossing the run in
/// plan are drawn where they cross (none: no such pipes; docs/adr/2760).
///
/// Refusals, in this order: `invalid_pipe_check` (an empty attribute's name, a catalogue of more than 200 sizes or a
/// size not above zero, the crossing network's too at `crossing.…`; `exaggeration` or `height` not above zero, a datum
/// not finite); `unknown_network`, `not_a_pipe_network`; the surface's (`not_a_road_project`, `unknown_terrain`,
/// `terrain_unbuildable`); `invalid_network`, `tools_unavailable`; the crossing network's at `crossing.network`
/// (`unknown_network`, `not_a_pipe_network`, `invalid_pipe_check` when it is the run's own or too large,
/// `invalid_network`); `invalid_pipe_run` (the start or the end not a pipe of the network, the end
/// not down the flow from the start, more than 10 000 pipes, no elevation to draw); `invalid_pipe_check` (a datum over the
/// lowest elevation); the layer's (`layer_not_found`, `not_a_layer`, `layer_locked`); `invalid_revision`,
/// `revision_conflict`. Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipesProfile {
    /// The network's id or name.
    pub network: String,
    /// The run's first pipe: its uid.
    pub start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end: Option<String>,
    pub inverts: PipeInverts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub diameter: Option<PipeDiameter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ground: Option<PipeGround>,
    pub layer_id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
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
    pub crossing: Option<PipeProfileOther>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// The network whose pipes cross the run (docs/adr/2760): its id or name, and where its inverts and diameters are read,
/// as the run's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeProfileOther {
    pub network: String,
    pub inverts: PipeInverts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub diameter: Option<PipeDiameter>,
}

/// A pipe of the other network crossing the run in plan (docs/adr/2760), as Civil 3D shows a crossing pipe in a
/// profile view: its uid and name (its `Ad`), the distance along the run, the place, its invert there and its diameter
/// (none where they are not known); drawn as an ellipse of its diameter across, its diameter times the exaggeration
/// high, from its invert up (a point at its invert without a diameter), its name and invert above. With its wall
/// (docs/adr/4360): the outer wall, an ellipse its diameter plus two walls across about the same centre, and the ring
/// between the two filled; the words above the outer wall.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeProfileCrossing {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub invert: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub diameter: Option<f64>,
    /// Its wall thickness (m), when the crossing network's diameter asks it and the pipe has it (docs/adr/4360): its
    /// outer wall drawn round the inner one, the ring between them filled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub wall: Option<f64>,
}

/// A pipe of the run: its uid and name (its `Ad` attribute), its distances along the run, its length, whether it is
/// drawn against the flow, its inverts up and down the flow, its slope, diameter (m), wall and least cover over the
/// crown (none asked: over the invert; its wall known: over its top outside, as `cad.pipes.check` measures it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeProfilePipe {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reversed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub upstream: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub downstream: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub slope: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub diameter: Option<f64>,
    /// Its wall thickness (m), when the diameter asks it (its own attribute or the outer diameter's) and the pipe's is
    /// known (docs/adr/4780): above zero, its outer walls are drawn, the bottom outside under the invert and the top
    /// outside over the crown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub wall: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub min_cover: Option<f64>,
}

/// A node of the run: its distance; the structure there (its uid and `Ad`), the ground over it (the surface's height
/// there, or the structure's cover level), the invert of the run's pipe coming in and of the one going out, and the
/// depth (the ground less the lower of the two).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeProfileStructure {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub ground: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub inlet: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub outlet: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub depth: Option<f64>,
}

/// A place of the ground line: its distance and height (none: off the surface).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipeProfileGround {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: Option<f64>,
}

/// Why the run ends: at the pipe asked (`end`), where the flow leaves the network (`outlet`), or at a node more than one
/// pipe leaves (`split`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PipeRunEnd {
    End,
    Outlet,
    Split,
}

/// Output of `cad.pipes.profile` v1, and its plan: the network's id, the run's pipes and nodes in the flow's order, the
/// ground line, the run's length, why it ends, the datum and the top of the frame (the crossing pipes in it too), the
/// exaggeration, the crossing pipes; the objects written (a plan: none) and the revision after it (a plan: before).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PipesProfiled {
    pub network: String,
    pub pipes: Vec<PipeProfilePipe>,
    pub structures: Vec<PipeProfileStructure>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PipeProfileGround>>", optional))]
    pub ground: Vec<PipeProfileGround>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    pub ends: PipeRunEnd,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub datum: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub top: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub exaggeration: f64,
    /// The other network's pipes crossing the run, along it (docs/adr/2760).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PipeProfileCrossing>>", optional))]
    pub crossings: Vec<PipeProfileCrossing>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
