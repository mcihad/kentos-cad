//! KentOS Yol's horizontal alignment commands (plan/road.md Y3, docs/adr/0901): `road.alignments.create` makes an
//! alignment from PIs (tangents, curves and clothoids) or a parallel axis of one; `road.alignments.edit` changes its
//! PIs, name, km or offset, reverses it, or removes it, and says what the edit did to its stations; and
//! `road.alignments.query` reads its elements and curves, the place at a station or a km, a point's station and
//! offset. The geometry is `kentos-road`'s (`kentos.alignment/1`); a definition is a project setting kept in the undo
//! history: every write is one undo step. Only a road project (`cadWork: road`) has them. The desktop's handlers are
//! `crates/native/application/src/road_alignments*.rs`; they and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.alignments.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{AlignmentOffset, AlignmentPi, AlignmentStationing};

pub const ROAD_ALIGNMENTS_CREATE: &str = "road.alignments.create";
pub const ROAD_ALIGNMENTS_CREATE_VERSION: u32 = 1;
pub const ROAD_ALIGNMENTS_EDIT: &str = "road.alignments.edit";
pub const ROAD_ALIGNMENTS_EDIT_VERSION: u32 = 1;
pub const ROAD_ALIGNMENTS_QUERY: &str = "road.alignments.query";
pub const ROAD_ALIGNMENTS_QUERY_VERSION: u32 = 1;

/// The most places one query asks (its stations, kms, points and ticks together).
pub const MAX_ALIGNMENT_PLACES: usize = 100_000;

/// Input of `road.alignments.create` v1: a new alignment from its PIs, or a parallel axis (`offsetOf`) of one; its id
/// when not given is the first free of `g-1`, `g-2` …
///
/// Refusals, in this order: `not_a_road_project`, `invalid_alignment` (the definition by `road_problem`, path the
/// field's; or PIs that make no alignment: two at one place, the direction turning back, a curve on the start or the
/// end), `alignment_spirals_long` (a PI's clothoids turn more than its deflection), `alignment_curves_overlap` (two
/// curves' tangent lengths pass each other; the message names both PIs and by how much), `invalid_stationing` (an
/// equation's back km is not after the one before it on the alignment), `offset_cusp` (the parallel axis folds where
/// a curve's radius is smaller than the offset, or crosses itself), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsCreate {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AlignmentPi>>", optional))]
    pub pis: Vec<AlignmentPi>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub offset_of: Option<AlignmentOffset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stationing: Option<AlignmentStationing>,
    /// The road's design speed, km/h (docs/adr/0985); a PI's `speed` stands for its own curve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub design_speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `road.alignments.edit` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AlignmentsEditOperation {
    /// The given fields (`name`, `pis`, `stationing`, `offset`, `designSpeed`) replace the alignment's.
    Set,
    /// The alignment runs the other way: its PIs in reverse order, each PI's clothoids swapped; its start km is kept and
    /// its equations go (the warning `equations_dropped`).
    Reverse,
    /// The alignment goes; refused while a parallel axis follows it (`alignment_in_use`).
    Remove,
}

/// Input of `road.alignments.edit` v1: the alignment by its id or name.
///
/// Refusals, in this order: `not_a_road_project`, `unknown_alignment`, `invalid_alignment` (set with nothing to set, `pis`
/// on a parallel axis or `offset` on an alignment of its own, or the definition by `road_problem`), the geometry's as
/// `road.alignments.create` (`alignment_spirals_long`, `alignment_curves_overlap`, `invalid_stationing`), `offset_cusp`
/// (the edited alignment or a parallel axis following it folds), `alignment_in_use` (remove while a parallel axis
/// follows it), `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsEdit {
    pub alignment: String,
    pub operation: AlignmentsEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pis: Option<Vec<AlignmentPi>>,
    /// The km; an empty one (no start, no equation) clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stationing: Option<AlignmentStationing>,
    /// A parallel axis's offset, m, right positive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: Option<f64>,
    /// The road's design speed, km/h (docs/adr/0985); 0 clears it. Not the geometry: the revision stays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub design_speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A km asked for: its value and, where the km falls on more than one place, its region (0 before the first equation).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KmAsk {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: Option<u32>,
}

/// Input of `road.alignments.query` v1: every alignment's report, or one's (`alignment`: its id or name) with the
/// places at its stations (`stations`, its own horizontal length from the start), at kms (`kms`), every `every` metres
/// of km (`every`: the round kms, as a plan's km ticks), and points' stations and offsets (`points`).
///
/// Refusals: `not_a_road_project`, `unknown_alignment`, `invalid_places` (a value not finite, `every` not above 0, more
/// than 100 000 places; or a question without `alignment`), `station_outside` (a station off the alignment, a km on no
/// place), `station_ambiguous` (a km on more than one place without `region`), and the geometry's when the alignment
/// no longer builds.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alignment: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: Vec<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<KmAsk>>", optional))]
    pub kms: Vec<KmAsk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub every: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub points: Vec<[f64; 2]>,
}

/// An element of an alignment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentElementRow {
    /// From its PI: `p3/aliyman`, `p3/giris`, `p3/kurp`, `p3/cikis`.
    pub id: String,
    /// `tangent`, `arc` or `spiral`.
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    /// The km at its start.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub start: [f64; 2],
    /// Its start direction, radians from east counter-clockwise.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub direction: f64,
    /// Curvatures at its start and end, 1/m, left turns positive.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1/m")))]
    pub k0: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1/m")))]
    pub k1: f64,
    /// A curve's radius.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: Option<f64>,
    /// A clothoid's parameter A.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub a: Option<f64>,
}

/// A curve on a PI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentCurveRow {
    pub pi: String,
    /// The deflection, left positive.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub delta: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_in: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_out: f64,
    /// Tangent lengths, PI to TS and PI to ST.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub t_in: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub t_out: f64,
    /// The circular part's length.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub arc: f64,
    /// The stations of TS, SC, CS, ST.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: [f64; 4],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub centre: [f64; 2],
}

/// Two curves with no tangent between them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentJoinRow {
    pub between: [String; 2],
    /// A reverse curve (S) when true, a compound curve when false.
    pub reverse: bool,
    /// The ove clothoid between them when the first PI asks for one (docs/adr/0979).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ove: Option<AlignmentOveRow>,
}

/// An ove clothoid (docs/adr/0979): its parameter A solved on the PIs, its length A²·|1/R2 − 1/R1|, the station it
/// starts at and the least gap between the two circles (an ove exists only while it is above 0).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentOveRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub a: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub gap: f64,
}

/// An alignment as it builds now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentReport {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    /// A parallel axis's source and offset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub offset_of: Option<AlignmentOffset>,
    /// The horizontal length on the project's plane (grid), m; a parallel axis's own.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    /// The length on the ground (the grid length over each piece's scale and height factor), m; absent without a
    /// system that has one scale at a point or without the project's mean ellipsoidal height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub ground_length: Option<f64>,
    /// How the ground length was had, or why it was not.
    pub ground: String,
    /// The km at the start and at the end.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km_start: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km_end: f64,
    pub elements: Vec<AlignmentElementRow>,
    pub curves: Vec<AlignmentCurveRow>,
    pub joins: Vec<AlignmentJoinRow>,
    /// Where the integral meets the PIs' geometry, the largest gap: place, m.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub residual: f64,
    /// … and direction, rad.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub residual_direction: f64,
    /// Every join of two elements and how far they part there in place, direction and curvature (docs/adr/0967).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AlignmentSeam>>", optional))]
    pub seams: Vec<AlignmentSeam>,
    /// `kentos.alignment/1`.
    pub algorithm: String,
}

/// Where two elements meet (docs/adr/0967): the later one's start and the gaps, G0 place (m), G1 direction (rad) and
/// G2 curvature (1/m). A circular curve without a clothoid meets its tangent with a curvature jump of 1/R.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentSeam {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub before: String,
    pub after: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub place: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub direction: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1/m")))]
    pub curvature: f64,
}

/// A place on an alignment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentPlace {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    /// The km written `2+345.678`.
    pub km_text: String,
    /// The km region (0 before the first equation).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub region: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub direction: f64,
    /// 1/m, left turns positive.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1/m")))]
    pub curvature: f64,
    pub element: String,
}

/// A point's station and offset: the nearest foot, every foot by distance, or the end it lies beyond.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointStation {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub point: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub nearest: Option<AlignmentFoot>,
    pub feet: Vec<AlignmentFoot>,
    /// `start` or `end` when no foot falls on the alignment: the point lies beyond that end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub beyond: Option<String>,
}

/// A foot of a point.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentFoot {
    pub at: AlignmentPlace,
    /// Right positive.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
    /// At a kink between two elements.
    pub corner: bool,
}

/// The places of a km.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KmPlaces {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    pub places: Vec<AlignmentPlace>,
}

/// A row of what an edit did to the stations (docs/adr/0901 §9).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StationMapRow {
    pub element: String,
    /// `kept`, `moved`, `changed`, `added`, `removed`, or `reversed` (the alignment turned round: `before` is where the
    /// stretch was, read from the old start).
    pub change: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub before: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub after: Option<[f64; 2]>,
}

/// An alignment in the project's list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentListed {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub offset_of: Option<String>,
}

/// Output of `road.alignments.create` and `.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsWritten {
    /// The alignment written; absent when removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alignment: Option<AlignmentReport>,
    /// What the edit did to the stations, by element.
    pub mapping: Vec<StationMapRow>,
    /// The parallel axes that follow it.
    pub dependents: Vec<String>,
    /// The superelevation schedules turned round with it in the same step (docs/adr/0964): their stations, sides and
    /// pivot read from the new start.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub superelevations: Vec<String>,
    /// The profiles turned round with it in the same step (docs/adr/0965): their PVIs and crossings read from the new
    /// start.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub profiles: Vec<String>,
    pub alignments: Vec<AlignmentListed>,
    pub revision: String,
}

/// Output of `road.alignments.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsQueried {
    /// Every alignment's report, or the one asked's.
    pub alignments: Vec<AlignmentReport>,
    pub at: Vec<AlignmentPlace>,
    pub kms: Vec<KmPlaces>,
    pub ticks: Vec<AlignmentPlace>,
    pub points: Vec<PointStation>,
}

pub const ROAD_ALIGNMENTS_LABEL: &str = "road.alignments.label";
pub const ROAD_ALIGNMENTS_LABEL_VERSION: u32 = 1;

/// Input of `road.alignments.label` v1 (plan/road.md Y3): an alignment's km written on a layer as objects, so it goes
/// to sheets and DXF: a tick across it at every round `every` metres of km (20 when absent), a longer one with its
/// km written at every `major` (100; a multiple of `every`), the start's and the end's km, and the curves' TS, SC, CS,
/// ST (TC, CT without clothoids) with their km (`curves`, true when absent). Texts are `height` paper millimetres high
/// at the project's drawing scale (2.5 when absent).
///
/// Refusals, in this order: `not_a_road_project`, `unknown_alignment`, `invalid_label` (`every` or `height` not above
/// 0, `major` not a whole multiple of `every`, more than 100 000 ticks), the layer's (`layer_not_found`,
/// `not_a_layer`, `service_layer`, `layer_locked`), `invalid_revision`, `revision_conflict`, and the geometry's when
/// the alignment no longer builds. A hidden layer is the warning `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsLabel {
    pub alignment: String,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub every: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub major: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub curves: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A written label: where it is, its km, what it says (none for a minor tick), and its kind (`tick`, `major`, `start`,
/// `end`, or a curve point's name).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentLabelRow {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub km: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
}

/// Output of `road.alignments.label` v1, and its plan: the labels in their order along the alignment, the objects
/// written (none in a plan).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsLabelled {
    pub alignment: String,
    pub labels: Vec<AlignmentLabelRow>,
    pub uids: Vec<String>,
    pub revision: String,
}

pub const ROAD_ALIGNMENTS_IMPACT: &str = "road.alignments.impact";
pub const ROAD_ALIGNMENTS_IMPACT_VERSION: u32 = 1;

/// `road.alignments.impact` (plan/road.md Y57, docs/adr/0963; ROAD-ALIGN-10): what an alignment's new PIs would do to
/// what hangs on its stations, before they are written. The old stretches whose places leave the new line by more than
/// `tolerance` (m; absent: 1 mm), the new stretches off the old line, the runs between and how far their stations
/// shift; every station a profile, a section set, a corridor or a superelevation schedule names, mapped to the new
/// line or told unresolved; the parallel axes that follow; the volumes and haul plans on the corridors it touches, an
/// approval among them told. Reads only.
///
/// Refusals: `not_a_road_project`, `unknown_alignment`, `invalid_alignment` (PIs that make no line or a parallel
/// axis), `invalid_alignment_impact` (a tolerance not above 0 or above 1 m).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentsImpact {
    /// The alignment's id or name.
    pub alignment: String,
    /// The PIs it would have; none with `reverse`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AlignmentPi>>", optional))]
    pub pis: Vec<AlignmentPi>,
    /// The alignment turned round instead (`road.alignments.edit reverse`, docs/adr/0965): every station s goes to
    /// L − s; the profiles and superelevation schedules turn with it, the rest is told.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reverse: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
}

/// A stretch of one line off the other: its stations on that line and their km, the greatest offset where every place
/// of it has a foot on the other line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentStretch {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub most: Option<f64>,
}

/// A stretch of old stations still on the line, and how far their stations move at its ends (new − old).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentRun {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    pub from_text: String,
    pub to_text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub shift_from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub shift_to: f64,
}

/// What names a station of the alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StationOwner {
    Profile,
    SectionSet,
    Corridor,
    Superelevation,
    /// A volume on a corridor of the alignment: its factor rows and topsoil stretches (docs/adr/0965).
    Volume,
    /// A haul diagram on such a volume: its sites, unsuitable stretches and barriers.
    Haul,
    Culvert,
    Wall,
    /// A sheet set on the alignment.
    SheetSet,
}

/// A station something names, where it lands: `to` on the new line, none where its place left the line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StationRef {
    pub owner: StationOwner,
    /// The owner's id and name.
    pub id: String,
    pub name: String,
    /// Which station of it: a PVI's id, `from`, `to`, `region 2 from`, `station 3`, `row 1`, `curve PI2`.
    pub item: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: Option<f64>,
    /// The edit writes it there itself (a reversal turns the profiles and schedules round, docs/adr/0965); otherwise
    /// it stays where it is named and goes stale.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub carried: bool,
}

/// An alignment that follows this one (a parallel axis).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentFollower {
    pub id: String,
    pub name: String,
}

/// What an alignment's new PIs would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentImpact {
    pub alignment: String,
    /// The elements matched by id (as `road.alignments.edit` writes it).
    pub mapping: Vec<StationMapRow>,
    /// Old stretches whose places leave the new line (old stations).
    pub removed: Vec<AlignmentStretch>,
    /// New stretches off the old line (new stations).
    pub added: Vec<AlignmentStretch>,
    pub runs: Vec<AlignmentRun>,
    /// The stations named that move or are left unresolved.
    pub refs: Vec<StationRef>,
    pub parallels: Vec<AlignmentFollower>,
    pub volumes: Vec<crate::ImpactDerived>,
    pub hauls: Vec<crate::ImpactDerived>,
    /// The line and every station named stay as they are.
    pub clear: bool,
}
