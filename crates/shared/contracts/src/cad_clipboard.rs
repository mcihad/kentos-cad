//! The clipboard's semantic commands of the product command catalog (docs/adr/0309; AutoCAD's COPYCLIP, COPYBASE,
//! PASTECLIP, PASTEORIG and PASTEBLOCK): a copy of objects with all they use, and that copy pasted into a drawing
//! (the same or another) with its layers, blocks and styles brought in by name, as one undo step.
//!
//! A copy is a [`Clip`]: the objects, the layer tree they are on, the block definitions they place, the text and
//! dimension styles and the library items they name, the source's unit and a base point. It is the copy the host's
//! clipboard keeps; Python and an agent pass it on as it came.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{
    DimensionStyleDef, DrawingUnit, Entity, LayerNode, ProjectStyles, TextStyleDef, Vec2,
    entity::BlockDefinition,
};

/// Copies objects with all they use.
pub const CAD_ENTITIES_COPY: &str = "cad.entities.copy";
pub const CAD_ENTITIES_COPY_VERSION: u32 = 1;
/// Pastes a copy.
pub const CAD_ENTITIES_PASTE: &str = "cad.entities.paste";
pub const CAD_ENTITIES_PASTE_VERSION: u32 = 1;

/// A copy of objects with all they use (docs/adr/0309 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Clip {
    /// The objects, numbered 1, 2 … in the drawing's order (so their drawing order is kept), whatever order they
    /// were given in.
    pub objects: Vec<Entity>,
    /// Each object's persistent id where it was copied, in the order of `objects`: what links between them name.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The layer tree the objects are on: their layers and the groups above them, with the source's looks.
    pub layers: Vec<LayerNode>,
    /// The block definitions the objects place, and those these place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockDefinition>>", optional))]
    pub blocks: Vec<BlockDefinition>,
    /// The text styles the objects name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TextStyleDef>>", optional))]
    pub text_styles: Vec<TextStyleDef>,
    /// The dimension styles the objects name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DimensionStyleDef>>", optional))]
    pub dimension_styles: Vec<DimensionStyleDef>,
    /// The library items (symbols, their images) the objects and their layers draw with; absent when they draw
    /// with none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub library: Option<ProjectStyles>,
    /// The source drawing's unit (absent: metres). Lengths are metres in every drawing, so a paste keeps the
    /// objects' sizes in another unit (docs/adr/0309 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<DrawingUnit>,
    /// Where the objects are placed from: the copy's base point.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub base: Vec2,
}

/// Input of `cad.entities.copy` v1 (Panoya kopyala, Taban noktasıyla kopyala): the objects `uids` copied with all
/// they use; the drawing does not change. The base point is `base`, or the lower left corner of the objects' box.
/// Links between the copied objects stay in the copy; links to others drop.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`, `invalid_uid`, `not_finite` (the base),
/// `entity_not_found`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCopy {
    /// The objects' persistent ids (lowercase UUID text with hyphens), at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The base point; absent: the lower left corner of the objects' box.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub base: Option<Vec2>,
}

/// Input of `cad.entities.paste` v1 (Yapıştır, Özgün koordinatlara yapıştır, Blok olarak yapıştır): the copy
/// `clip` pasted as one undo step “Yapıştır”, its base point at `at` (absent: where it was copied).
///
/// - The layers: each object goes to the layer of this drawing at its layer's path (names compared case and
///   Turkish letters aside); a path this drawing lacks is made, with the source's look and its groups.
/// - The blocks: a definition of a name this drawing has is this drawing's (its own definition is used); the others
///   are added.
/// - The styles: a style of a name this drawing has is this drawing's; the others are added (a project setting).
/// - The objects take new persistent ids. Their links drop, as when a layer is taken from another drawing: a linked
///   label is pasted as a plain text, an associative hatch as a plain hatch, a schedule without its source.
/// - With `asBlock` the objects become one new block named after it (a free name: “Ad (2)” …), its base the
///   copy's base point, placed at `at` on the active layer.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities` (an empty copy), `invalid_clip` (its ids and
/// objects do not match, a layer or a block it names is not in it), `not_finite`, `empty_name` (`asBlock`
/// blank), `invalid_revision`, `revision_conflict` (status `conflict`), `layer_locked` (a layer an object would go
/// to). Warnings: `units_differ` (the source's unit is not this drawing's: sizes kept), `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesPaste {
    /// The copy, as `cad.entities.copy` gave it.
    pub clip: Clip,
    /// Where the copy's base point goes; absent: where it was copied (Özgün koordinatlara yapıştır).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// Paste as one block of this name (Blok olarak yapıştır).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub as_block: Option<String>,
    /// A copy's block or style of a name this drawing has (docs/adr/0351 §4): `keep` uses this drawing's, `rename`
    /// brings the copy's under a free name (“Ad (2)” …) and the pasted objects take it; absent: `keep`.
    #[serde(default, skip_serializing_if = "is_keep")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<PasteConflicts>"))]
    pub conflicts: PasteConflicts,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What a paste does with a copy's block or style of a name the drawing has (docs/adr/0351 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PasteConflicts {
    /// This drawing's is used.
    #[default]
    Keep,
    /// The copy's comes in under a free name.
    Rename,
}

fn is_keep(c: &PasteConflicts) -> bool {
    *c == PasteConflicts::Keep
}

/// A copy's block or style brought under a new name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PasteRenamed {
    /// `block`, `textStyle` or `dimensionStyle`.
    pub kind: String,
    pub from: String,
    pub to: String,
}

/// What a paste brought in besides its objects (docs/adr/0309 §3).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PasteReport {
    /// The layers and groups made, by their paths (“Kadastro / Bina”).
    pub layers: Vec<String>,
    /// The block definitions added, by name.
    pub blocks: Vec<String>,
    /// The copy's block names this drawing has: its own definitions are used.
    pub kept: Vec<String>,
    /// The text and dimension styles added, by name.
    pub styles: Vec<String>,
    /// The block the copy became (Blok olarak yapıştır): its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<String>,
    /// The copy's blocks and styles brought under new names (`conflicts: rename`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<PasteRenamed>>"))]
    pub renamed: Vec<PasteRenamed>,
}

/// Output of `cad.entities.paste` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesPasted {
    /// The new objects' persistent ids, in the copy's order (one insert with `asBlock`).
    pub created: Vec<String>,
    pub report: PasteReport,
    /// The document's revision after the write.
    pub revision: String,
}

/// Plan of `cad.entities.paste` v1: what it would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesPastePlan {
    /// The new objects as they would be written (their `id` 0).
    pub created: Vec<Entity>,
    pub report: PasteReport,
    /// The document's revision the plan was made at.
    pub revision: String,
}
