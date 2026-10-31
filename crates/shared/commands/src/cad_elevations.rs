//! The derived drawing commands (docs/adr/0799, ADV-06f): a room's interior elevations drawn from what the model holds
//! (the space's straight sides, the walls whose faces lie on them and their heights, the openings in those walls with
//! their sills and heads), on a layer of their own with a derivation record (`cad.elevations.edit`: derive, refresh,
//! release), and the derived drawings read with whether they still match the model (`cad.elevations.query`). A view
//! mark (docs/adr/0345) is a reference only: placing one never claims a drawing was made, and a derived drawing is not
//! a mark. What the model does not hold (a curved side, a column or a stair in front, a fixture's height) is not drawn
//! but listed (`leftOut`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{Entity, Vec2};

pub const CAD_ELEVATIONS_EDIT: &str = "cad.elevations.edit";
pub const CAD_ELEVATIONS_EDIT_VERSION: u32 = 1;
pub const CAD_ELEVATIONS_QUERY: &str = "cad.elevations.query";
pub const CAD_ELEVATIONS_QUERY_VERSION: u32 = 1;
/// The derivation record's tool (docs/adr/0633 §2): a layer whose record names it is a room's interior elevation.
pub const ELEVATION_TOOL: &str = "cad.elevations.edit";
/// The record's label and the undo step's name.
pub const ELEVATION_STEP: &str = "Oda iç görünüşü";
/// The group the elevations' layers go into.
pub const ELEVATION_GROUP: &str = "İç görünüşler";

/// What `cad.elevations.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ElevationsOperation {
    /// İç görünüş üret: the space `space`'s sides (`sides`, its outer rings' edges numbered from 0 in order, part by
    /// part; absent: all) drawn left to right from `at` (absent: 3 m right of the space's box, at its bottom), `gap`
    /// apart (absent: 1 m), on a new layer named `name` (absent: “101 Salon iç görünüşü”) in the group İç görünüşler.
    Derive {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        space: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        sides: Vec<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        gap: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
    },
    /// Yeniden üret: the elevation on the layer `layer` drawn again from the model as it is now, its objects kept by
    /// their roles (the same ids), those no longer drawn deleted, new ones added.
    Refresh { layer: String },
    /// Bağı kopar: the layer's derivation record removed; its objects stay as ordinary drawing objects.
    Release { layer: String },
}

/// Input of `cad.elevations.edit` v1.
///
/// Refusals: `invalid_input`, `not_finite`, `invalid_uid`, `entity_not_found`, `not_a_space`,
/// `elevation_space_not_net`, `elevation_no_sides`, `layer_not_found`, `elevation_not_found`,
/// `elevation_source_gone`, `layer_locked`, `nothing_to_set`, `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ElevationsEdit {
    pub operation: ElevationsOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A side of the room as its elevation draws it: its number and letter, its length and height (none: not known), where
/// its drawing begins, the walls whose faces it shows and the openings drawn on it, and the stretches along it no wall's
/// face covers (from its left end as one looks at it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ElevationSide {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub side: u32,
    pub letter: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub origin: Vec2,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub walls: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub openings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<[f64; 2]>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub open: Vec<[f64; 2]>,
}

/// Why something of the room is not drawn (or drawn only in part).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ElevationLeft {
    /// A curved side: an interior elevation of a curved wall is not unrolled.
    CurvedSide,
    /// A hole in the space (a shaft, a column's outline): what stands in it is not known.
    Hole,
    /// A column inside the room: it stands in front of the wall.
    Column,
    Stair,
    Ramp,
    /// A block (furniture, casework) inside the room: its height is not known.
    Fixture,
    /// A flue: inside the wall, not seen on its face.
    Flue,
    /// An opening not where its wall says (docs/adr/0341 §3).
    BrokenOpening,
    /// An opening without a head: its jambs drawn dashed, its top not.
    HeadUnknown,
}

/// Something of the room the elevation does not draw, or draws in part: the side it is on, if one, and the object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ElevationLeftOut {
    pub why: ElevationLeft,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub side: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
}

/// Output of `cad.elevations.edit` (and its plan): the elevation's layer and name, its sides, its objects after (a
/// plan's new ones without an id), how many were made, changed and deleted, what it left out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ElevationsEdited {
    pub layer: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ElevationSide>>", optional))]
    pub sides: Vec<ElevationSide>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Entity>>", optional))]
    pub drawn: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub made: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ElevationLeftOut>>", optional))]
    pub left_out: Vec<ElevationLeftOut>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.elevations.query` v1 (read-only): every derived elevation, the one on `layer`, or the one the object
/// `uid` is a member of.
///
/// Refusals: `layer_not_found`, `elevation_not_found`, `invalid_uid`, `entity_not_found`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ElevationsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: Option<String>,
}

/// Whether a derived elevation still is what the model would draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ElevationState {
    /// Güncel: drawn again it would be the same, and all its objects are there.
    Current,
    /// Eski: the model changed what it draws, or some of its objects were deleted.
    Stale,
    /// Kaynağı yok: its space is gone (or no longer a space); it cannot be drawn again.
    Gone,
}

/// A derived elevation as `cad.elevations.query` reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ElevationRow {
    pub layer: String,
    pub name: String,
    pub space: String,
    /// The space's number and name now (“101 Salon”); none when it is gone or has neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub space_label: Option<String>,
    /// The sides asked for; empty: all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub sides: Vec<u32>,
    pub state: ElevationState,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub members: u32,
    /// Its objects deleted from the drawing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub missing: Vec<String>,
    /// Its objects changed by hand (a refresh writes them back); said only while the model's drawing is unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub edited: Vec<String>,
    /// The drawing's revision it was derived at.
    pub derived_at: String,
}

/// Output of `cad.elevations.query`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ElevationsQueried {
    pub elevations: Vec<ElevationRow>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What a derivation record of an elevation keeps in its `parameters` (docs/adr/0799 §4): the space, the sides asked
/// for, where and how far apart they were drawn, each object by its role, and the digest of what was drawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevationRecord {
    pub space: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sides: Vec<u32>,
    pub at: Vec2,
    pub gap: f64,
    pub members: BTreeMap<String, String>,
    pub digest: String,
}
