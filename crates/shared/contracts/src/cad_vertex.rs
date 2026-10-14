//! A path's vertices and edges as a product command (docs/adr/0279; plan/cad.md CAD-INPUT-15): Köşeyi sil, Ortasına
//! köşe ekle, Düz kenar yap and Yaya dönüştür on an object named by its persistent id and a place on it (docs/adr/0249),
//! the geometry the command's, as one undo step; with Topolojik düzenleme, the neighbours sharing what changes go
//! in the same step (docs/adr/0160). The grip menu writes through it; Python and an agent ask the same.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_stretch::StretchTopology;
use crate::subref::SubPlace;

/// Edits a path's vertex or edge.
pub const CAD_ENTITIES_VERTEX: &str = "cad.entities.vertex";
pub const CAD_ENTITIES_VERTEX_VERSION: u32 = 1;

/// What is done at the place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum VertexAction {
    /// Köşeyi sil (a `vertex` place): its two edges become one straight edge. “Köşe sil”.
    Remove,
    /// Ortasına köşe ekle (a `mid` place): a vertex at the edge's middle; or Köşe ekle at an `along` place: a vertex
    /// there, inside the edge. An arc edge is split in two on its circle; a line becomes a polyline of two edges.
    /// “Köşe ekle”.
    Insert,
    /// Düz kenar yap (a `mid` place): the edge straight. “Düz kenar yap”.
    Straighten,
    /// Yaya dönüştür (a `mid` place): a straight edge a gentle arc, its sagitta a quarter of the chord (bulge 0.5),
    /// bowing out of a ring or to the right of an open path. “Yaya dönüştür”.
    Arc,
}

/// Input of `cad.entities.vertex` v1. The place is on the object's own outer rings (a hole's vertices and edges are
/// not edited here); a multi-part area's other parts and an area's holes stay. An edge already as the action would
/// make it (straight, or an arc) does not change: nothing is written.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid`, `invalid_revision`, `revision_conflict`
/// (status `conflict`), `entity_not_found`, `layer_locked`, `not_a_path` (not a polyline or an area; a line takes
/// Ortasına köşe ekle only), `wrong_place` (Köşeyi sil's place not a vertex, Köşe ekle's not an edge's middle or a place along it, the others'
/// not an edge's middle),
/// `hole_edit` (a hole's place), `no_such_place` (no such part, vertex or edge, or a place along an edge at its vertex), `too_few_corners` (a ring left
/// with fewer than three vertices), `too_few_points` (an open path left with fewer than two).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesVertex {
    /// The object's persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub action: VertexAction,
    /// A `vertex` place for Köşeyi sil, a `mid` place (the edge from that vertex) for the others; Köşe ekle takes an
    /// `along` place too (a point inside the edge, docs/adr/0263 §1).
    pub place: SubPlace,
    /// Topolojik düzenleme: the visible, unlocked neighbours sharing the changed vertices and edges follow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub topology: Option<StretchTopology>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.vertex` v1: the objects written (the object, then its neighbours), their slots, the
/// revision after.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesVertexed {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub changed: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Plan of `cad.entities.vertex` v1: the objects as execute would write them, and the revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesVertexPlan {
    pub changed: Vec<crate::entity::Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
