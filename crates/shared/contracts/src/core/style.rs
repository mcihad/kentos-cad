//! The .kstil style file (`StyleFile` in `apps/web/src/style/file.ts`), version 1.
//! Library items stay opaque here until the style core moves to Rust
//! (`crates/style-core`, CLAUDE.md §14); the TypeScript reader validates them.

use serde::{Deserialize, Serialize};

pub const STYLE_FORMAT: &str = "kentos-style";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StyleFile {
    pub format: String,
    pub version: u32,
    /// ISO 8601 time of export.
    pub exported: String,
    pub items: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<serde_json::Value>>,
}
