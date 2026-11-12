//! API responses (`/v1/...`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// `GET /v1/health`: the server is up, and which build and contracts it speaks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Health {
    #[cfg_attr(feature = "ts", ts(type = "\"ok\""))]
    pub status: String,
    pub service: String,
    pub version: String,
    /// Git commit the server was built from, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub commit: Option<String>,
    /// `CONTRACTS_VERSION` of this build.
    pub contracts: u32,
    /// Supported project.changes envelope versions. Absent on older servers: no
    /// opt-in capability, so a client must not send architectural dimension recipes.
    #[serde(
        default,
        rename = "projectChangesVersions",
        skip_serializing_if = "Vec::is_empty"
    )]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<u32>>"))]
    pub project_changes_versions: Vec<u32>,
}
