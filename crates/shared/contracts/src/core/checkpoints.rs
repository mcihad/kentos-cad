//! Checkpoints: named points of a project's history (docs/adr/0034;
//! TODOS.md SYNC-11, CLOUD-07).
//!
//! - A database project's checkpoint is its snapshot of one moment
//!   (docs/adr/0033) kept in the object store: the project as that data
//!   revision left it, as one KCAD v2 file.
//! - A file project's checkpoint names one of its revisions (the newest when
//!   none is given, docs/adr/0031); the revision is the content, nothing is
//!   copied.
//!
//! `project.checkpoint.create` makes one ([`CheckpointCreate`]) and
//! `project.checkpoint.delete` removes one ([`CheckpointDelete`]); both
//! answer with [`CheckpointChange`]. `project.checkpoint.restore`
//! ([`CheckpointRestore`]) makes a new project of one, or of a file
//! project's revision. `GET …/checkpoints` lists them
//! ([`ProjectCheckpoints`], `project.history`); `GET …/checkpoints/{id}`
//! downloads one's file (`project.history` and `project.download`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Names the project's present state (or a file project's revision).
pub const PROJECT_CHECKPOINT_CREATE: &str = "project.checkpoint.create";
pub const PROJECT_CHECKPOINT_CREATE_VERSION: u32 = 1;

/// Removes a checkpoint (never the revision a file project's checkpoint names).
pub const PROJECT_CHECKPOINT_DELETE: &str = "project.checkpoint.delete";
pub const PROJECT_CHECKPOINT_DELETE_VERSION: u32 = 1;

/// A new project from a checkpoint or a file revision, as a copy is made.
pub const PROJECT_CHECKPOINT_RESTORE: &str = "project.checkpoint.restore";
pub const PROJECT_CHECKPOINT_RESTORE_VERSION: u32 = 1;

/// Event kind of a checkpoint made or removed (`EventRecord.kind`, no objects).
pub const PROJECT_CHECKPOINT_EVENT: &str = "project.checkpoint";

/// Longest checkpoint name, in characters.
pub const CHECKPOINT_NAME_MAX: usize = 120;

/// Longest checkpoint note, in characters.
pub const CHECKPOINT_NOTE_MAX: usize = 2000;

/// What a checkpoint keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CheckpointKind {
    /// A database project's snapshot, kept in the object store.
    Snapshot,
    /// A file project's revision, named.
    Revision,
}

/// Input of `project.checkpoint.create` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CheckpointCreate {
    /// 1 to [`CHECKPOINT_NAME_MAX`] characters, e.g. "Belediyeye teslim".
    pub name: String,
    /// At most [`CHECKPOINT_NOTE_MAX`] characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    /// A file project's revision to name (decimal text); its newest when
    /// absent. A database project takes none: its checkpoint is its present state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file_revision: Option<String>,
}

/// Input of `project.checkpoint.delete` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CheckpointDelete {
    pub checkpoint_id: String,
}

/// Input of `project.checkpoint.restore` v1: exactly one of `checkpointId`
/// and `fileRevision` (a file project's revision). The answer is the new
/// project, as `project.duplicate` gives it (`ProjectDuplicated`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CheckpointRestore {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub checkpoint_id: Option<String>,
    /// A file project's revision (decimal text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file_revision: Option<String>,
    /// The new project's name; the source's with the point's name after it when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// The workspace of the new project; the source's when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tenant_id: Option<String>,
}

/// One checkpoint of a project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Checkpoint {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    pub kind: CheckpointKind,
    /// The data revision a snapshot shows, or the file revision a file
    /// project's checkpoint names (decimal text).
    pub revision: String,
    /// The file's size in bytes (decimal text).
    pub size: String,
    /// The file's SHA-256, lowercase hex.
    pub sha256: String,
    /// How many objects the file holds (decimal text), when it is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub objects: Option<String>,
    pub created_by: String,
    pub created_by_name: String,
    /// RFC 3339.
    pub created_at: String,
}

/// Output of `project.checkpoint.create` and `project.checkpoint.delete` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CheckpointChange {
    pub checkpoint: Checkpoint,
    /// It was removed (`project.checkpoint.delete`).
    pub removed: bool,
    /// The stored answer of an earlier identical command (its key was seen).
    #[serde(default)]
    pub replayed: bool,
}

/// `GET …/checkpoints`: a project's checkpoints, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectCheckpoints {
    pub checkpoints: Vec<Checkpoint>,
}
