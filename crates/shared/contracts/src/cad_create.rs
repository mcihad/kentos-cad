//! The create command of the product command catalog (docs/adr/0057): new
//! objects of any kind on a named layer, as one undo step. The drawing tools
//! whose objects have no command of their own write through
//! `cad.entities.create` on the web (`apps/web/src/product`) and on the
//! desktop (`crates/native/application`): Elips, Eğri, Yardımcı çizgi, Işın,
//! Halka, Paralel çizgi, Dik in, Dik çık and Böl. Both pass the shared cases
//! in `fixtures/commands/v1`.
//!
//! The geometry is given, not computed here: the tools compute it with the
//! shared geometry core from what was clicked and typed (an ellipse from its
//! axis, the sides of a parallel line, the points along an object), and the
//! command writes what the preview showed. A geometry is typed as
//! `cad.entities.edit` types it ([`EntityGeometry`]) and checked by the same
//! rules, so a geometry one command takes the other takes too.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::cad_edit::EntityGeometry;
use crate::entity::Entity;

/// Writes new objects on a named layer in one undo step.
pub const CAD_ENTITIES_CREATE: &str = "cad.entities.create";
pub const CAD_ENTITIES_CREATE_VERSION: u32 = 1;

/// The drawing tool whose step has its own name; without one the step is
/// “Ekle”, as for every object a drawing tool adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CreateOperation {
    /// Paralel çizgi: the lines beside an axis, the corridor between them, the axis.
    Parallel,
    /// Dik in: a perpendicular from a point down to a reference line.
    PerpendicularIn,
    /// Dik çık: a perpendicular up from a point of a reference line.
    PerpendicularOut,
    /// Böl: points along an object.
    Divide,
}

/// One new object: its geometry and what else it carries. The layer is the
/// input's; the persistent id and the slot are given when it is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewObject {
    /// Its kind and the fields that place and shape it.
    pub geometry: EntityGeometry,
    /// Colour override (`EntityBase.color`). Absent: the layer's colour (katmana göre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// GIS attributes, text in v1. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    /// The text shown beside it (`EntityBase.label`). Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
}

/// Input of `cad.entities.create` v1: new objects on a named layer, written
/// as one undo step. Everything the command depends on is here (TODOS.md
/// CMD-07): the tool fills `layerId` from the active layer and each object's
/// `color` from the current colour; the command reads neither.
///
/// The objects are written in their order, each with a new persistent id.
/// The undo step is named after `operation`, or “Ekle”.
///
/// Refusals (`CommandError.code`), checked in this order: `no_objects`, then
/// each object's geometry in order: `too_few_points` (a polyline),
/// `too_few_corners` (a closed area's or a hatch's ring or hole),
/// `not_finite`, `invalid_radius`; then `invalid_revision`,
/// `revision_conflict` (status `conflict`), `layer_not_found`,
/// `not_a_layer`, `layer_locked`; on the desktop also `slots_exhausted`.
/// Warning: `layer_hidden` (they are written all the same).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCreate {
    /// The layer they go on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// What is written, at least one, in this order.
    pub objects: Vec<NewObject>,
    /// The drawing tool the objects come from, when its step has its own
    /// name: Paralel çizgi, Dik in, Dik çık, Böl. Absent: “Ekle”.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operation: Option<CreateOperation>,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.create` v1: the objects written, in the input's order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCreated {
    /// Their persistent ids (UUIDv7, docs/adr/0014): the names files, the
    /// cloud, Python and AI use.
    pub created: Vec<String>,
    /// Their slots in the open document (`Entity.id`); they mean nothing
    /// once the document is closed.
    pub ids: Vec<u32>,
    /// The document's revision after the write, as decimal text. Inside an
    /// open transaction or group the write joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCreatePlan {
    /// The objects as they would be stored, in the input's order: `id` 0,
    /// as their slots are given when they are written.
    pub entities: Vec<Entity>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
