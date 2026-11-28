//! File-host resource and layout inspection without an output path or filesystem writes.
use serde::{Deserialize, Serialize};
pub const FILE_PREFLIGHT_SHEETS: &str = "file.preflight.sheets";
pub const FILE_PREFLIGHT_SHEETS_VERSION: u32 = 1;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreflightSheets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawing: Option<String>,
    /// None inspects all pages; a supplied list must be nonempty and unique.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheets: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="dpi")))]
    pub dpi: Option<u16>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SheetsPreflighted {
    pub source: crate::SheetPublishPin,
    pub sheets: Vec<String>,
    pub issues: Vec<crate::SheetPublishIssue>,
    pub ready: bool,
}
