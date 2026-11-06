//! KentOS Harita's work structure (plan/map-cad.md MAP-01, docs/adr/1200): `map.work.query` reads a map project's
//! items (survey campaigns, reference sources, working data, designs, check results, delivery packages) and what the
//! objects were derived from, each with how it stands now and why, and for one object whether it is a source, derived
//! from one or the user's own drawing; `map.work.edit` adds, changes, relinks and removes items and records
//! derivations. The desktop's handlers are `crates/native/application/src/map_work*.rs`; they and the Python SDK pass
//! the shared cases in `fixtures/commands/v1/map.work.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{WorkKind, WorkStatus};

pub const MAP_WORK_QUERY: &str = "map.work.query";
pub const MAP_WORK_QUERY_VERSION: u32 = 1;
pub const MAP_WORK_EDIT: &str = "map.work.edit";
pub const MAP_WORK_EDIT_VERSION: u32 = 1;

/// Input of `map.work.query` v1: `deep` reads every item's file (its SHA-256 against the item's revision: `changed`,
/// and the derivations of a changed file out of date); without it a file is only looked for (`found`, `missing`).
/// `uid` also says where that object comes from.
///
/// Refusals, in this order: `not_a_map_project`, `invalid_uid`, `entity_not_found`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapWorkQuery {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub deep: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
}

/// How an item's file stands: there (looked for only), the same content as its revision, other content, not there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WorkFileState {
    Found,
    Current,
    Changed,
    Missing,
}

/// How a row stands: up to date; an item whose file is not there; whose file changed since the project accepted it
/// (deep); a layer it names is not in the drawing; an approved item holding derived objects that are out of date. A
/// derivation: up to date, out of date (its source moved on), some of its objects gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WorkState {
    Current,
    Missing,
    Changed,
    Incomplete,
    Stale,
}

/// An item's file: where, the revision the project accepted, how it stands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkFileRow {
    pub path: String,
    pub revision: String,
    pub state: WorkFileState,
}

/// An item as it stands: its id, kind, name, status and layers, how many objects its layers hold, its file, its state
/// and why in sentences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkRow {
    pub id: String,
    pub kind: WorkKind,
    pub name: String,
    pub status: WorkStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<WorkFileRow>,
    pub state: WorkState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub why: Vec<String>,
}

/// A derivation as it stands: its id, source item, operation, the source's revision it was made from, how many of its
/// objects the drawing holds of how many, its state and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkDerivationRow {
    pub id: String,
    pub source: String,
    pub operation: String,
    pub revision: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub recorded: u32,
    pub state: WorkState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub why: Vec<String>,
}

/// What an object is in the map's work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WorkObjectRole {
    /// Source data: on a campaign's or a reference's layer, not derived.
    Source,
    /// Made from a source item: a derivation names it.
    Derived,
    /// The user's own drawing.
    Free,
}

/// Where one object comes from: its role, the item its layer belongs to, and when it is derived, the derivation, the
/// item it came from, the revision it was made from, the operation and whether it is up to date; a source object's
/// source is its own item and its revision that item's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkObjectSource {
    pub uid: String,
    pub role: WorkObjectRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub derivation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub current: Option<bool>,
}

/// Output of `map.work.query` v1: the items in the project's order, the derivations in theirs, the object asked about,
/// how many derivations are out of date, how many rows have a problem (an item not up to date, a derivation missing
/// objects), whether it was deep, and the revision read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapWorkTree {
    pub items: Vec<WorkRow>,
    pub derivations: Vec<WorkDerivationRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub object: Option<WorkObjectSource>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub problems: u32,
    pub deep: bool,
    pub revision: String,
}

/// What `map.work.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MapWorkOperation {
    /// A new item `is-N`: its kind, name, layers (each in no other item) and, for a file the project rests on, the
    /// file (an absolute path; its SHA-256 is its revision).
    Add {
        kind: WorkKind,
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        layers: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        file: Option<String>,
    },
    /// An item's name, status or layers. Approving an item that holds out-of-date derived objects is refused; the
    /// status is the office's record, never an institution's approval.
    Set {
        item: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        status: Option<WorkStatus>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        layers: Option<Vec<String>>,
    },
    /// An item taken out, with the derivations made from it (their objects stay, the user's own drawing).
    Remove { item: String },
    /// An item's file is now `file` (moved): the item's path, and the path of every raster and image on its layers that
    /// showed the old file, in one step; the new file's content is the item's revision (other content than before:
    /// warning `source_content_changed`, its derivations out of date).
    Relink { item: String, file: String },
    /// The item's file as it is now accepted as its revision; its derivations made from before are out of date.
    Refresh { item: String },
    /// Objects recorded as made from `source` (an item with a file) by `operation`, at its revision now.
    Derive { source: String, uids: Vec<String>, operation: String },
    /// A derivation's record taken away; its objects stay, the user's own drawing.
    Forget { derivation: String },
}

/// Input of `map.work.edit` v1: one operation, one undo step (“Harita işine ekle”, “Harita işi öğesini değiştir”,
/// “Harita işinden çıkar”, “Kaynağı yeniden bağla”, “Kaynağı yenile”, “Kaynaktan türet”, “Türetme kaydını sil”).
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_map_project`, then by operation:
/// `unknown_work_item`, `unknown_derivation`, `invalid_work_text` (a name or an operation not 1–200 characters without
/// space at its ends), `layer_not_found`, `layer_in_work_item` (a layer another item holds, or one given twice),
/// `nothing_to_set`, `invalid_file_path` (not an absolute path), `file_not_found`, `work_item_no_file` (relink, refresh
/// or derive from an item without a file), `no_entities`, `invalid_uid`, `entity_not_found`, `already_derived` (an
/// object another derivation names), `approval_stale`, `layer_locked` (a raster or an image to relink on a locked
/// layer), `work_full`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapWorkEdit {
    pub operation: MapWorkOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `map.work.edit` v1 (and its plan): the item or derivation it made or changed, the item's revision after
/// (its file's SHA-256), how many rasters and images a relink pointed at the new file, how many derivations were taken
/// away with an item, how many derivations are out of date after, and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapWorkEdited {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub derivation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file_revision: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub relinked: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub released: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}
