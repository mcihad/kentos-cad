//! Projects kept as files (docs/adr/0031; TODOS.md SYNC-02..06, SYNC-16):
//! a cloud project whose content is a sequence of immutable, verified
//! `.kcad` v2 revisions (ADR 0025) instead of objects in PostGIS
//! (`ProjectStorage::File`).
//!
//! Saving a revision is capture → encode → upload → verify → commit →
//! acknowledge:
//!
//! 1. `POST …/projects/{project}/uploads` with [`FileUploadBegin`] (the size
//!    and the SHA-256 the client computed) opens an upload ([`FileUpload`]).
//! 2. `PUT …/uploads/{upload}` sends the bytes. The server writes them to a
//!    temporary object and keeps them only if the size and the hash match
//!    and they read as a KCAD v2 file.
//! 3. The product command `project.file.commit` ([`FileCommit`]) checks the
//!    caller's access again, compares `expectedVersions["@file"]` with the
//!    newest revision, makes the upload the next revision and tells the
//!    project's open connections ([`FileCommitted`]).
//!
//! `GET …/files` lists the revisions ([`FileRevisions`]); `GET …/files/{n}`
//! downloads one (with `project.download`). An upload that is not committed
//! within [`UPLOAD_LIFETIME_HOURS`] is removed with its bytes.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Makes an upload the project's next file revision.
pub const PROJECT_FILE_COMMIT: &str = "project.file.commit";
pub const PROJECT_FILE_COMMIT_VERSION: u32 = 1;

/// Key of the newest file revision in `expectedVersions` ("0" before the first).
pub const PROJECT_FILE_KEY: &str = "@file";

/// Event kind of a committed revision (`EventRecord.kind`, no objects).
pub const PROJECT_FILE_COMMITTED: &str = "project.file";

/// Largest file one upload may carry, in bytes (256 MiB; a 100 000-object
/// drawing is about 48 MB, ADR 0025). The server reads a file whole to verify
/// it, so larger projects wait for resumable, multipart uploads verified as
/// they stream (TODOS.md SYNC-10).
pub const FILE_UPLOAD_MAX: u32 = 256 * 1024 * 1024;

/// Hours an upload waits to be committed before it is removed with its bytes.
pub const UPLOAD_LIFETIME_HOURS: u32 = 24;

/// Opening an upload: what the client will send.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FileUploadBegin {
    /// Bytes, at most [`FILE_UPLOAD_MAX`].
    pub size: u32,
    /// The bytes' SHA-256, lowercase hex.
    pub sha256: String,
}

/// An upload as the server keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FileUpload {
    pub id: String,
    pub size: u32,
    pub sha256: String,
    /// RFC 3339.
    pub created_at: String,
    /// When it is removed if nobody commits it (RFC 3339).
    pub expires_at: String,
    /// The bytes arrived and were verified; it can be committed.
    pub received: bool,
}

/// Input of `project.file.commit` v1: the verified upload that becomes the
/// next revision. `expectedVersions["@file"]` is the revision the client's
/// file was based on ("0" for the first).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FileCommit {
    pub upload_id: String,
}

/// Output of `project.file.commit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FileCommitted {
    /// The new revision (decimal text).
    pub revision: String,
    pub size: u32,
    pub sha256: String,
    /// The stored answer of an earlier identical command (its key was seen).
    pub replayed: bool,
}

/// One committed revision of a file project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FileRevision {
    pub revision: String,
    pub size: u32,
    pub sha256: String,
    pub created_by: String,
    pub created_by_name: String,
    /// RFC 3339.
    pub created_at: String,
}

/// `GET …/files`: a file project's revisions, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FileRevisions {
    /// The newest revision; absent before the first commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub current: Option<String>,
    pub revisions: Vec<FileRevision>,
}
