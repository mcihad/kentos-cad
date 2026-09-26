//! The edit command of the product command catalog (docs/adr/0047): objects
//! named by their persistent ids given a new geometry, replaced by pieces,
//! followed by new objects made from them, or deleted, as one undo step
//! named after the modify tool. Ötele, Buda, Uzat, Köşe yuvarla, Pah, Kır,
//! Birleştir, Patlat, Uzat-kısalt and Köşe ekle/sil write through
//! `cad.entities.edit` on the web (`apps/web/src/product`) and on the
//! desktop (`crates/native/application`); both pass the shared cases in
//! `fixtures/commands/v1`.
//!
//! The geometry is given, not computed here: the tools compute it with the
//! shared geometry core, from what they picked and what the view shows (the
//! visible edges a trim cuts against, the corner under the cursor), and the
//! command writes what the preview showed. What every tool had to do alike
//! is the command's: which object keeps its place and persistent id, what a
//! piece inherits, the locked layer, one undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, RingGeometry, Vec2};

/// Reshapes, splits, joins and explodes objects in one undo step.
pub const CAD_ENTITIES_EDIT: &str = "cad.entities.edit";
pub const CAD_ENTITIES_EDIT_VERSION: u32 = 1;

/// The modify tool an edit comes from; it names the undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EditOperation {
    /// Ötele: a parallel copy.
    Offset,
    /// Buda: the part between two boundaries goes.
    Trim,
    /// Uzat: an end grows to a boundary.
    Extend,
    /// Köşe yuvarla: a corner rounded by an arc.
    Fillet,
    /// Pah: a corner cut by a line.
    Chamfer,
    /// Kır: the part between two points goes, or the object splits at one.
    Break,
    /// Birleştir: chains of objects meeting end to end become one each.
    Join,
    /// Patlat: an object comes apart into simple ones.
    Explode,
    /// Uzat-kısalt: a new total length, changed at one end.
    Lengthen,
    /// Köşe ekle: a vertex added on an edge.
    VertexAdd,
    /// Köşe sil: a vertex removed.
    VertexRemove,
    /// Esnet: the vertices in a window move, the rest stay.
    Stretch,
}

/// A drawing object's geometry alone: its kind and the fields that place and
/// shape it, without the fields every object has (layer, colour, attributes,
/// label, symbol). Coordinates are x east (Y), y north (X), in the project's
/// units (m), float64; angles in radians unless said otherwise. Dimensions
/// and hatches are not edited through this command in v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EntityGeometry {
    Point {
        p: Vec2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        z: Option<f64>,
    },
    Line {
        a: Vec2,
        b: Vec2,
    },
    /// An open path: vertices and DXF bulges (tan(θ/4), CCW positive), the
    /// one at vertex i bending the edge to vertex i + 1.
    Polyline {
        pts: Vec<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        bulges: Option<Vec<f64>>,
    },
    /// A closed area: its ring and, when it has any, its holes.
    Polygon {
        pts: Vec<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        bulges: Option<Vec<f64>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        holes: Option<Vec<RingGeometry>>,
    },
    Circle {
        c: Vec2,
        r: f64,
    },
    /// Counter-clockwise from `a0` to `a1`.
    Arc {
        c: Vec2,
        r: f64,
        a0: f64,
        a1: f64,
    },
    /// Centre, major axis vector, minor/major ratio, parameters `t0` → `t1`.
    Ellipse {
        c: Vec2,
        major: Vec2,
        ratio: f64,
        t0: f64,
        t1: f64,
    },
    Spline {
        pts: Vec<Vec2>,
        closed: bool,
    },
    /// A construction line through `p` along the unit direction `dir`.
    Xline {
        p: Vec2,
        dir: Vec2,
    },
    /// A ray from `p` along the unit direction `dir`.
    Ray {
        p: Vec2,
        dir: Vec2,
    },
    /// Text at `p`; `height` in metres, `rotation` in degrees counter-clockwise from east.
    Text {
        p: Vec2,
        text: String,
        height: f64,
        rotation: f64,
    },
}

/// One change of an edit. The objects are named by their persistent ids
/// (docs/adr/0014).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EntityEdit {
    /// The object takes a new geometry and keeps everything else: its slot,
    /// persistent id, layer, colour, attributes, label and symbol (an end
    /// extended, a corner rounded, a vertex added).
    Update {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        geometry: EntityGeometry,
    },
    /// The object becomes another in its place: the new geometry in its slot,
    /// with its persistent id, layer and colour; its attributes and label only
    /// with `keepData`; not its symbol (the part a trim leaves, the first part
    /// of a break, a chain joined into its first object).
    Replace {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        geometry: EntityGeometry,
        /// True: the attributes and the label carry over.
        #[serde(rename = "keepData", default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        keep_data: Option<bool>,
    },
    /// A new object made from `from`: its layer and colour; its attributes and
    /// label only with `keepData` (an offset copy, a piece of a break or an
    /// explode, a fillet's arc).
    Add {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        from: String,
        geometry: EntityGeometry,
        /// True: the attributes and the label carry over.
        #[serde(rename = "keepData", default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        keep_data: Option<bool>,
    },
    /// The object is deleted.
    Remove {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
    },
}

/// Input of `cad.entities.edit` v1: changes to objects named by their
/// persistent ids, written as one undo step named after `operation`. The
/// modify tools give the objects they picked and the geometry the shared
/// core computed (TODOS.md CMD-07); the command reads no selection, layer or
/// view.
///
/// The changes are made in their order, against the document as it was
/// before them: an `add` may come from an object the same edit replaces or
/// deletes. An object is changed (`update`, `replace`, `remove`) by one
/// change at most.
///
/// An object on a locked layer (by itself or a group above it) is not
/// changed, and nothing is made from it: when one is named, nothing is
/// written and the answer is `layer_locked`. The pieces of a split object
/// belong together, so an edit is written whole or not at all.
///
/// Refusals (`CommandError.code`), checked in this order: `no_changes`,
/// `invalid_uid` (each change's id in order), `not_finite`,
/// `too_few_points`, `too_few_corners`, `invalid_radius` (each geometry in
/// order), `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found` (each id in order), `repeated_entity` (an object
/// changed twice), `layer_locked`; on the desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesEdit {
    /// The modify tool the edit comes from; it names the undo step: Ötele,
    /// Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Patlat, Uzat-kısalt,
    /// Köşe ekle, Köşe sil, Esnet.
    pub operation: EditOperation,
    /// What changes, at least one.
    pub changes: Vec<EntityEdit>,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.edit` v1: what changed, what was made and what went.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesEdited {
    /// The objects updated or replaced in place, in the input's order.
    pub changed: Vec<String>,
    /// The new objects' persistent ids, in the order of their `add`s.
    pub created: Vec<String>,
    /// The objects deleted, in the input's order.
    pub removed: Vec<String>,
    /// The document's revision after the write, as decimal text. Inside an
    /// open transaction or group the write joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.edit` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesEditPlan {
    /// The objects updated or replaced, each as execute would write it, with
    /// its own slot (`id`), in the input's order.
    pub changed: Vec<Entity>,
    /// The new objects as execute would write them, `id` 0 (their slots are
    /// given when they are written), in the order of their `add`s.
    pub created: Vec<Entity>,
    /// The ids of the objects execute would delete, in the input's order.
    pub removed: Vec<String>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
