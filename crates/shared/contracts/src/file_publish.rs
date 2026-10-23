//! Ordered sheet publication into a fresh delivery folder; the command never overwrites an issued set.
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;
pub const FILE_PUBLISH_SHEETS: &str = "file.publish.sheets";
pub const FILE_PUBLISH_SHEETS_VERSION: u32 = 1;
fn naming() -> String {
    "{index}-{sheet}".into()
}
fn yes() -> bool {
    true
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub enum SheetPublishMode {
    #[default]
    Single,
    Separate,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SheetPublishPin {
    pub revision: String,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishSheets {
    /// Absolute new or empty delivery folder; existing nonempty folders are refused.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    /// None means every sheet; an explicit list is used in exactly its supplied order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheets: Option<Vec<String>>,
    #[serde(default)]
    pub mode: SheetPublishMode,
    /// One PDF's leaf name; .pdf is appended when missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// Separate PDF leaf names: {index} (001...), {sheet}, {id}. Collisions get a stable numeric suffix.
    #[serde(default = "naming")]
    pub naming: String,
    /// None preserves each page's own physical raster setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="dpi")))]
    pub dpi: Option<u16>,
    #[serde(default = "yes")]
    pub geo: bool,
    #[serde(default = "yes")]
    pub layers: bool,
    /// A plan's immutable document pin; changed source refuses publication before any file is exposed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pin: Option<SheetPublishPin>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetPublishedFile {
    pub name: String,
    pub sheets: Vec<String>,
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="bytes")))]
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetPublishIssue {
    pub sheet: String,
    pub code: String,
    pub message: String,
    pub fix: String,
    pub error: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetsPublished {
    pub path: String,
    pub sheets: Vec<String>,
    pub files: Vec<SheetPublishedFile>,
    pub issues: Vec<SheetPublishIssue>,
    pub source: SheetPublishPin,
    pub ready: bool,
    pub manifest: String,
    pub sha256: String,
}
