//! `map.features.code` v1 (plan/map-cad.md MAP-04, docs/adr/1360): Kodla çiz, the field-to-finish of a surveyor's
//! coded points through a data profile's classes. A point's code (its `Kod` field, else the field given) names a
//! class by one of the class's field codes (`ProfileClass.codes`); digits after the code number a string, the words
//! after it are controls: B begins a new string, K closes it, S ends it. A point class's points go on its layer; a line
//! class's strings become polylines and an area class's closed strings polygons on its layer, the points staying where
//! they are. The handler is `crates/native/application/src/map_features.rs`; the shared cases are
//! `fixtures/commands/v1/map.features.code.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const MAP_FEATURES_CODE: &str = "map.features.code";
pub const MAP_FEATURES_CODE_VERSION: u32 = 1;

/// The field a point's code is read from when none is given.
pub const DEFAULT_CODE_FIELD: &str = "Kod";

/// Input of `map.features.code` v1: the profile (`name`: KentOS's, or one beside the project's file `drawing`), the
/// field the codes are in, and the points (their lasting ids, in the order they were measured); none: every point with
/// a code on a shown, unlocked layer, in the drawing's order.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `unknown_profile`, `invalid_profile`,
/// `invalid_input` (an empty field name), `invalid_uid`, `entity_not_found`, `not_a_point`, `no_coded_points`,
/// `class_layer_missing` (a class a code names has no layer in the drawing: apply the profile first), `layer_locked`
/// (a point to move or a class's layer locked).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeaturesCode {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What one class got: its points put on its layer, its lines and areas made.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CodedClass {
    pub class: String,
    pub title: String,
    /// The class's layer, by id.
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moved: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub areas: u32,
}

/// A code no class of the profile has, and how many points wrote it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnknownCode {
    pub code: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
}

/// Output of `map.features.code` v1 (and its plan): each class a code named, in the profile's order; the objects made,
/// by their lasting ids (the plan's are empty); the codes no class has; and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeaturesCoded {
    pub classes: Vec<CodedClass>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub created: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<UnknownCode>>", optional))]
    pub unknown: Vec<UnknownCode>,
    pub revision: String,
}

pub const MAP_FEATURES_RECONCILE: &str = "map.features.reconcile";
pub const MAP_FEATURES_RECONCILE_VERSION: u32 = 1;

/// The attribute a matched candidate keeps (docs/adr/1760; named “Eşleştirme” since 8 November 2026): the asset it went
/// into, or why it was rejected. A candidate holding it, or the earlier name, is not read again: a reviewer's decision
/// is not undone by the next run.
pub const RECONCILED_FIELD: &str = "Eşleştirme";

/// The mark's earlier name, still read in drawings written before 8 November 2026.
pub const RECONCILED_FIELD_BEFORE: &str = "Uzlaştırma";

/// The longest a candidate source list may be.
pub const MAX_CANDIDATE_SOURCES: usize = 8;

/// A source of candidates (docs/adr/1760): a layer of the drawing and the name the answer and the assets give it
/// (LiDAR, Ortofoto, Saha …).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CandidateSource {
    pub layer: String,
    pub name: String,
}

/// A reviewer's decision on a candidate: its class corrected (`class`, a class of the profile), a single source's
/// candidate accepted (`accept`), rejected with a reason (`reject`), or the asset's missing fields given (`fields`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CandidateDecision {
    pub candidate: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub accept: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reject: Option<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<std::collections::BTreeMap<String, String>>"))]
    pub fields: std::collections::BTreeMap<String, String>,
}

/// Input of `map.features.reconcile` v1 (plan/map-cad.md MAP-04, docs/adr/1760): candidates of map assets from several
/// sources (a point cloud's and an orthophoto's extraction, a field survey) reconciled through a data profile's
/// classes. A candidate is a point, a polyline or an area (docs/adr/2140) on a source's layer whose `field` (`Kod` when
/// not given) names a class by its id or one of its codes; `confidence` names a number field (higher is better).
/// Candidates within `radius` (m; a polyline's distance is the larger of the two Hausdorff ways) are one group; two
/// areas are when their overlap ratio, the area of their intersection over that of their union, exceeds `overlap`
/// (0.5 when not given: the PASCAL VOC detection criterion). The decisions are a reviewer's (`reviewer` is written with
/// them). The asset's values, its candidates' and the decisions' alike, pass its class layer's field rules as any write
/// to that layer does (docs/adr/2540): a value that fits is written in its canonical form (a list's label as its code,
/// a decimal comma as a point), one that does not is said in the group's `invalid` and keeps it from being written
/// until a decision gives one that fits. Plan says what would be written; execute writes, in one undo step.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `unknown_profile`, `invalid_profile`,
/// `invalid_input` (no source, too many, one layer twice, an empty name or field, a radius not above 0, an overlap not
/// between 0 and 1, a decision on no candidate or naming no class of the profile, accept with reject),
/// `layer_not_found` (a source's layer),
/// `invalid_uid`, `entity_not_found`, `class_layer_missing` (a class to write has no layer: apply the profile first),
/// `layer_locked` (a class's layer to write on, or a source's whose candidates are marked).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeaturesReconcile {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    pub sources: Vec<CandidateSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub confidence: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    /// The overlap ratio (intersection over union, between 0 and 1) two area candidates' must exceed to be one group;
    /// 0.5 when not given (docs/adr/2140).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub overlap: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CandidateDecision>>", optional))]
    pub decisions: Vec<CandidateDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reviewer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A candidate as read: its object, its source's name, its class (after a correction) and what its field said, its
/// confidence, its group, and what it is (`member`: its source's representative in the group; `duplicate`: another of
/// its source in the group; `unknown`: its field names no class; `wrongKind`: a class of another kind than its object
/// (a point class on a polyline, an area class on a point …); `rejected`; `reviewed`: reconciled before, not read
/// again).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReconciledCandidate {
    pub uid: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub read: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub group: Option<u32>,
    pub status: String,
}

/// A group of candidates: its class (none when they disagree) and the classes they name, what it is (`agreed`: two
/// sources or more name one class; `single`: one source; `conflict`: the classes differ; `existing`: an accepted asset
/// of its class is there already), its sources, its candidates, its place (a point group's representatives' mean; a
/// line group's chosen polyline's first vertex; an area group's chosen footprint's centroid), how far its
/// representatives lie apart (m), an area group's smallest overlap ratio between its representatives, the asset (the
/// one there, or the one written), the required fields still missing, the values its class layer's field rules refuse
/// (docs/adr/2540), and whether execute writes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReconciledGroup {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub class: Option<String>,
    pub classes: Vec<String>,
    pub status: String,
    pub sources: Vec<String>,
    pub candidates: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spread: f64,
    /// An area group's smallest overlap ratio between two of its representatives (docs/adr/2140).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub overlap: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub missing: Vec<String>,
    /// The asset's values its class layer's field rules refuse (docs/adr/2540), in the layer's field order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<FieldTrouble>>"))]
    pub invalid: Vec<FieldTrouble>,
    pub writes: bool,
}

/// A value a field's rules refuse (docs/adr/2540): the field and why, as the field rules say it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldTrouble {
    pub field: String,
    pub message: String,
}

/// Output of `map.features.reconcile` v1 (and its plan): the groups in the order of their first candidate, every
/// candidate in the drawing's order, the assets written (the plan's: none), how many candidates were marked, and the
/// drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeaturesReconciled {
    pub groups: Vec<ReconciledGroup>,
    pub candidates: Vec<ReconciledCandidate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub written: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub marked: u32,
    pub revision: String,
}
