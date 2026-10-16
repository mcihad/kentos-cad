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

/// Blocks from a drawing file (docs/adr/0316 §3): Dosyadan blok ekle, Başka çizimden yeniden tanımla.
pub const FILE_IMPORT_BLOCK: &str = "file.import.block";
pub const FILE_IMPORT_BLOCK_VERSION: u32 = 1;

/// Places a block (docs/adr/0315 §4): Blok ekle's command.
pub const CAD_BLOCKS_INSERT: &str = "cad.blocks.insert";
pub const CAD_BLOCKS_INSERT_VERSION: u32 = 1;

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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
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
    /// The unit it is drawn in (docs/adr/0315 §4); absent: the drawing's. `unitless`: its numbers are kept in the
    /// drawing's units, an insert scaled by the drawing's unit wherever it goes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<crate::entity::BlockUnits>,
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
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
    /// A copy of a definition under a new name (“Blok kopyala”, docs/adr/0314 §3): its objects, base point,
    /// description and attributes; the drawing's inserts stay on the original.
    Duplicate,
    /// Definitions as a `kentos.blocks` file's text, with the definitions they insert (“Blokları dışa aktar”);
    /// nothing is written.
    Export,
    /// A `kentos.blocks` file's definitions taken in (“Blokları al”): a name the drawing has is kept, or with
    /// `replace` redefined; inserts in them point at the drawing's definitions of their names.
    Import,
    /// The unit it is drawn in (“Blok birimi”, docs/adr/0315 §4): with `rescale` its numbers are kept and their
    /// meaning changes (a block drawn in millimetres in a drawing taken for metres is a thousandth of its size
    /// after); a block made unitless or real-size keeps its size in this drawing.
    Units,
    /// Its objects given whole, in its own coordinates, each with its local id (“Blok düzenle”, the block editor's
    /// save; docs/adr/0316 §1): an id it had is kept (a reference to that piece follows it), 0 is a new object
    /// and takes the next id the definition never had; the base point and the attribute definitions when given.
    Update,
}

/// The format of a block library file (docs/adr/0314 §3).
pub const BLOCKS_FORMAT: &str = "kentos.blocks";
/// Its version.
pub const BLOCKS_VERSION: u32 = 1;

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
///   empty: none);
/// - `duplicate`: `block`, `name` (the copy's);
/// - `export`: `blocks` (absent: every definition);
/// - `import`: `library` (a `kentos.blocks` file's text), `replace`;
/// - `units`: `block`, `units`, `rescale`;
/// - `update`: `block`, `entities` (its objects, local ids kept, 0 new), `base` and `attributes` when they change.
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
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
    /// `export`: the definitions, by id (absent: every one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub blocks: Option<Vec<BlockId>>,
    /// `import`: a `kentos.blocks` file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub library: Option<String>,
    /// `update`: the definition's objects, in its own coordinates, each with its local id (0: a new one;
    /// docs/adr/0316 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub entities: Option<Vec<Entity>>,
    /// `units`: the unit it is drawn in (docs/adr/0315 §4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<crate::entity::BlockUnits>,
    /// `units` between two units: true keeps its numbers (its size changes by their ratio); absent or false keeps
    /// its size (only the unit's name changes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rescale: Option<bool>,
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub id: Option<u32>,
    /// `redefine` with `replace`: the objects deleted, in the input's order.
    pub deleted: Vec<String>,
    /// `duplicate`, `import`: the definitions added, in the drawing's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<BlockId>,
    /// `import`: the file's names the drawing has, kept as they are.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
    /// `export`: the file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub library: Option<String>,
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
    /// `duplicate`, `import`: the definitions execute would add.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<BlockDefinition>,
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

/// Input of `cad.blocks.insert` v1 (docs/adr/0315 §4): an insert of a block, named by its id or its name, at `at`
/// on `layerId`. Everything it depends on is here (TODOS.md CMD-07): Blok ekle gives the active layer, the current
/// colour, its scale, turn and mirror, the values asked.
///
/// The insert's factors: `scale` (absent: 1) and `yScale` (absent: `scale`), times the unit's factor: a unitless
/// block takes the drawing's unit (a 1 of it is a millimetre in a millimetre drawing); a real-size block keeps
/// its size (factor 1) and, drawn in another unit than the drawing's, is said so (`block_units_differ`).
///
/// Attribute values are the given ones by tag (an attribute left out shows its default); a tag the block does
/// not have is refused. `attrs` are the insert's other data (an object template's, a GIS record's), not checked
/// against the block's tags; a value under a tag of both is the attribute's.
///
/// Refusals, in this order: `no_block` (neither `block` nor `name`), `not_finite` (`at`, then `rotation`),
/// `invalid_scale` (`scale`, `yScale`: positive and finite); `invalid_revision`, `revision_conflict` (status
/// `conflict`); `unknown_block` (no such id or name, Turkish case folded); `unknown_tag`; `layer_not_found`,
/// `not_a_layer`, `layer_locked`; on the desktop `slots_exhausted`. Warnings: `layer_hidden`, `block_units_differ`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksInsert {
    /// The block, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<BlockId>,
    /// The block, by name, when `block` is absent (Turkish case folded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// Where its base point goes.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// The factor along its x axis (absent: 1), before the unit's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    /// The factor along its y axis (absent: `scale`), before the unit's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub y_scale: Option<f64>,
    /// Radians, counter-clockwise from east (absent: 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: Option<f64>,
    /// Mirrored in its x axis, before the turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mirror: Option<bool>,
    /// The layer it goes on, a layer's id.
    pub layer_id: String,
    /// Its own colour (absent: its layer's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Its attribute values by tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attributes: Option<std::collections::BTreeMap<String, String>>,
    /// Its other data (an object template's), under the attribute values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<std::collections::BTreeMap<String, String>>,
    /// Its label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// A library symbol over its layer's style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.blocks.insert` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlockInserted {
    /// The insert's persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub id: u32,
    /// The block placed.
    pub block: BlockId,
    /// The unit's factor in the insert's scale (1 for a real-size block).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub unit_factor: f64,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.blocks.insert` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksInsertPlan {
    /// The insert as execute would write it, `id` 0.
    pub insert: Entity,
    /// The unit's factor in its scale.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub unit_factor: f64,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// How `file.import.block` takes a drawing (docs/adr/0316 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BlockFileMode {
    /// The drawing's objects as one block (pictures and tables left out), its base the lower left of their extent.
    #[default]
    Drawing,
    /// The drawing's blocks by name, with the blocks they place.
    Blocks,
}

/// Input of `file.import.block` v1 (docs/adr/0316 §3): a `.kcad` drawing (v2, or v1 JSON) taken in, as one undo
/// step “Dosyadan blok: ad.kcad”: as one block (`drawing`: named `name`, else after the file; a name the drawing has
/// gets “(2)”), or its blocks by name (`blocks`: `names`, every one when absent; a name the drawing has is kept,
/// or with `replace` redefined: every insert shows the file's). The layers the objects are on come with them by
/// path. Refusals, in order: `invalid_file_path` (empty, relative), `file_not_found`, `import_unreadable`,
/// `invalid_revision`, `revision_conflict`, `nothing_to_import` (no object a block may hold; no block of the names),
/// `unknown_block` (a name the file has not), `block_refused` (the drawing's block rules), `layer_locked`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportBlock {
    /// The drawing to read, an absolute path.
    pub path: String,
    #[serde(default)]
    pub mode: BlockFileMode,
    /// `drawing`: the block's name (absent: the file's name without its extension).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// `blocks`: the names to take (absent: every block of the file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub names: Option<Vec<String>>,
    /// `blocks`: a name the drawing has is redefined from the file's block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub replace: Option<bool>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `file.import.block` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlockFileImported {
    /// `drawing`: the new block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<BlockId>,
    /// The definitions added and redefined, in the drawing's order.
    pub added: Vec<BlockId>,
    pub changed: Vec<BlockId>,
    /// The file's names the drawing has, kept as they are.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
    /// The layers and groups the import opened.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub layers: u32,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
