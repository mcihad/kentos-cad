//! The project's style library through a command (docs/adr/0314 §3; plan/cad.md CAD-STYLE-14): `cad.library.edit`
//! creates, updates, copies, renames, moves and removes the project's items (symbols, drawings and pictures, object
//! templates), takes a `.kstil` file's items in, gives them out as a `.kstil` file's text, and gives a symbol to
//! objects or layers. The library is a project part outside the undo steps (docs/adr/0092): writing it makes the
//! drawing unsaved; giving a symbol is one undo step “Sembol ver”. The system's and the user's libraries are the
//! host's: the command sees the project's.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_LIBRARY_EDIT: &str = "cad.library.edit";
pub const CAD_LIBRARY_EDIT_VERSION: u32 = 1;

/// What `cad.library.edit` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LibraryOperation {
    /// `item` added: its id one the library does not have.
    Create,
    /// `id`'s fields from `patch` (null takes one away); never its id or kind.
    Update,
    /// A copy of `id` under `newId` (absent: a new one), `name` (absent: “… (kopya)”) and `path`.
    Duplicate,
    /// `id` renamed to `name` (blank: “Adsız”).
    Rename,
    /// `id` moved to the category `path`.
    Move,
    /// `id` taken away when nothing uses it: an object's or a layer's symbol, a template, a symbol drawing it.
    Remove,
    /// A `.kstil` file's text (`kstil`) taken in; an id the library has kept, replaced (`conflict: replace`).
    Import,
    /// The items `ids` (absent: every one) with what they draw with, as a `.kstil` file's text; nothing is written.
    Export,
    /// The symbol `id` given to objects (`uids`) or a layer (`layer`, its single symbol): one undo step.
    Assign,
}

/// What an import does with an id the library has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LibraryConflict {
    /// The library's kept.
    #[default]
    Skip,
    /// The file's in its place.
    Replace,
}

/// Input of `cad.library.edit` v1.
///
/// Refusals (`CommandError.code`), in this order: `no_item` (create without `item`), `no_id` (an operation that
/// names an item without `id`), `no_kstil` (import without `kstil`), `no_targets` (assign without `uids` or
/// `layer`), `invalid_library_item` (an item the file's rules refuse: kind, name, path, a symbol's layers, a
/// drawing's format), `invalid_kstil` (a file that does not read, with its issues), `invalid_revision`,
/// `revision_conflict`, `library_item_exists` (create's id, duplicate's `newId`), `library_item_not_found`,
/// `library_item_in_use` (remove), `not_a_symbol` (assign of a drawing or a template), then assign's
/// `cad.entities.set` and `cad.layers.renderer` refusals.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LibraryEdit {
    pub operation: LibraryOperation,
    /// `create`: the item as the library keeps it (`id`, `kind`, `name`, `path`, the kind's fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub item: Option<serde_json::Value>,
    /// The item named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    /// `update`: the fields to set; null takes one away.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub patch: Option<serde_json::Value>,
    /// `duplicate`: the copy's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub new_id: Option<String>,
    /// `rename`, `duplicate`: the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// `move`, `duplicate`: the category path from its top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<Vec<String>>,
    /// `import`: a `.kstil` file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kstil: Option<String>,
    #[serde(default)]
    pub conflict: LibraryConflict,
    /// `export`: the items (absent: every one of the project's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ids: Option<Vec<String>>,
    /// `assign`: the objects, by persistent id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    /// `assign`: the layer whose single symbol it becomes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.library.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LibraryEdited {
    /// Whether anything was written.
    pub changed: bool,
    /// Items added (create, duplicate, import), by id.
    pub added: Vec<String>,
    /// Items changed (update, rename, move; import's replaced).
    pub replaced: Vec<String>,
    /// Import's items the library kept.
    pub skipped: Vec<String>,
    pub removed: Vec<String>,
    /// `export`: the `.kstil` file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kstil: Option<String>,
    /// `assign`: the objects given the symbol, by persistent id, and the layer.
    pub objects: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The document's revision afterwards (plan: the one it was made against), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
