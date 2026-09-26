//! The envelope of a lasting command (CLAUDE.md §18): the same shape from
//! the interface, the CLI, MCP and chat. The server takes the actor from the
//! session and checks rights itself; nothing here is trusted as authority.
//!
//! How a product command call ends is [`CommandResult`] (TODOS.md CMD-05,
//! docs/adr/0022), on every host and in every mode: validate, plan and
//! execute (CMD-04). Locally a command needs no envelope and no tenant; the
//! envelope stays the server's wire form (CMD-06).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Parse → authenticate → authorize → preview/validate → transaction → outbox → result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandEnvelope {
    /// `alan.eylem`, e.g. "feature.update".
    pub command_name: String,
    /// Version of the command's input schema; unknown versions are refused.
    pub version: u32,
    pub tenant_id: String,
    pub project_id: String,
    pub request_id: String,
    /// The same key returns the same result on retry (no duplicate commit).
    pub idempotency_key: String,
    /// Row version each touched object had when the edit started; bigint as decimal text (§24.1).
    pub expected_versions: BTreeMap<String, String>,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: serde_json::Value,
}

/// How a product command call ended (TODOS.md CMD-05, docs/adr/0022). The
/// status is the tag, so no caller reads success out of a boolean, and a
/// refusal always says why. Every mode answers with it, each with its own
/// output: nothing (`null`) for validate, the plan for plan, the command's
/// output for execute.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandResult<T> {
    /// Done. After execute, what the command wrote is in the document as one undo step.
    Completed {
        output: T,
        /// What the caller should know although it completed (a hidden layer); empty when nothing.
        #[serde(default)]
        warnings: Vec<CommandWarning>,
    },
    /// Accepted as a job (cost `job`); the result comes with the job.
    Queued { job_id: String },
    /// Something only the user can give is missing; `error.path` names it.
    NeedsInput { error: CommandError },
    /// The document or project is no longer in the state the input was
    /// prepared for (its expected revision); nothing was written. Prepare
    /// the input again from the current state.
    Conflict { error: CommandError },
    /// The user or the caller cancelled it; nothing was written.
    Cancelled,
    /// Refused; nothing was written. `error` says why and how to fix it.
    Failed { error: CommandError },
}

/// Why a command did not complete (TODOS.md ARCH-07): a stable code for
/// programs, a message for people, the input field it is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandError {
    /// Stable, lowercase with underscores (`layer_locked`): programs branch on it, never on the message.
    pub code: String,
    /// Turkish, for people: the cause and how to fix it (CLAUDE.md §8).
    pub message: String,
    /// The input field it is about, as `pts[2].y` or `holes[0].bulges`; absent when none is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// The document's revision now (decimal text), when the error is about it: a conflict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
}

/// Something a completed command wants the caller to know.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandWarning {
    /// Stable, lowercase with underscores (`layer_hidden`).
    pub code: String,
    /// Turkish, for people.
    pub message: String,
    /// The input field it is about; absent when none is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The wire form every host writes and the web reads (the generated TS type).
    #[test]
    fn a_result_is_tagged_by_its_status() {
        let error = CommandError {
            code: "revision_conflict".into(),
            message: "…".into(),
            path: Some("expectedRevision".into()),
            revision: Some("7".into()),
        };
        let cases: [(CommandResult<()>, serde_json::Value); 6] = [
            (
                CommandResult::Completed {
                    output: (),
                    warnings: vec![],
                },
                json!({ "status": "completed", "output": null, "warnings": [] }),
            ),
            (
                CommandResult::Queued {
                    job_id: "j1".into(),
                },
                json!({ "status": "queued", "jobId": "j1" }),
            ),
            (
                CommandResult::NeedsInput {
                    error: CommandError {
                        path: None,
                        revision: None,
                        ..error.clone()
                    },
                },
                json!({ "status": "needs_input", "error": { "code": "revision_conflict", "message": "…" } }),
            ),
            (
                CommandResult::Conflict {
                    error: error.clone(),
                },
                json!({ "status": "conflict", "error": { "code": "revision_conflict", "message": "…", "path": "expectedRevision", "revision": "7" } }),
            ),
            (CommandResult::Cancelled, json!({ "status": "cancelled" })),
            (
                CommandResult::Failed {
                    error: error.clone(),
                },
                json!({ "status": "failed", "error": { "code": "revision_conflict", "message": "…", "path": "expectedRevision", "revision": "7" } }),
            ),
        ];
        for (result, wire) in cases {
            assert_eq!(serde_json::to_value(&result).expect("serializes"), wire);
            assert_eq!(
                serde_json::from_value::<CommandResult<()>>(wire).expect("reads back"),
                result
            );
        }
        // A completed result read without warnings has none.
        let read: CommandResult<u32> =
            serde_json::from_value(json!({ "status": "completed", "output": 3 })).expect("reads");
        assert_eq!(
            read,
            CommandResult::Completed {
                output: 3,
                warnings: vec![]
            }
        );
    }
}
