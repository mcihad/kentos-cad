//! The modify commands of the product command catalog (docs/adr/0037):
//! objects named by their persistent ids moved, rotated, scaled or mirrored,
//! in place or as copies. The move, copy, rotate, scale and mirror tools
//! write through `cad.entities.transform` on the web (`apps/web/src/product`)
//! and on the desktop (`crates/native/application`); both pass the shared
//! cases in `fixtures/commands/v1`.
//!
//! The transform is typed by what the tools ask for (a displacement, a
//! centre and an angle, a centre and a factor, the two points of an axis),
//! not a matrix: the shared geometry core builds the matrix from it on both
//! platforms and moves every kind of object with it (arcs stay counter-
//! clockwise, mirrored text stays readable).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};

/// Moves, rotates, scales or mirrors objects, in place or as copies.
pub const CAD_ENTITIES_TRANSFORM: &str = "cad.entities.transform";
pub const CAD_ENTITIES_TRANSFORM_VERSION: u32 = 1;

/// One similarity of the plane, given as the modify tools ask for it
/// (docs/adr/0037). Coordinates are x east (Y), y north (X), in the
/// project's units (m), float64.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Transform {
    /// Move (taşı) by a displacement: `dx` east, `dy` north.
    Move { dx: f64, dy: f64 },
    /// Rotate (döndür) about `center` by `angle` radians, counter-clockwise.
    Rotate { center: Vec2, angle: f64 },
    /// Scale (ölçekle) about `center` by `factor`, above zero.
    Scale { center: Vec2, factor: f64 },
    /// Mirror (aynala) across the line through `a` and `b`, two different points.
    Mirror { a: Vec2, b: Vec2 },
}

/// Input of `cad.entities.transform` v1: objects named by their persistent
/// ids (docs/adr/0014) moved by one transform as one undo step, in place or
/// as copies. The modify tools make the selection explicit here (TODOS.md
/// CMD-07): they give the selected objects' ids; the command reads no
/// selection, layer or view.
///
/// In place, each object keeps its slot, its persistent id and every other
/// field; only its geometry changes. As copies, each new object takes every
/// field of its original (layer, colour, attributes, label, symbol) and a new
/// persistent id; the originals stay. The undo step is the tool's name:
/// “Taşı” (a move), “Kopyala” (a move as copies), “Döndür”, “Ölçekle”,
/// “Aynala”.
///
/// Objects on a locked layer (by itself or a group above it) stay where they
/// are and are not copied: with others to transform they are named in the
/// output's `locked` with a `layer_locked` warning; when every one is locked
/// nothing is written and the answer is `layer_locked`. A repeated id counts
/// once.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`,
/// `invalid_uid` (each id in order), `not_finite` (the transform's numbers,
/// in their order), `invalid_factor` (a scale not above zero),
/// `invalid_axis` (a mirror axis without a direction), `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found` (each id in
/// order), `layer_locked`, then `not_finite` again (path `transform`) when
/// the transform would carry a coordinate past the largest float64; on the
/// desktop also `slots_exhausted` for copies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesTransform {
    /// The objects' persistent ids (lowercase UUID text with hyphens), at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// What happens to them.
    pub transform: Transform,
    /// True: copies are made and the originals stay (Kopyala, Kopya (K),
    /// Aynala keeping its source). Absent or false: the objects themselves change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copy: Option<bool>,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.transform` v1: what changed, what was made and
/// what stayed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesTransformed {
    /// In place: the ids of the objects transformed, in the input's order (a
    /// repeated one once). Empty for copies.
    pub changed: Vec<String>,
    /// As copies: the new objects' persistent ids, in the order of their
    /// originals in the input. Empty in place.
    pub created: Vec<String>,
    /// The ids left alone because their layer is locked, in the input's order.
    pub locked: Vec<String>,
    /// The document's revision after the write, as decimal text. Inside an
    /// open transaction or group the write joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.transform` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesTransformPlan {
    /// The ids of the objects execute would transform or copy, in the input's order.
    pub sources: Vec<String>,
    /// Each of them as execute would write it, in the same order: in place
    /// with its own slot (`id`); a copy with `id` 0, as its slot is given
    /// when it is written.
    pub entities: Vec<Entity>,
    /// The ids it would leave alone because their layer is locked.
    pub locked: Vec<String>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
