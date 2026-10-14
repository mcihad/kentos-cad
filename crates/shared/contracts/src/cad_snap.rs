//! The object snap as a product command (docs/adr/0272; plan/cad.md CAD-INPUT-08): what a click at a point would
//! snap to, read with its object's persistent id, its place on the object (docs/adr/0249, 0263), its kind and the
//! drawing's revision, so that Python and an agent take the point a tool takes and bind a dimension to it
//! (`cad.dimensions.create`'s `place` points take the answer's `uid`, `place` and `path` as they are). It writes
//! nothing.
//!
//! Points and the aperture are metres on the wire, x east (Y), y north (X), whatever unit the project shows
//! (docs/adr/0165 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Vec2;
use crate::subref::SubPlace;

/// Finds the object snap at a point.
pub const CAD_SNAP_FIND: &str = "cad.snap.find";
pub const CAD_SNAP_FIND_VERSION: u32 = 1;

/// A snap kind a command can ask for: the drawing's own points (Uzantı, Paralel and Karelaj belong to a running
/// command's session and are not asked here).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SnapKindName {
    Endpoint,
    Midpoint,
    Center,
    Node,
    Quadrant,
    Intersection,
    Perpendicular,
    Tangent,
    Nearest,
    Centroid,
    Insertion,
}

/// Input of `cad.snap.find` v1.
///
/// Refusals (`CommandError.code`), in this order: `not_finite` (`at`, the aperture, `from`), `invalid_aperture`
/// (the aperture not above zero), `no_snap_kinds` (an empty `kinds`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SnapFind {
    /// Where the click is.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// How far from `at` a snap point may be, metres (a tool's aperture is its pixels at the view's scale).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub aperture: f64,
    /// The kinds that apply; absent: the settings' kinds on at first (all but Yakın and Ağırlık merkezi).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kinds: Option<Vec<SnapKindName>>,
    /// The running command's last point: Dik's foot and Teğet's point are measured from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Option<Vec2>,
    /// Every candidate best first (Tab's order, docs/adr/0271), not only the snap.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub all: bool,
}

/// A snap point as a command reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FoundSnap {
    pub kind: SnapKindName,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub point: Vec2,
    /// The object's persistent id (a crossing's: the first of its two objects in the drawing's order).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: Option<String>,
    /// Its place on the object (docs/adr/0249, 0263): absent for a crossing (on two objects) and a centroid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub place: Option<SubPlace>,
    /// When the object is an insert, the piece inside it the place is on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub path: Vec<u32>,
}

/// Output of `cad.snap.find` v1, and its plan: the snap (or with `all` every candidate, best first), none when
/// nothing is within the aperture; and the revision it was read at.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SnapFound {
    pub snaps: Vec<FoundSnap>,
    /// The document's revision the snaps were read at, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
