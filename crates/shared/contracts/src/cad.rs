//! Drawing commands of the product command catalog (docs/adr/0013, 0022):
//! their names, versions and typed input and output. The web's handlers are
//! TypeScript (`apps/web/src/product`), the desktop's Rust
//! (`crates/native/application`); both pass the shared cases in
//! `fixtures/commands/v1`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{Entity, RingGeometry, Vec2};

/// Writes one closed area (kapalı alan) on a named layer.
pub const CAD_POLYGON_CREATE: &str = "cad.polygon.create";
pub const CAD_POLYGON_CREATE_VERSION: u32 = 1;

/// Input of `cad.polygon.create` v1: one closed area on a named layer, written
/// as one undo step. Everything the command depends on is here (TODOS.md
/// CMD-07): the closed-area tool fills `layerId` from the active layer and
/// `color` from the current colour; the command reads neither.
///
/// Refusals (`CommandError.code`), checked in this order: `too_few_corners`,
/// `not_finite`, `bulge_count` (the outer ring, then each hole),
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer`, `layer_locked`; on the desktop also
/// `slots_exhausted`. Warning: `layer_hidden` (it is written all the same).
/// Geometric validity (self-intersection, zero area, a hole outside the
/// ring) is not checked, as the drawing tools and the file reader do not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PolygonCreate {
    /// The layer it goes on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// Corners of the outer ring in order: x east (Y), y north (X), in the
    /// project's units (m), float64. At least 3; the first is not repeated.
    pub pts: Vec<Vec2>,
    /// One bulge per edge, the closing edge (last corner → first) last:
    /// tan(θ/4) of the edge's included angle, positive counter-clockwise, 0
    /// straight. Absent: every edge is straight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bulges: Option<Vec<f64>>,
    /// Holes: closed rings of at least 3 corners, each with one bulge per edge when it has bulges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub holes: Option<Vec<RingGeometry>>,
    /// Colour override (`EntityBase.color`). Absent: the layer's colour (katmana göre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// GIS attributes, text in v1. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A document revision as text: a whole number in decimal, no sign, no
/// leading zero (DOM-12: revisions cross JSON as text, never as a float).
/// Only equality means something; the step between two revisions does not.
#[cfg(feature = "schema")]
const REVISION_TEXT: &str = "^(0|[1-9][0-9]*)$";

/// Output of `cad.polygon.create` v1: the object written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PolygonCreated {
    /// Its persistent id (UUIDv7, docs/adr/0014): the name files, the cloud, Python and AI use.
    pub uid: String,
    /// Its slot in the open document (`Entity.id`); it means nothing once the document is closed.
    pub id: u32,
    /// The document's revision after the write, as decimal text. Inside an
    /// open transaction or group the write joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.polygon.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PolygonPlan {
    /// The polygon as it would be stored. Its `id` is 0: the slot is given when it is written.
    pub entity: Entity,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
