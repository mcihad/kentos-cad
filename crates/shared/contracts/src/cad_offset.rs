//! The semantic offset command of the product command catalog
//! (docs/adr/0246; plan/cad.md CAD-CMD-01, 02): parallel copies of objects
//! named by their persistent ids, at a distance or through a point, to the
//! side of a point, on one side or both, with the sources deleted or kept,
//! as one undo step named “Ötele”.
//!
//! Its input is what the user asked for, not the result: the copies are
//! computed by the shared geometry core (`ops::offset::offset_entity`, the
//! rule the Ötele tool has always used) inside the command, so the tool,
//! Python and an agent offsetting the same object by the same distance get
//! the same copy. `cad.entities.edit`'s `offset` operation, which writes a
//! geometry already computed, stays for compatibility.
//!
//! Distances and coordinates are metres on the wire (x east (Y), y north
//! (X)), whatever unit the project shows (docs/adr/0165 §2): a tool converts
//! what was typed before it gives it here.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};

/// Offsets objects by a distance or through a point.
pub const CAD_ENTITIES_OFFSET: &str = "cad.entities.offset";
pub const CAD_ENTITIES_OFFSET_VERSION: u32 = 1;

/// How far the copies go and to which side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OffsetBy {
    /// `distance` metres (above zero) from each object, on the side of
    /// `side`: the side of a line or a path is judged at its edge nearest
    /// the point; a circle, an arc or an ellipse grows when the point is
    /// outside it and shrinks when inside.
    Distance {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        side: Vec2,
    },
    /// Noktadan geç: each copy passes through `point`, its distance that of
    /// the point from its object's nearest edge (the side is the point's).
    Through {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        point: Vec2,
    },
}

/// Input of `cad.entities.offset` v1: parallel copies of the objects named
/// by their persistent ids (docs/adr/0014), as one undo step “Ötele”. The
/// Ötele tool gives the object it picked; the command reads no selection,
/// layer or view (TODOS.md CMD-07). A whole object is offset: the tool picks
/// objects, not their edges, so there is no sub-object reference in v1.
///
/// What is offset: a line, a polyline (with its arcs; a multi-part one part
/// by part), a closed area (a multi-part one with its parts joined), a
/// circle, an arc, an ellipse or an elliptical arc, a construction line or a
/// ray. Each copy is a new object from its source: its layer (or
/// `layerId`), its colour and its own line weight, not its attributes,
/// label, symbol or attachments; a vertex takes its source vertex's
/// elevation and measure. The copies are written in the input's order, each
/// source's copy on the side asked for first and, with `bothSides`, the one
/// on the other side next; then, with `deleteSource`, the sources that gave
/// a copy are deleted in the same step.
///
/// A copy that cannot be made (a circle shrunk past its centre, a path
/// whose arc would turn inside out, the side point on the object with
/// `bothSides`) is left out with an `offset_failed` warning at `uids[i]`;
/// when no copy can be made nothing is written and the answer is
/// `offset_failed`. A repeated id counts once.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`,
/// `invalid_uid` (each id in order), `not_finite` (the distance, then the
/// point's x and y), `invalid_distance` (a distance not above zero),
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found` (each id in order), `layer_locked` (an object on a
/// locked layer, by itself or a group above it, at `uids[i]`: nothing is
/// written), `not_offsettable` (an object of another kind, at `uids[i]`),
/// the target layer's `layer_not_found`, `not_a_layer`, `service_layer` and
/// `layer_locked` at `layerId`, then `offset_failed`; on the desktop also
/// `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesOffset {
    /// The objects' persistent ids (lowercase UUID text with hyphens), at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The distance and the side, or the point the copies pass through.
    pub by: OffsetBy,
    /// İki yana: a second copy of each object at the same distance on the other side.
    #[serde(rename = "bothSides", default)]
    pub both_sides: bool,
    /// Kaynağı sil: the objects offset are deleted in the same step.
    #[serde(rename = "deleteSource", default)]
    pub delete_source: bool,
    /// The layer the copies go on (`LayerNode.id`, a layer, not a group);
    /// without it each copy goes on its source's layer.
    #[serde(rename = "layerId", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.offset` v1: what was made and deleted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesOffsetMade {
    /// The copies' persistent ids, in the order they were written.
    pub created: Vec<String>,
    /// The sources deleted (`deleteSource`), in the input's order.
    pub removed: Vec<String>,
    /// Each source's distance in metres, in the input's order (a repeated id
    /// once): the one given, or with `through` the point's from the object.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distances: Vec<f64>,
    /// The document's revision after the write, as decimal text. Inside an
    /// open transaction or group the write joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.offset` would write (plan mode); nothing is written.
/// Preview and write are one computation: execute against the plan's
/// revision writes exactly `entities`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesOffsetPlan {
    /// The ids of the objects offset, in the input's order (a repeated id once).
    pub sources: Vec<String>,
    /// The copies as execute would write them, in the same order as
    /// `created`: `id` 0, as their slots are given when they are written.
    pub entities: Vec<Entity>,
    /// The sources execute would delete (`deleteSource`).
    pub removed: Vec<String>,
    /// Each source's distance in metres, as in the output.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distances: Vec<f64>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
