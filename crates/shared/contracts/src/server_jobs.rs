//! Durable server jobs (docs/adr/0542; plan/gis.md CBS-39): a job a user asks the server to run for a project, its
//! state, progress, attempts and staged artifact. The input is kept as given with its SHA-256 and the project's data
//! revision when asked; a worker claims it with a lease and a fence, and finishes it as the user who asked, with the
//! access asked again. The kinds are the server's own: today `project.export`, the project as one `.kcad` file.

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The kinds of server jobs.
pub const SERVER_JOB_KINDS: &[&str] = &[PROJECT_EXPORT_JOB];
/// The project (a database project) as one KCAD v2 file, staged in the object store.
pub const PROJECT_EXPORT_JOB: &str = "project.export";
/// The most jobs one listing gives.
pub const SERVER_JOB_LIST_MAX: usize = 100;
/// The longest time a job may run, in seconds, and its default.
pub const SERVER_JOB_TIMEOUT_MAX: u32 = 86_400;
pub const SERVER_JOB_TIMEOUT_DEFAULT: u32 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ServerJobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl ServerJobState {
    pub fn of(text: &str) -> Option<Self> {
        Some(match text {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    /// Whether it will not change again.
    pub fn is_over(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

/// What a user asks for (`POST …/projects/{project}/jobs`). A request with a request id the same user sent before for
/// the project gets that job, whatever this one says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJobCreate {
    pub kind: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: Value,
    /// 1–200 characters: the client's own id of this request.
    pub request_id: String,
    /// Seconds, 1–86 400; absent: 600.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub timeout_seconds: Option<u32>,
}

/// `project.export`'s input: the export refused (`revision_conflict`) when the project is no longer at this data
/// revision when it runs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectExportInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// A succeeded job's artifact in the object store.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJobArtifact {
    /// Bytes, as text.
    pub size: String,
    pub sha256: String,
    /// The file name a download gives it.
    pub name: String,
}

/// Why a job failed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJobError {
    /// `timeout`, `lease_lost`, `forbidden`, `revision_conflict`, `gone`, `invalid`, `storage` or `failed`.
    pub code: String,
    pub message: String,
}

/// A job as its project's users see it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJob {
    pub id: String,
    pub project: String,
    pub kind: String,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: Value,
    pub input_sha256: String,
    /// The project's data revision when the job was asked for, as text.
    pub base_revision: String,
    pub state: ServerJobState,
    /// 0–1.
    pub progress: f32,
    pub attempts: u32,
    pub max_attempts: u32,
    pub timeout_seconds: u32,
    pub cancel_requested: bool,
    pub created_by: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub artifact: Option<ServerJobArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub error: Option<ServerJobError>,
}

/// A project's jobs, newest first, at most [`SERVER_JOB_LIST_MAX`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServerJobList {
    pub jobs: Vec<ServerJob>,
}
