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
