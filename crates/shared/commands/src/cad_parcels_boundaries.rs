//! `cad.parcels.boundaries` v1 (plan/map-cad.md MAP-08, docs/adr/2000): Sınır grafiği, read only. The parcels'
//! boundaries as a graph, so that a shared boundary is a boundary of two parcels and not two polygons' coordinates
//! that happen to meet: every corner (a place where a parcel ring has a vertex), every boundary (a ring edge, the same
//! edge of two parcels once, its parcel on the left and on the right), each parcel's boundaries in ring order and its
//! neighbours, and what keeps the geometry from being a clean graph:
//!
//! - `missingVertex`: a parcel's corner lies on the inside of another parcel's edge (within 1 µm), which has no corner
//!   there: the boundary is shared only in part. On a straight edge the foot of its perpendicular is between the ends;
//!   on an arc it is near the arc's circle and on the arc's side of the chord (docs/adr/2220);
//! - `splitCorner`: two corners closer than 1 µm that are not the same place: coordinates that meet by chance;
//! - `overlap`: one edge on three or more parcels;
//! - `arcChord`: two boundaries between the same two corners, one an arc and the other its chord, or two different arcs
//!   (docs/adr/2620): the parcels disagree on the boundary's shape, a sliver lies between them (a gap or an overlap);
//!   its area and how far apart the two are at their middles.
//!
//! The ids are the graph's, from the geometry (`K1`… by east then north, `S1`… by their corners'): the same geometry
//! gives the same ids; they are no lasting record. The geometric areas never touch a recorded one. The shared cases
//! are `fixtures/commands/v1/cad.parcels.boundaries.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_PARCELS_BOUNDARIES: &str = "cad.parcels.boundaries";
pub const CAD_PARCELS_BOUNDARIES_VERSION: u32 = 1;

/// Input of `cad.parcels.boundaries` v1: the parcels (none: every area on the parcel layer `parsel`).
///
/// Refusals: `invalid_uid` and `entity_not_found` (`uids[i]`), `not_a_parcel` (`uids[i]`: not an area).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsBoundaries {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
}

/// A corner: its id, place and the parcels with a vertex there (in the drawing's order).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelCorner {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    pub parcels: Vec<String>,
}

/// A boundary: its id, its corners (from the lower id to the higher), its bulge that way (0: straight), its length,
/// and the parcel on its left and on its right going from `from` to `to` (none: no parcel on that side).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelBoundary {
    pub id: String,
    pub from: String,
    pub to: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulge: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub left: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub right: Option<String>,
}

/// A parcel in the graph: its id, its identity as written (İl/İlçe/Mahalle/Ada/Parsel, the empty ones left out), its
/// geometric area, its boundaries in ring order (the outer ring's, then its holes' and parts'), its neighbours.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelInGraph {
    pub uid: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m2")))]
    pub area: f64,
    pub boundaries: Vec<String>,
    pub neighbours: Vec<String>,
}

/// What keeps the geometry from a clean graph: `missingVertex`, `splitCorner`, `overlap` or `arcChord`; where; the
/// parcels.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BoundaryProblem {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    pub parcels: Vec<String>,
    /// `arcChord` (docs/adr/2620): the two boundaries between the same corners, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub boundaries: Vec<String>,
    /// `arcChord`: the area between the two curves, the sliver one parcel lacks or both claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m2")))]
    pub area: Option<f64>,
    /// `arcChord`: how far apart the two curves are at their middles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub gap: Option<f64>,
}

/// Output of `cad.parcels.boundaries` v1: the corners, the boundaries, the parcels, the problems; how many boundaries
/// two parcels share and how many have a parcel on one side only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsBounded {
    pub corners: Vec<ParcelCorner>,
    pub boundaries: Vec<ParcelBoundary>,
    pub parcels: Vec<ParcelInGraph>,
    pub problems: Vec<BoundaryProblem>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub shared: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outer: u32,
}
