//! The array command of the product command catalog (docs/adr/0047): copies
//! of objects named by their persistent ids laid out in rows and columns, or
//! around a centre, as one undo step. The Dizi and Kutupsal dizi tools write
//! through `cad.entities.array` on the web (`apps/web/src/product`) and on
//! the desktop (`crates/native/application`); both pass the shared cases in
//! `fixtures/commands/v1`.
//!
//! The array is typed by what the tools ask for (rows, columns and their
//! spacing; a centre, a count, an angle to fill and whether the copies
//! turn), not by a list of matrices: the shared geometry core lays the
//! copies out from it on both platforms (`array_transforms`) and moves every
//! kind of object with them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};

/// Copies objects into a rectangular or a polar array.
pub const CAD_ENTITIES_ARRAY: &str = "cad.entities.array";
pub const CAD_ENTITIES_ARRAY_VERSION: u32 = 1;

/// How the copies are laid out. The originals take the first place and stay
/// where they are. Coordinates are x east (Y), y north (X), in the
/// project's units (m), float64.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArrayLayout {
    /// Dizi: `rows` × `cols` places, 2 to 10 000 of them; the copy `j`
    /// columns and `i` rows from the originals moves `j·dx` east and `i·dy`
    /// north. The copies are made row after row, each row column after
    /// column. A direction with more than one place needs a spacing.
    Grid {
        #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 10000)))]
        rows: u32,
        #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 10000)))]
        cols: u32,
        dx: f64,
        dy: f64,
    },
    /// Kutupsal dizi: `count` items around `center`, 2 to 1000, the
    /// originals among them, over `fill` degrees (counter-clockwise; minus
    /// clockwise; 360 a full turn). A full turn shares the circle out; a
    /// partial fill puts the last copy on its end. With `rotate` each copy
    /// turns about the centre; without, it keeps its direction and moves as
    /// the middle of the copied objects' box goes round.
    Polar {
        center: Vec2,
        #[cfg_attr(feature = "schema", schemars(range(min = 2, max = 1000)))]
        count: u32,
        fill: f64,
        rotate: bool,
    },
}

/// Input of `cad.entities.array` v1: copies of objects named by their
/// persistent ids (docs/adr/0014) laid out by `layout`, as one undo step.
/// The array tools make the selection explicit here (TODOS.md CMD-07): they
/// give the selected objects' ids; the command reads no selection, layer or
/// view.
///
/// Each copy takes every field of its original (layer, colour, attributes,
/// label, symbol) and a new persistent id; the originals stay. The copies
/// are written place after place, each place the objects in the input's
/// order. The undo step is the tool's name: “Dizi” or “Kutupsal dizi”.
///
/// Objects on a locked layer (by itself or a group above it) are not
/// copied: with others they are named in the output's `locked` with a
/// `layer_locked` warning; when every one is locked nothing is written and
/// the answer is `layer_locked`. A polar array's copies that do not turn are
/// placed by the middle of the box of the objects that are copied. A
/// repeated id counts once.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`,
/// `invalid_uid` (each id in order), `not_finite` (the layout's numbers, in
/// their order), `invalid_count` (rows and columns, or the count, out of
/// their range), `invalid_spacing` (a grid direction with more than one
/// place and no spacing), `invalid_fill` (a polar fill of zero or past a
/// full turn), `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found` (each id in order), `layer_locked`, then `not_finite`
/// again (path `layout`) when a copy would lie past the largest float64; on
/// the desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesArray {
    /// The objects' persistent ids (lowercase UUID text with hyphens), at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// Where the copies go.
    pub layout: ArrayLayout,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.array` v1: what was made and what stayed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesArrayed {
    /// The copies' persistent ids, place after place, each place in the order
    /// of their originals in the input.
    pub created: Vec<String>,
    /// The ids not copied because their layer is locked, in the input's order.
    pub locked: Vec<String>,
    /// The document's revision after the write, as decimal text. Inside an
    /// open transaction or group the write joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.array` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesArrayPlan {
    /// The ids of the objects execute would copy, in the input's order.
    pub sources: Vec<String>,
    /// The copies as execute would write them, in the same order as
    /// `created`: `id` 0, as their slots are given when they are written.
    pub entities: Vec<Entity>,
    /// The ids it would not copy because their layer is locked.
    pub locked: Vec<String>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
