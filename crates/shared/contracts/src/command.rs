//! The envelope of a lasting command (CLAUDE.md §18): the same shape from
//! the interface, the CLI, MCP and chat. The server takes the actor from the
//! session and checks rights itself; nothing here is trusted as authority.

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
