//! KentOS Harita's work structure (plan/map-cad.md MAP-01, docs/adr/1200): `map.work.query` reads a map project's
//! items (survey campaigns, reference sources, working data, designs, check results, delivery packages) and what the
//! objects were derived from, each with how it stands now and why, and for one object whether it is a source, derived
//! from one or the user's own drawing; `map.work.edit` adds, changes, relinks and removes items and records
//! derivations. An item without a file has a content revision and an approval stamps the revision it approved
//! (docs/adr/2080). The desktop's handlers are `crates/native/application/src/map_work*.rs`; they and the Python SDK pass
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
pub const MAP_WORK_RESTORE: &str = "map.work.restore";
pub const MAP_WORK_RESTORE_VERSION: u32 = 1;

/// Input of `map.work.query` v1: `deep` reads every item's file (its SHA-256 against the item's revision: `changed`,
/// and the derivations of a changed file out of date) and works out every item's content revision; without it a file
/// is only looked for (`found`, `missing`) and a content revision is worked out only where it is judged (an item a
/// derivation comes from, an approved item with a stamp; docs/adr/2080 §1).
/// `uid` also says where that object comes from; `drawing` lets a moved project's files be found by their relative
/// paths and the search paths (docs/adr/1201 §1).
///
/// Refusals, in this order: `not_a_map_project`, `invalid_file_path` (`drawing` not absolute), `invalid_uid`,
/// `entity_not_found`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapWorkQuery {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub deep: bool,
    /// The open project's file, an absolute path (docs/adr/1201 §1): an item's file not at its path is looked for at
    /// its relative path from this file's folder, then in the project's search paths. None: only at its path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
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
/// (deep); a layer it names is not in the drawing; an approved item holding derived objects that are out of date; an
/// approved item whose own revision moved on since its approval stamped it (docs/adr/2080 §3). A derivation: up to
/// date, out of date (its source moved on), some of its objects gone.
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
    Revised,
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
    /// Where it was found (docs/adr/1201 §1): at its path, at its relative path from the project's folder, in a
    /// search path; `none` when missing.
    pub via: crate::AssetVia,
    /// The file found when it is not at its path (its relative path's or a search path's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
    /// Its relative path as the project keeps it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub relative: Option<String>,
}

/// An item as it stands: its id, kind, name, status and layers, how many objects its layers hold, its file, its
/// content revision when it has no file and it was worked out, the revision its approval stamped, its state and why
/// in sentences.
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
    /// Without a file: the digest of the objects its layers hold (`kentos.work-content/1`, docs/adr/2080 §1), when the
    /// read worked it out (deep, a source of a derivation, an approved item with a stamp).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub content: Option<String>,
    /// The revision its approval stamped (docs/adr/2080 §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approved: Option<String>,
    /// What changed since its approval, object by object (docs/adr/2880): on a revised item without a file whose
    /// approval kept its objects' digests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub changes: Option<WorkChanges>,
    /// Its typed layers' profile and how its classes stand (docs/adr/4880).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<WorkProfileRow>,
    pub state: WorkState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub why: Vec<String>,
}

/// The objects of an item that changed since its approval (docs/adr/2880): how many were added, removed and changed,
/// and the persistent ids of those there now (added or changed), by id, at most [`WORK_CHANGES_LISTED`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkChanges {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    /// Of the changed objects, how many changed in each part (docs/adr/3280; an object may change in several), when the
    /// approval kept its objects' parts (schema 211).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parts: Option<WorkChangedParts>,
    /// Each changed object with the parts that changed (docs/adr/3280), by id, at most [`WORK_CHANGES_LISTED`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<WorkChangedObject>>", optional))]
    pub objects: Vec<WorkChangedObject>,
    /// Each removed object as approved (docs/adr/3680), when the approval kept its objects' values (schema 213), by id,
    /// at most [`WORK_CHANGES_LISTED`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<WorkRemovedObject>>", optional))]
    pub removed_objects: Vec<WorkRemovedObject>,
}

/// A removed object as approved (docs/adr/3680): its persistent id and its snapshot form then (without the slot id).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkRemovedObject {
    pub uid: String,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub approved: serde_json::Value,
}

/// An attribute of a changed object that changed (docs/adr/3680): its name and its value as approved and now (none: it
/// was not there then, or is not now).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkChangedAttribute {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub approved: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub now: Option<serde_json::Value>,
}

/// How many changed objects changed in each part (docs/adr/3280): geometry (its kind's own fields: places, corners, a
/// text's words …), attributes (`attrs`, attachments), layer, look (colour, line, label, order …).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkChangedParts {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub geometry: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub attributes: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub layer: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub look: u32,
}

/// A changed object and the parts that changed (docs/adr/3280: `geometry`, `attributes`, `layer`, `look`, in that
/// order); with the approval's values kept (docs/adr/3680), each attribute that changed and whether its attachments did,
/// and (docs/adr/4480) when its geometry changed, the object as approved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkChangedObject {
    pub uid: String,
    pub parts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<WorkChangedAttribute>>", optional))]
    pub attributes: Vec<WorkChangedAttribute>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub attachments: bool,
    /// Its snapshot form as approved (without the slot id; docs/adr/4480), when its geometry changed and the approval
    /// kept its objects' values: the shape Harita işi draws over the drawing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "Record<string, unknown>"))]
    pub approved: Option<serde_json::Value>,
    /// How far its geometry moved from the approved one (docs/adr/4880), when its geometry changed and the approval kept
    /// its objects' values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shift: Option<WorkShift>,
}

/// How far a changed object's geometry moved from the approved one (docs/adr/4880): with as many vertices of the same
/// kind, vertex by vertex (the largest move and its number, the moves' root mean square); always the two outlines'
/// Hausdorff distance (the farthest a point of either is from the other) and where it is reached.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkShift {
    /// Its vertices pair one to one with the approved ones (same kind, as many).
    pub paired: bool,
    /// How many vertices it had approved and has now.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub vertices: [u32; 2],
    /// The largest vertex move, when paired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub largest: Option<WorkVertexMove>,
    /// The vertex moves' root mean square, when paired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub rms: Option<f64>,
    /// The outlines' Hausdorff distance, to a millionth of a millimetre; none for an object without a point (a text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hausdorff: Option<WorkFarthest>,
}

/// One vertex's move: its number (from 0, in the object's vertex order), east and north, its length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkVertexMove {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub index: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub dx: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub dy: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub d: f64,
}

/// The Hausdorff distance and where it is reached: `from` on the approved outline, `to` on the outline now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkFarthest {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub d: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: [f64; 2],
}

/// An item's typed layers' profile as it stands (docs/adr/4880): the profile (its title and version, when it was
/// found), each class the item holds with its layer, objects and findings, all the findings, and the item's layers that
/// hold objects but no class: its free drawing layers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkProfileRow {
    pub name: String,
    /// The profile was found (built in, the drawing's copy or the project's folder) and holds.
    pub found: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::ProfileClassSummary>>", optional))]
    pub classes: Vec<crate::ProfileClassSummary>,
    /// The classes' findings (`cad.profiles.check`'s), at most [`WORK_CHANGES_LISTED`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::ProfileFinding>>", optional))]
    pub findings: Vec<crate::ProfileFinding>,
    /// The item's layers holding objects that no class of it types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub free: Vec<String>,
}

/// The most ids [`WorkChanges`] lists.
pub const WORK_CHANGES_LISTED: usize = 1000;

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
/// source is its own item and its revision that item's (its file's, or its content revision).
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
    /// An item's name, status or layers. Approving stamps the item's revision then (its file's, or without a file its
    /// content revision; docs/adr/2080 §3); approving an approved item that moved on since stamps it again; leaving
    /// the approved status takes the stamp away. Approving an item that holds out-of-date derived objects is refused;
    /// the status is the office's record, never an institution's approval.
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
    /// An item's typed layers' profile (docs/adr/4880): the data profile `name` (built in, the drawing's copy or the
    /// project's folder by `drawing`) and its `classes` (none: all). Each class's layer the drawing lacks is added as the
    /// profile places it, the fields a class's layer lacks are added (a layer's own field of another kind is kept,
    /// warning `profile_field_differs`; a locked layer gets none, `profile_layer_skipped`), and the classes' layers join
    /// the item; its other layers stay free drawing layers. Without `name` the profile is taken away; the layers stay.
    Profile {
        item: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        classes: Vec<String>,
    },
    /// An item taken out, with the derivations made from it (their objects stay, the user's own drawing).
    Remove { item: String },
    /// An item's file is now `file` (moved): the item's path, and the path of every raster and image on its layers that
    /// showed the old file, in one step; the new file's content is the item's revision (other content than before:
    /// warning `source_content_changed`, its derivations out of date). External references whose source was the old
    /// file follow it in the same step when the new file's content is the one they last loaded (docs/adr/1201 §2); one
    /// that loaded other content, and a bound layer on the old file (its link is the project's setting, outside undo),
    /// stay (warning `link_not_followed`: Dış başvurular › Yolu değiştir, Kaynak › Yeniden bağla).
    Relink { item: String, file: String },
    /// The item's file as it is now accepted as its revision; its derivations made from before are out of date.
    Refresh { item: String },
    /// Objects recorded as made from `source` by `operation`, at its revision now: its file's, or without a file its
    /// content revision (docs/adr/2080 §2); an object its own layers hold is not made from an item without a file.
    Derive { source: String, uids: Vec<String>, operation: String },
    /// A derivation's record taken away; its objects stay, the user's own drawing.
    Forget { derivation: String },
    /// Objects let go of their derivations (promoted to the user's own drawing, docs/adr/1201 §3): each object out of the derivation
    /// that names it, a derivation left without objects taken away; the objects stay, the user's own drawing.
    Release { uids: Vec<String> },
}

/// Input of `map.work.edit` v1: one operation, one undo step (“Harita işine ekle”, “Harita işi öğesini değiştir”,
/// “Harita işinden çıkar”, “Kaynağı yeniden bağla”, “Kaynağı yenile”, “Kaynaktan türet”, “Türetme kaydını sil”,
/// “Bağımsız yap”).
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_map_project`, then by operation:
/// `unknown_work_item`, `unknown_derivation`, `invalid_work_text` (a name or an operation not 1–200 characters without
/// space at its ends), `layer_not_found`, `layer_in_work_item` (a layer another item holds, or one given twice),
/// `nothing_to_set`, `invalid_file_path` (not an absolute path), `file_not_found`, `work_item_no_file` (relink or
/// refresh of an item without a file), `no_entities`, `invalid_uid`, `entity_not_found`, `already_derived` (an object
/// another derivation names), `own_content` (an object of the source's own layers, the source without a file),
/// `not_derived` (release of an object no derivation names), `approval_stale`, `unknown_profile` and `invalid_profile`
/// (a profile, `operation.name`; an approved item's, `operation.status`), `unknown_profile_class` (`operation.classes[i]`),
/// `approval_profile` (approving an item whose typed layers break its profile), `layer_refused` (a class's layer or
/// field the drawing refuses), `layer_locked` (a raster, an image, an external reference's insert or a bound layer to relink on a locked layer),
/// `work_full`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapWorkEdit {
    pub operation: MapWorkOperation,
    /// The open project's file, an absolute path (docs/adr/4880): a profile not built in is looked for in its folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `map.work.edit` v1 (and its plan): the item or derivation it made or changed, the item's revision after
/// (its file's SHA-256, or without a file its content revision a derive recorded or an approval stamped), the stamp an
/// approval wrote, how many rasters and images a relink pointed at the new file and how many external references,
/// how many derivations were taken away with an item (or emptied by a release), how many objects a
/// release let go of, how many derivations are out of date after, and the drawing's revision.
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
    /// Without a file: the item's content revision a derive recorded or an approval stamped (docs/adr/2080 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub content: Option<String>,
    /// The stamp an approval wrote (docs/adr/2080 §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approved: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub relinked: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub released: u32,
    /// How many external references a relink pointed at the new file (docs/adr/1201 §2).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub links: u32,
    /// How many objects a release let go of.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub freed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    /// The layers a profile added (docs/adr/4880).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// The fields a profile added, `layer.field`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// Input of `map.work.restore` v1 (docs/adr/4080): an approved item's objects put back as its approval kept them (its
/// values, docs/adr/3680), in one undo step “Onaylı hâline getir”. An approved object no longer in the drawing is made
/// again with its persistent id; one changed (or moved off the item's layers) is set back to its value. `uids` names
/// only those (each one of the item's approved objects, removed or changed now); none: all of them. Objects added
/// since the approval stay. All of them are put back or none.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_map_project`, `unknown_work_item` (`item`),
/// `work_not_approved`, `no_approved_values`, `invalid_uid` and `not_approved_object` (`uids[i]`), `nothing_to_restore`
/// (`uids[i]`: that object is as approved; `item`: nothing is), then for each object put back its layer as approved
/// (`layer_not_found`, `not_a_layer`, `layer_locked`, `layer_geometry_mismatch`) and a changed object's layer now
/// (`layer_locked`), at `uids[i]` or `item`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapWorkRestore {
    /// The item's id (`is-N`).
    pub item: String,
    /// Persistent ids of the approved objects to put back; none: every one removed or changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub uids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `map.work.restore` v1 (docs/adr/4080): the objects made again (they were removed) and those set back (they
/// were changed), each by its persistent id in order; how many objects added since the approval stay; whether the
/// item's content is now the one approved (its revision its approval's stamp); the document's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapWorkRestored {
    pub item: String,
    pub restored: Vec<String>,
    pub reverted: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    pub as_approved: bool,
    pub revision: String,
}

