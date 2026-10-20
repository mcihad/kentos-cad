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
pub const CAD_PROCESSING_BATCH: &str = "cad.processing.batch";
pub const CAD_PROCESSING_BATCH_VERSION: u32 = 1;

/// Input of `cad.processing.tools` v1: every tool, or those whose id, name, keywords or aliases hold `query`
/// (case and Turkish marks aside), or one category's (`category`, its id). `models` lists İşlemler's models too
/// (docs/adr/0529): a model's id is `model:` and its own id (`model:builtin.parcelSheet`), its parameters its inputs.
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub models: bool,
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
    pub sink: Option<RunSink>,    /// The open drawing's own file, an absolute path, when it has one: a raster's relative path is found from its
    /// folder (then in the project's search paths), a result written beside it. The host gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
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
    /// The layer's content when it was read: the SHA-256 of its objects' kinds, geometries and attributes by lasting id
    /// (`sources::layer_digest`; docs/adr/0511), so a result tells which state of its input it came from.
    pub digest: String,
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
    /// What the run was made of and how it went (docs/adr/0529 §2): the same record a script keeps to run it again.
    pub record: ProcessingRecord,
    pub revision: String,
}

/// The record of a run (docs/adr/0529 §2), made when it ends and handed back with its answer: what ran (tool or model
/// and its version), with which explicit values and objects, on which state of the data (the revision, each input
/// layer's content digest, the project's CRS), the seed where a tool has one, the engine and the host, and what
/// flowed through each step and how long it took. It is not kept in the project file: a script keeps it as JSON
/// (`kentos.run-record` 1) and `kentos_processing::record::rerun_kind` says whether running it again meets the same
/// revision, the same data under another revision, or changed data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingRecord {
    /// Always `kentos.run-record`.
    pub format: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: u32,
    /// `tool`, `model` or `batch`.
    pub kind: String,
    /// The tool's id, the model's (`model:…`), or a batch's tool.
    pub tool: String,
    pub label: String,
    /// The tool's version: bumped when what it computes from the same values changes (1 for every tool today); a
    /// model's, the version of the tool-and-model contract.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub tool_version: u32,
    /// A model's definition as the SHA-256 of its JSON (the steps, their wiring and fixed values); none for a tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub definition: Option<String>,
    /// The values the run took, defaults filled in, with objects as persistent ids (`scope: uids`) and a file as its
    /// name and path, not its rows: giving them to `cad.processing.run` runs it again.
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "BTreeMap<String, serde_json::Value>")
    )]
    pub parameters: BTreeMap<String, Value>,
    /// The input's own `selection` and `view`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub selection: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub view: Option<[f64; 4]>,
    /// The layers the features inputs read, each with its content digest when it was read (docs/adr/0511).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RunInput>>", optional))]
    pub inputs: Vec<RunInput>,
    /// The drawing's revision when the run began.
    pub document_revision: String,
    /// The project's coordinate system (EPSG/SRID); none for a local project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// The seed a tool with one took (its `seed` parameter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub seed: Option<u64>,
    /// `KentOS <version>`.
    pub engine: String,
    /// Where it ran: `desktop` or `headless` (Python, MCP, the agent's connection).
    pub host: String,
    /// When it began, UTC (`2026-10-19T09:30:00.000Z`).
    pub started_at: String,
    /// How long the whole took, milliseconds.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "ms")))]
    pub ms: u64,
    /// The rasters a raster tool read and wrote, each with its grid (docs/adr/0535): the run again on the same
    /// grids gives the same cells.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RecordGrid>>", optional))]
    pub grids: Vec<RecordGrid>,
    /// The tool as one step, a model's steps in order, a batch's rows in order.
    pub steps: Vec<RecordStep>,
    /// What flowed through the whole, counted when it ended.
    pub total: RecordTotal,
}

/// A raster a run read or wrote and its grid (docs/adr/0535): where its cells lie, their size and how many, the
/// band read, what counts as empty, the system; a source's `window` the cells the run worked out (an area's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordGrid {
    /// `input` or `output`.
    pub role: String,
    /// The raster's file (its path as the object names it), or none for one embedded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// `[x₀, a, b, y₀, c, d]`, GDAL's geotransform order: the cell (i, j)'s corner at x₀ + a·i + b·j, y₀ + c·i + d·j.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub affine: [f64; 6],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub height: u32,
    /// The band read (1 and up); none for a result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub band: Option<u32>,
    /// The value taken as empty; none: NaN only (or the file's own for a source read with its own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub nodata: Option<f64>,
    /// The raster's EPSG/SRID; none when it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// A source's cells worked out: first column, first row, columns, rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub window: Option<[u32; 4]>,
}

/// A step of a run (docs/adr/0529 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordStep {
    /// A model step's id, a batch row's `rows[i]`, a tool's own id.
    pub id: String,
    pub tool: String,
    /// A step's name (“1. adım (Köşe noktalarını numarala)”), a row's label.
    pub label: String,
    /// The objects its features inputs resolved to, and the distinct objects it made, changed or chose; counted when
    /// the step ended, so exact.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub records_in: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub records_out: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "ms")))]
    pub ms: u64,
    pub summary: String,
}

/// What flowed through a run in all, counted when it ended (docs/adr/0529 §2): `records_in` adds the steps' (an object
/// several steps read counts once for each), `records_out` counts the distinct objects the run made, changed or chose.
/// The steps' own counts are exact too; a tool that reports only approximate progress while it runs
/// (`Feedback::progress`) has this last, exact total.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordTotal {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub records_in: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub records_out: u64,
}

/// The values one row of a batch takes over the batch's (docs/adr/0529 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BatchRow {
    /// Its name in the answer (“Satır 3” when not given).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// The parameters it gives its own value to; every other parameter takes the batch's, then the default.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>", optional))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<BTreeMap<String, serde_json::Value>>")
    )]
    pub parameters: BTreeMap<String, Value>,
}

/// Input of `cad.processing.batch` v1: one tool (or model, `model:…`) run over a table of rows of parameter values,
/// through the same descriptor and executor as `cad.processing.run`: each row is the run `tool` with `parameters`
/// and the row's own values over them, with the input's `selection` and `view`. The rows run in order, each seeing
/// what the rows before it wrote; the whole batch is one undo step named “Toplu: <tool>”, written all or not at all:
/// a row that stops (or a stop asked between rows) takes back what the earlier rows did. At most 500 rows. Refusals,
/// in order: `tools_unavailable`, `empty_batch`, `too_many_rows`, then each row's as `cad.processing.run` refuses
/// them (`unknown_tool`, `invalid_uid`, `entity_not_found`, `invalid_parameters`), the path prefixed `rows[i].`; at run
/// time `batch_row_failed` (the row ran and stopped, or its values no longer fit what the rows before it wrote:
/// nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingBatch {
    pub tool: String,
    /// The values every row takes unless it gives its own.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>", optional))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<BTreeMap<String, serde_json::Value>>")
    )]
    pub parameters: BTreeMap<String, Value>,
    pub rows: Vec<BatchRow>,
    /// As `cad.processing.run`'s: the objects every row takes as chosen, and the box it takes as on screen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub selection: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub view: Option<[f64; 4]>,
    /// As `cad.processing.run`'s: the open drawing's own file, an absolute path, when it has one: a raster's relative path is found from its
    /// folder (then in the project's search paths), a result written beside it. The host gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A row of a batch as it ran.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BatchRowRan {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u32,
    pub label: String,
    /// What the row did, as `cad.processing.run` answers it (its own record included).
    pub ran: ProcessingRan,
}

/// What `cad.processing.batch` did, or with `plan` would do (on a copy of the drawing).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProcessingBatched {
    pub tool: String,
    pub summary: String,
    /// Every row, in order, as it ran.
    pub rows: Vec<BatchRowRan>,
    /// The undo step's name; none when the drawing did not change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub step: Option<String>,
    /// The batch's own record: its rows as steps.
    pub record: ProcessingRecord,
    pub revision: String,
}
