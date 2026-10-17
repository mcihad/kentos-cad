//! İşlemler's tools as product commands (docs/adr/0508, plan/gis.md CBS-41): `cad.processing.tools` lists the
//! tools and their parameters with the drawing's defaults, `cad.processing.run` runs one with explicit values, the
//! same runner the desktop's İşlemler window runs, one undo step named by the tool. What depends on a screen is not
//! read: a features input names a layer, every visible layer (`all`) or objects by their persistent ids, never the
//! selection or the view (CMD-07): those are the input's own `selection` and `view`. A host that has the processing
//! crate registers its tools, so the desktop, the headless host, Python and an MCP agent run the same tools.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_PROCESSING_TOOLS: &str = "cad.processing.tools";
pub const CAD_PROCESSING_TOOLS_VERSION: u32 = 1;
pub const CAD_PROCESSING_RUN: &str = "cad.processing.run";
pub const CAD_PROCESSING_RUN_VERSION: u32 = 1;

/// Input of `cad.processing.tools` v1: every tool, or those whose id, name, keywords or aliases hold `query`
/// (case and Turkish marks aside), or one category's (`category`, its id).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingTools {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub category: Option<String>,
}

/// A parameter of a tool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ToolParameter {
    pub name: String,
    pub label: String,
    /// What it takes: `features`, `number`, `text`, `boolean`, `choice`, `layer`, `point`, `expression`, `field`,
    /// `file`, `saveFile`, `network`, `addressSource`, `rasterValues`, `rasterPairs`, `fieldMap`, `layers`.
    pub kind: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub optional: bool,
    /// Shown under the window's Gelişmiş ayarlar.
    #[serde(default)]
    pub advanced: bool,
    /// Its value when not given, for this drawing (null: none).
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<serde_json::Value>"))]
    pub default: Option<Value>,
    /// A choice's values (empty for another kind).
    #[serde(default)]
    pub options: Vec<String>,
}

/// A tool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingTool {
    pub id: String,
    pub label: String,
    /// Its category's path (“Geometri › Tampon”).
    pub category: String,
    pub description: String,
    /// The names the command line takes for it.
    #[serde(default)]
    pub aliases: Vec<String>,
    pub parameters: Vec<ToolParameter>,
}

/// What `cad.processing.tools` lists.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingToolsListed {
    pub tools: Vec<ProcessingTool>,
}

/// Input of `cad.processing.run` v1: the tool (its id) and the values of its parameters; a parameter not given takes
/// its default for the drawing. A features input is `{ "scope": "all" }` (the visible layers), `{ "scope": "layer",
/// "layerId": … }`, `{ "scope": "uids", "uids": [ … ] }`, `{ "scope": "selection" }` (the objects `selection` names)
/// or `{ "scope": "visible" }` (those in `view`), optionally with `kinds`; a features input's default is the
/// selection, so a run on chosen objects gives `selection`. A file parameter is `{ "path": … }` (a table file read
/// as the dialog reads it) or `{ "name": …, "rows": [[…]] }`. Refusals, in order: `tools_unavailable`,
/// `unknown_tool`, `invalid_uid`, `entity_not_found`, `invalid_parameters` (each named: a parameter the tool has
/// not, a value its rules refuse, a file unread, the visible scope without `view`), `tool_failed` (it ran and
/// stopped: a locked layer, nothing to do, its own refusal; nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingRun {
    pub tool: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>", optional))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<BTreeMap<String, serde_json::Value>>")
    )]
    pub parameters: BTreeMap<String, Value>,
    /// The objects the run takes as chosen (persistent ids, in order): a features input's `selection` scope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub selection: Vec<String>,
    /// The box taken as on screen (minimum east, north, maximum east, north): a features input's `visible` scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub view: Option<[f64; 4]>,
    /// Where the result goes instead of the drawing (docs/adr/0508 §4): a new GeoPackage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sink: Option<RunSink>,
}

/// A run's result written to a new GeoPackage instead of the drawing (docs/adr/0508 §4): the tool runs on a copy,
/// the objects it made are written, each layer they are on a table (the layer's name, its fields as columns), and the
/// drawing does not change. The file is written whole or not at all; one that is there is replaced only with
/// `overwrite`, and the old file stays until the new one is complete.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RunSink {
    /// The GeoPackage's path (`.gpkg`).
    pub path: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub overwrite: bool,
}

/// A layer the run read: a bound layer gives its source and the revision it read (docs/adr/0503).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RunInput {
    pub layer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
}

/// A note the tool made while it ran.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ToolNote {
    /// `info` or `warn`.
    pub level: String,
    pub text: String,
}

/// What `cad.processing.run` did, or with `plan` would do (on a copy of the drawing).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingRan {
    pub tool: String,
    /// The tool's sentence of what it did.
    pub summary: String,
    /// The objects it made and changed (persistent ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub added: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub changed: Vec<String>,
    /// The objects it chose (a selecting tool's result; persistent ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub selected: Vec<String>,
    /// The layers it made (ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// Its outputs by name (a features output as persistent ids, a number, a text, a table as its rows).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>", optional))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<BTreeMap<String, serde_json::Value>>")
    )]
    pub outputs: BTreeMap<String, Value>,
    /// Its notes (what resolving left out, what it skipped).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ToolNote>>", optional))]
    pub notes: Vec<ToolNote>,
    /// The layers the features inputs read, in the inputs' order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RunInput>>", optional))]
    pub inputs: Vec<RunInput>,
    /// The GeoPackage the result went to (or with `plan` would go to), when the input gave a sink.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sink: Option<crate::GpkgExported>,
    /// The undo step's name; none when the drawing did not change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub step: Option<String>,
    pub revision: String,
}
