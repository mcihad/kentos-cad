//! Drawing commands of the product command catalog (docs/adr/0013, 0022,
//! 0027, 0029): their names, versions and typed input and output. The web's
//! handlers are TypeScript (`apps/web/src/product`), the desktop's Rust
//! (`crates/native/application`); both pass the shared cases in
//! `fixtures/commands/v1`. Each command has its own input, output and plan
//! types, even where two look alike: a command's version covers its own
//! schemas, and a shared type would tie the versions of every command using it.

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

/// Writes one straight line (çizgi) on a named layer.
pub const CAD_LINE_CREATE: &str = "cad.line.create";
pub const CAD_LINE_CREATE_VERSION: u32 = 1;

/// Input of `cad.line.create` v1: one straight line from `a` to `b` on a
/// named layer, written as one undo step. The line tool calls it once per
/// segment of its chain, so each segment stays its own object and its own
/// undo step (docs/adr/0027). Everything the command depends on is here
/// (TODOS.md CMD-07): the tool fills `layerId` from the active layer and
/// `color` from the current colour; the command reads neither.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite` (`a`,
/// then `b`, x before y), `invalid_revision`, `revision_conflict` (status
/// `conflict`), `layer_not_found`, `not_a_layer`, `layer_locked`; on the
/// desktop also `slots_exhausted`. Warning: `layer_hidden` (it is written
/// all the same). A line whose ends coincide is not refused: geometric
/// validity is not checked, as for `cad.polygon.create`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineCreate {
    /// The layer it goes on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// Where it starts: x east (Y), y north (X), in the project's units (m), float64.
    pub a: Vec2,
    /// Where it ends.
    pub b: Vec2,
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

/// Output of `cad.line.create` v1: the object written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineCreated {
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

/// What `cad.line.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LinePlan {
    /// The line as it would be stored. Its `id` is 0: the slot is given when it is written.
    pub entity: Entity,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Writes one open polyline (çoklu çizgi) on a named layer.
pub const CAD_POLYLINE_CREATE: &str = "cad.polyline.create";
pub const CAD_POLYLINE_CREATE_VERSION: u32 = 1;

/// Input of `cad.polyline.create` v1: one open polyline on a named layer,
/// written as one undo step. Everything the command depends on is here
/// (TODOS.md CMD-07): the polyline tool fills `layerId` from the active
/// layer and `color` from the current colour; the command reads neither.
///
/// Refusals (`CommandError.code`), checked in this order: `too_few_points`,
/// `not_finite` (points, x before y), `bulge_count`, `not_finite` (bulges),
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer`, `layer_locked`; on the desktop also
/// `slots_exhausted`. Warning: `layer_hidden` (it is written all the same).
/// Geometric validity (a path crossing itself, zero-length edges) is not
/// checked, as for `cad.polygon.create`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PolylineCreate {
    /// The layer it goes on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// Its points in order: x east (Y), y north (X), in the project's units
    /// (m), float64. At least 2.
    pub pts: Vec<Vec2>,
    /// One bulge per edge (`pts[i]` → `pts[i + 1]`), so one fewer than the
    /// points: tan(θ/4) of the edge's included angle, positive
    /// counter-clockwise, 0 straight. Absent: every edge is straight. The
    /// document keeps one value per point, as DXF does: the polyline is
    /// stored with a 0 after these, for the closing edge it does not have.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bulges: Option<Vec<f64>>,
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

/// Output of `cad.polyline.create` v1: the object written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PolylineCreated {
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

/// What `cad.polyline.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PolylinePlan {
    /// The polyline as it would be stored. Its `id` is 0: the slot is given when it is written.
    pub entity: Entity,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Deletes objects of the open drawing, named by their persistent ids.
pub const CAD_ENTITIES_DELETE: &str = "cad.entities.delete";
pub const CAD_ENTITIES_DELETE_VERSION: u32 = 1;

/// A persistent object id as text: lowercase, with hyphens (docs/adr/0014).
#[cfg(feature = "schema")]
const UID_TEXT: &str = "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$";

/// Input of `cad.entities.delete` v1: the objects to delete, named by their
/// persistent ids (docs/adr/0014), deleted as one undo step (“Sil”). What
/// the interface knows implicitly, the selection, is here (TODOS.md CMD-07):
/// the erase tool (Sil, Delete) fills `uids` from the selection, or with the
/// object it picks; the command reads no selection, so the same call deletes
/// the same objects whatever is selected.
///
/// Objects on a locked layer (by themselves or through a group above them)
/// stay, as the erase tool always left them (docs/adr/0029): with others to
/// delete they are named in the output's `locked` with a `layer_locked`
/// warning; when every one is locked nothing is deleted and the answer is
/// `layer_locked`. A repeated id counts once.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`,
/// `invalid_uid` (each id in order), `invalid_revision`, `revision_conflict`
/// (status `conflict`), `entity_not_found` (each id in order), `layer_locked`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesDelete {
    /// The objects' persistent ids (lowercase UUID text with hyphens), at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is deleted and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.delete` v1: what was deleted and what stayed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesDeleted {
    /// The ids deleted, in the input's order (a repeated one once). Undo
    /// brings them back with the same ids, in their places.
    pub removed: Vec<String>,
    /// The ids left in place because their layer is locked, in the input's order.
    pub locked: Vec<String>,
    /// The document's revision after the delete, as decimal text. Inside an
    /// open transaction or group the delete joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.delete` would delete (plan mode); nothing is deleted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesDeletePlan {
    /// The ids execute would delete, in the input's order.
    pub removed: Vec<String>,
    /// The ids it would leave in place because their layer is locked.
    pub locked: Vec<String>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to delete exactly these, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
