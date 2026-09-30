//! The block commands of the product command catalog (docs/adr/0144 §4):
//! `cad.blocks.define` makes a definition from a drawing's objects,
//! `cad.blocks.edit` renames, redefines, moves the base point of, deletes and
//! purges definitions. Each is one undo step. Blok oluştur and the Bloklar
//! panel write through them on the web (`apps/web/src/product`) and on the
//! desktop (`crates/native/application`); both pass the shared cases in
//! `fixtures/commands/v1`. An insert is made with `cad.entities.create`
//! (geometry `insert`) and exploded with `cad.entities.edit` (Patlat).
//!
//! The block rules are `kentos_contracts::blocks`' (names once, Turkish case
//! folded; no cycles; at most 16 levels), said in the documents' words.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{BlockDefinition, Entity, Vec2};
use crate::identity::BlockId;

/// Makes a block definition from objects in one undo step.
pub const CAD_BLOCKS_DEFINE: &str = "cad.blocks.define";
pub const CAD_BLOCKS_DEFINE_VERSION: u32 = 1;

/// Renames, redefines, moves the base point of, deletes or purges definitions.
pub const CAD_BLOCKS_EDIT: &str = "cad.blocks.edit";
pub const CAD_BLOCKS_EDIT_VERSION: u32 = 1;

/// Input of `cad.blocks.define` v1: a new definition made of the objects
/// named by their persistent ids, copied as they are (their coordinates,
/// layers, colours and data; local ids 1, 2, … in the input's order), with
/// the base point given. Everything the command depends on is here (TODOS.md
/// CMD-07): Blok oluştur fills `layerId` from the active layer.
///
/// With `replace` the objects are deleted and an insert of the new block
/// takes their place at the base point (scale 1, no turn) on `layerId`, in
/// the same undo step, “Blok tanımla”.
///
/// An id given twice is one object.
///
/// Refusals (`CommandError.code`), checked in this order: `empty_name` (a
/// name empty or only white space), `no_entities`, `invalid_uid` (each id in
/// order), `not_finite` (the base point), `no_layer` (`replace` without
/// `layerId`); then `invalid_revision`, `revision_conflict` (status
/// `conflict`), `entity_not_found` (each id in order), `duplicate_block`
/// (the name taken, Turkish case folded), `block_too_deep`; with `replace`
/// `layer_not_found`, `not_a_layer`, `layer_locked` (the insert's layer),
/// then `layer_locked` for an object on a locked layer (it is taken into the
/// definition, but not deleted); on the desktop also `slots_exhausted`.
/// Warning: `layer_hidden` (the insert's layer).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksDefine {
    /// The block's name, unique in the drawing (Turkish case folded).
    pub name: String,
    /// The point an insert places, in the drawing's coordinates.
    pub base: Vec2,
    /// The objects it is made of, at least one, by persistent id.
    pub uids: Vec<String>,
    /// What the block is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    /// True: the objects are replaced by an insert of the new block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub replace: Option<bool>,
    /// With `replace`: the layer the insert goes on, a layer's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

/// Output of `cad.blocks.define` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlockDefined {
    /// The new definition's id.
    pub block: BlockId,
    /// With `replace`: the insert's persistent id and its slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub insert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<u32>,
    /// With `replace`: the objects deleted, in the input's order.
    pub removed: Vec<String>,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.blocks.define` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksDefinePlan {
    /// The definition as execute would write it; its id the nil UUID, as the
    /// id is given when it is written.
    pub block: BlockDefinition,
    /// With `replace`: the insert as execute would write it, `id` 0 and its
    /// `block` the nil UUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub insert: Option<Entity>,
    /// With `replace`: the objects execute would delete, in the input's order.
    pub removed: Vec<String>,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.blocks.edit` does; it names the undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BlockEditOperation {
    /// A new name (“Blok değiştir”).
    Rename,
    /// New objects from the drawing's (“Blok değiştir”): every insert shows them.
    Redefine,
    /// A new base point (“Blok değiştir”): every insert shifts by the difference.
    Rebase,
    /// A definition no insert uses deleted (“Blok sil”).
    Remove,
    /// Every definition no insert uses deleted (“Blokları temizle”).
    Purge,
    /// Its attribute definitions, the whole list (“Blok değiştir”): every
    /// insert shows them (docs/adr/0144 §7).
    Attributes,
}

/// Input of `cad.blocks.edit` v1: one change of the drawing's definitions, as
/// one undo step named after the operation. A definition is named by its id.
///
/// - `rename`: `block`, `name`;
/// - `redefine`: `block`, `uids` (the objects it is made of now, copied as
///   `cad.blocks.define` copies them), `base` (absent: kept), `replace` and
///   `layerId` as `cad.blocks.define` has them;
/// - `rebase`: `block`, `base`;
/// - `remove`: `block`;
/// - `purge`: nothing; a definition only unused ones use goes too;
/// - `attributes`: `block`, `attributes` (the whole list, in its order;
///   empty: none).
///
/// What would change nothing (the same name, base point, objects or
/// attributes; nothing unused to purge) writes nothing: the answer is
/// completed, with nothing in `changed` and `removed`.
///
/// Refusals (`CommandError.code`), checked in this order: `no_block` (none
/// given where the operation needs one); `rename`: `empty_name`; `redefine`:
/// `no_entities`, `invalid_uid` (each id in order), `not_finite` (the base
/// point), `no_layer`; `rebase`: `no_base`, `not_finite`; `attributes`:
/// `no_attributes`, then each attribute in order: `empty_tag` (empty or only
/// white space), `duplicate_tag` (a tag an earlier one has, exactly),
/// `not_finite` (its point, east first, then its turn), `invalid_height` (not
/// above zero); then
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `unknown_block`; `rename`: `duplicate_block`; `redefine`:
/// `entity_not_found`, `block_cycle` (an object is an
/// insert of this block or of one holding it), `block_too_deep`, with
/// `replace` the layer checks of `cad.blocks.define`; `remove`: `block_in_use`
/// (inserts of it in the drawing or in another definition).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksEdit {
    pub operation: BlockEditOperation,
    /// The definition; every operation but `purge`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<BlockId>,
    /// `rename`: the new name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// `redefine`: the objects it is made of now, by persistent id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    /// `rebase`: the new base point; `redefine`: the base point (absent: kept).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base: Option<Vec2>,
    /// `redefine`: true, the objects are replaced by an insert of the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub replace: Option<bool>,
    /// `redefine` with `replace`: the layer the insert goes on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    /// `attributes`: the definition's attribute definitions, the whole list
    /// in its order (empty: it has none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attributes: Option<Vec<crate::entity::AttributeDefinition>>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.blocks.edit` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksEdited {
    /// The definitions changed (`rename`, `redefine`, `rebase`, `attributes`).
    pub changed: Vec<BlockId>,
    /// The definitions deleted (`remove`, `purge`), in the drawing's order.
    pub removed: Vec<BlockId>,
    /// `redefine` with `replace`: the insert's persistent id and its slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub insert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<u32>,
    /// `redefine` with `replace`: the objects deleted, in the input's order.
    pub deleted: Vec<String>,
    /// The document's revision after the write (the same when nothing was
    /// written), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.blocks.edit` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksEditPlan {
    /// The definitions as execute would write them (`rename`, `redefine`, `rebase`, `attributes`).
    pub changed: Vec<BlockDefinition>,
    /// The definitions execute would delete (`remove`, `purge`).
    pub removed: Vec<BlockId>,
    /// `redefine` with `replace`: the insert as execute would write it, `id` 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub insert: Option<Entity>,
    /// `redefine` with `replace`: the objects execute would delete.
    pub deleted: Vec<String>,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
