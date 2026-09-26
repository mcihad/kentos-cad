//! The point, circle and arc commands of the product command catalog
//! (docs/adr/0032): their names, versions and typed input and output. The
//! point, circle and arc tools write through them on the web
//! (`apps/web/src/product`) and on the desktop (`crates/native/application`);
//! both pass the shared cases in `fixtures/commands/v1`. As in `cad.rs`, each
//! command has its own input, output and plan types.
//!
//! The tools compute a circle or an arc from what was clicked and typed (three
//! points, two tangents and a radius, a start with a centre and an angle …)
//! through the shared geometry core; the command takes the result, the object
//! as the document stores it. A construction is the tool's, a stored object
//! the command's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::{Entity, Vec2};

/// Writes one point (nokta) on a named layer.
pub const CAD_POINT_CREATE: &str = "cad.point.create";
pub const CAD_POINT_CREATE_VERSION: u32 = 1;

/// Input of `cad.point.create` v1: one point object on a named layer, written
/// as one undo step. The point tool calls it once per point it places.
/// Everything the command depends on is here (TODOS.md CMD-07): the tool fills
/// `layerId` from the active layer (or its own layer, as the spot elevation
/// tool does) and `color` from the current colour; the command reads neither.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite` (`p`,
/// x before y, then `z`), `invalid_revision`, `revision_conflict` (status
/// `conflict`), `layer_not_found`, `not_a_layer`, `layer_locked`; on the
/// desktop also `slots_exhausted`. Warning: `layer_hidden` (it is written
/// all the same).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCreate {
    /// The layer it goes on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// Where it is: x east (Y), y north (X), in the project's units (m), float64.
    pub p: Vec2,
    /// Its elevation (kot), in metres. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<f64>,
    /// The text shown beside it (`EntityBase.label`): a point's name, a spot
    /// elevation. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
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

/// Output of `cad.point.create` v1: the object written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCreated {
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

/// What `cad.point.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointPlan {
    /// The point as it would be stored. Its `id` is 0: the slot is given when it is written.
    pub entity: Entity,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Writes one circle (daire) on a named layer.
pub const CAD_CIRCLE_CREATE: &str = "cad.circle.create";
pub const CAD_CIRCLE_CREATE_VERSION: u32 = 1;

/// Input of `cad.circle.create` v1: one circle by its centre and radius on a
/// named layer, written as one undo step. The circle tool computes the circle
/// of each of its methods (centre and radius or diameter, two points, three
/// points, tangent-tangent-radius, three tangents) through the shared core
/// and gives it here. Everything the command depends on is here (TODOS.md
/// CMD-07): the tool fills `layerId` from the active layer and `color` from
/// the current colour; the command reads neither.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite` (`c`,
/// x before y, then `r`), `invalid_radius` (not above zero),
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer`, `layer_locked`; on the desktop also
/// `slots_exhausted`. Warning: `layer_hidden` (it is written all the same).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CircleCreate {
    /// The layer it goes on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// Its centre: x east (Y), y north (X), in the project's units (m), float64.
    pub c: Vec2,
    /// Its radius, in the project's units; above zero.
    pub r: f64,
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

/// Output of `cad.circle.create` v1: the object written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CircleCreated {
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

/// What `cad.circle.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CirclePlan {
    /// The circle as it would be stored. Its `id` is 0: the slot is given when it is written.
    pub entity: Entity,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Writes one circular arc (yay) on a named layer.
pub const CAD_ARC_CREATE: &str = "cad.arc.create";
pub const CAD_ARC_CREATE_VERSION: u32 = 1;

/// Input of `cad.arc.create` v1: one circular arc on a named layer, as the
/// document stores it (centre, radius, and the angles it runs between counter-
/// clockwise), written as one undo step. The arc tool computes the arc of each
/// of its methods (three points; start, centre and end, angle or chord; start,
/// end and centre, angle, direction or radius; centre first; continuing the
/// last object) through the shared core and gives it here. Everything the
/// command depends on is here (TODOS.md CMD-07): the tool fills `layerId`
/// from the active layer and `color` from the current colour.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite` (`c`,
/// x before y, then `r`, `a0`, `a1`), `invalid_radius` (not above zero),
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer`, `layer_locked`; on the desktop also
/// `slots_exhausted`. Warning: `layer_hidden` (it is written all the same).
/// The angles are stored as given: equal angles are a full turn, as the
/// document reads them; no angle is refused for its size.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArcCreate {
    /// The layer it goes on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// Its centre: x east (Y), y north (X), in the project's units (m), float64.
    pub c: Vec2,
    /// Its radius, in the project's units; above zero.
    pub r: f64,
    /// Where it starts: radians, counter-clockwise from east (a geometry
    /// angle, not a surveying bearing).
    pub a0: f64,
    /// Where it ends: the arc runs counter-clockwise from `a0` to `a1`.
    pub a1: f64,
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

/// Output of `cad.arc.create` v1: the object written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArcCreated {
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

/// What `cad.arc.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArcPlan {
    /// The arc as it would be stored. Its `id` is 0: the slot is given when it is written.
    pub entity: Entity,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
