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
}
