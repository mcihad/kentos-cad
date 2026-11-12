//! `cad.schema.describe` (docs/adr/0443 §1): the project's data schema found a page at a time, writing
//! nothing: its layers (with their fields), forms, value lists and relations, each as a short summary, its
//! whole definition only when asked (`expand`), narrowed by kind, by a layer and by words in its id or name.
//! What an agent reads of a project, a page at a time: not every schema and value in every context.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_SCHEMA_DESCRIBE: &str = "cad.schema.describe";
pub const CAD_SCHEMA_DESCRIBE_VERSION: u32 = 1;

/// The most a page lists, and how many when none is said.
pub const MOST_DESCRIBED: u32 = 100;
pub const DESCRIBED: u32 = 20;

/// What the schema holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SchemaKind {
    Layers,
    Forms,
    Domains,
    Relations,
}

/// Input of `cad.schema.describe` v1.
///
/// Refusals: `invalid_count` (a limit not 1–100), `invalid_cursor`, `layer_not_found`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SchemaDescribe {
    /// The kinds, listed in the order layers, forms, value lists, relations; none: all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SchemaKind>>", optional))]
    pub kinds: Vec<SchemaKind>,
    /// Words its id or name holds (Turkish folded); none: every one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub query: Option<String>,
    /// Only what a layer has: itself, its forms, the value lists its fields link, the relations it is a
    /// side of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The whole definitions too (a layer's fields, a form's draft and releases, a list's values, a
    /// relation); else summaries.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub expand: bool,
    /// The page after the one whose `next` this is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cursor: Option<String>,
    /// How many a page lists, 1–100; none: 20.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
}

/// One thing of the schema.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SchemaItem {
    pub kind: SchemaKind,
    pub id: String,
    pub name: String,
    /// A form's layer; a relation's parent layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// A relation's child layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub child: Option<String>,
    /// A layer's fields, a form draft's items, a list's values; none for a relation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: Option<u32>,
    /// A form's latest release; none unpublished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: Option<u32>,
    /// The whole definition, with `expand`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub detail: Option<serde_json::Value>,
}

/// Output of `cad.schema.describe` v1, and what plan says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SchemaDescribed {
    pub items: Vec<SchemaItem>,
    /// Every one found, all pages.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub total: u32,
    /// The next page's cursor; none on the last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<String>,
}
