//! The command protocol (docs/adr/0253): how any product command is called,
//! what every answer adds to the command's own result, and what a document
//! records of the commands it ran.
//!
//! - [`CallEnvelope`]: a command by name, the call's form (`validate`,
//!   `plan`, `execute`), its input, the request's id and the state the input
//!   was prepared against ([`CallExpectations`]).
//! - [`CallAnswer`]: the command's [`CommandResult`] and, beside it, the
//!   [`CommandSummary`] (what it touched, what changed with it, the stamps
//!   before and after), the plan's id, whether the answer is a request
//!   answered before.
//! - [`CallRecord`]: a row of the document's journal (`kentos.calls/1`):
//!   what was run, with what, against what, by which algorithm, and what it
//!   made; [`ReplayJournal`] and [`ReplayReport`] replay them.
//! - [`RequestRecord`]: a request a document executed, kept in its file so a
//!   request sent again is never applied twice.
//! - [`JobStatus`]: a command run as a job, its phase and progress.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::command::{CommandError, CommandResult, CommandWarning};
use crate::document::Bounds;

/// The journal's rows (docs/adr/0253 §3).
pub const CALLS_FORMAT: &str = "kentos.calls/1";
/// A journal written out to be replayed (docs/adr/0253 §3).
pub const REPLAY_FORMAT: &str = "kentos.replay/1";
pub const REPLAY_VERSION: u32 = 1;
/// The numeric policy every command computes by today: float64 throughout,
/// the shared core's tolerances, the display rule (docs/adr/0149) only for
/// people (CLAUDE.md §23).
pub const NUMERIC_POLICY: &str = "kentos.numeric/1";

/// The most requests a document remembers (docs/adr/0253 §6).
pub const REQUEST_LEDGER: usize = 256;
/// The largest answer a request keeps, bytes of JSON; a larger one is kept
/// without its answer (`answer` none), and a request sent again says so.
pub const REQUEST_ANSWER_LIMIT: usize = 64 * 1024;
/// The most rows the journal keeps.
pub const JOURNAL_ROWS: usize = 2_000;
/// The largest input a row keeps, bytes of JSON; past it only its digest.
pub const JOURNAL_INPUT_LIMIT: usize = 1 << 20;
/// The most ids a row lists of each of created, changed, removed.
pub const JOURNAL_IDS: usize = 10_000;

/// What a call does (docs/adr/0013): `validate` and `plan` write nothing,
/// `execute` writes one change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CallOp {
    Validate,
    Plan,
    #[default]
    Execute,
}

impl CallOp {
    pub fn name(self) -> &'static str {
        match self {
            CallOp::Validate => "validate",
            CallOp::Plan => "plan",
            CallOp::Execute => "execute",
        }
    }
}

/// A call of a command (docs/adr/0253 §4): the same for Python, MCP, an
/// agent and the headless host; the command's input is unchanged by it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CallEnvelope {
    /// The command's id (`cad.entities.offset`).
    pub command: String,
    /// Its version; the host's when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub version: Option<u32>,
    #[serde(default)]
    pub op: CallOp,
    /// The command's input, as its catalog schema says.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: serde_json::Value,
    /// Names this write (1–128 of `A–Z a–z 0–9 . _ : -`): sent again with
    /// the same call it is answered as before and applied once; with
    /// another call it is refused (`request_reused`). Only `execute` takes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub request_id: Option<String>,
    /// The state the input was prepared against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected: Option<CallExpectations>,
    /// The write relaxes the constraints instead of keeping them (docs/adr/0337 §1): no solve follows it; the
    /// constraints it leaves unmet go with it (`constraints_relaxed`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub relax_constraints: bool,
}

/// What must still hold for the call to write (docs/adr/0253 §2): the
/// document's revision, objects' and definitions' stamps, the project's
/// parts' stamps, and the plan the write must equal. When one does not
/// hold, nothing is written: the answer is `conflict` (`revision_conflict`,
/// `stale_state`, `plan_changed`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CallExpectations {
    /// The document's revision, decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
    /// Objects by persistent id and the stamp each must still have.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub objects: BTreeMap<String, String>,
    /// Block definitions by id and the stamp each must still have.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub blocks: BTreeMap<String, String>,
    /// The layer tree's stamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<String>,
    /// The drawing's name and settings' stamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub settings: Option<String>,
    /// The project's style library's stamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub styles: Option<String>,
    /// The sheet book's stamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheets: Option<String>,
    /// The constraints' and parameters' stamp (docs/adr/0336 §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub constraints: Option<String>,
    /// The id of the plan the write must equal (a `plan` call's `planId`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plan: Option<String>,
}

impl CallExpectations {
    /// Whether nothing is expected.
    pub fn is_empty(&self) -> bool {
        self == &CallExpectations::default()
    }
}

/// The answer of a call (docs/adr/0253 §5): the command's own result, its
/// status and output as the catalog says, and beside it the summary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CallAnswer {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub result: CommandResult<serde_json::Value>,
    /// What the plan or the write touched; absent for `validate` and for
    /// what did not complete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub summary: Option<CommandSummary>,
    /// A plan's id: given back in `expected.plan`, the write must equal it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plan_id: Option<String>,
    /// The request was executed before: this is its answer then, nothing was applied now.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub replayed: bool,
    /// The request's write was undone since (only with `replayed`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub undone: bool,
    /// The journal's row of the write (its `seq`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub call: Option<u64>,
}

impl CallAnswer {
    /// The command's result alone.
    pub fn of(result: CommandResult<serde_json::Value>) -> Self {
        Self {
            result,
            summary: None,
            plan_id: None,
            replayed: false,
            undone: false,
            call: None,
        }
    }
}

/// A part of the project outside the objects, the layer tree's ops and the
/// blocks: project metadata touched by a command. `undo.outside` identifies
/// which parts are outside its undo step (ADR 0590).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProjectPart {
    /// The drawing's name.
    Name,
    Settings,
    /// The project's style library.
    Styles,
    /// The sheet book.
    Sheets,
    /// The layers' visibility, lock, snapping, names, folding and the active layer.
    LayerFlags,
    /// The drawing's constraints and parameters (docs/adr/0334, 0336).
    Constraints,
}

/// What a plan or a write touched (docs/adr/0253 §5), from the write itself.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandSummary {
    /// Objects added, by persistent id.
    pub added: Vec<String>,
    /// Objects changed.
    pub changed: Vec<String>,
    /// Objects removed.
    pub removed: Vec<String>,
    /// Of the changed and removed, those the input did not name: what
    /// changed with them (a dimension, a linked text, a hatch).
    pub dependents: Vec<String>,
    pub counts: SummaryCounts,
    /// The box of what was touched, before and after, metres; none when no
    /// object was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bounds: Option<Bounds>,
    /// Layers and groups added, changed (their style, fields, service,
    /// time, filter, card, composition, place) and removed, by id.
    pub layers: NodeChanges,
    /// Block definitions added, changed and removed, by id.
    pub blocks: NodeChanges,
    /// The project's parts that changed; `undo.outside` identifies those not restored by undo.
    pub project: Vec<ProjectPart>,
    pub undo: UndoInfo,
    /// The command's warnings, losses among them (`measure_lost`).
    pub warnings: Vec<CommandWarning>,
    /// The stamps before: of what was touched and the definitions it rests
    /// on, of the parts that changed, and the revision.
    pub sources: StateStamps,
    /// The stamps after: of what was added and changed, and of the parts.
    pub stamps: StateStamps,
    pub algorithm: AlgorithmInfo,
}

/// How many, of each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SummaryCounts {
    pub added: u32,
    pub changed: u32,
    pub removed: u32,
    pub dependents: u32,
}

/// Ids added, changed and removed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NodeChanges {
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

impl NodeChanges {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }
}

/// What undo takes back of a write.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UndoInfo {
    /// The undo step's name; none when the write made no step (only parts
    /// outside it), or none yet (inside a group: the group's step).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// The parts that changed and that undo does not take back.
    pub outside: Vec<ProjectPart>,
}

/// State stamps (docs/adr/0253 §2): 16 hex digits each.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StateStamps {
    /// The document's revision, decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub objects: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub blocks: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub settings: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub styles: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheets: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub constraints: Option<String>,
}

/// What computed a result: the command, its version, the numeric policy and
/// the shared core's build.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlgorithmInfo {
    pub command: String,
    pub version: u32,
    /// The numeric policy ([`NUMERIC_POLICY`]).
    pub policy: String,
    /// The core's build (`kentos-native-application` and its version).
    pub core: String,
}

/// A row's kind: a command's write, or another edit of the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CallKind {
    Command,
    /// A tool's or a window's edit made without a command, an import, a
    /// change of the project's parts: kept in the order, not replayed.
    Edit,
}

/// Whether a row's write is in the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CallStatus {
    Done,
    /// Undone since; redo makes it done again while it can.
    Undone,
}

/// Where a command computed (docs/adr/0253 §3): the drawing's unit, its
/// system, its type and the coordinate plane.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CallContext {
    /// `m`, `cm` or `mm` (docs/adr/0165 §2).
    pub unit: String,
    /// The project's SRID; 0 for a local project.
    pub srid: u32,
    /// `cad`, `gis` or `unset`.
    pub workspace: String,
    /// The coordinate plane the input is in: `world` (no user coordinate system yet).
    pub plane: String,
}

/// A file a command wrote (docs/adr/0253 §10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FileEffect {
    pub path: String,
    pub bytes: u64,
    /// SHA-256, 64 hex digits.
    pub sha256: String,
}

/// A row of the journal (`kentos.calls/1`, docs/adr/0253 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CallRecord {
    /// Its place in the document's order, from 1.
    pub seq: u64,
    pub kind: CallKind,
    /// The undo step's name, or the edit's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub version: Option<u32>,
    /// The command's input as its type writes it back (its defaults filled
    /// in); absent past [`JOURNAL_INPUT_LIMIT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub input: Option<serde_json::Value>,
    /// 32 hex digits of the SHA-256 of the input's canonical JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub input_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fingerprint: Option<String>,
    /// The document's revision before and after, decimal text.
    pub base: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub context: Option<CallContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub algorithm: Option<AlgorithmInfo>,
    /// The seed of a command that draws random numbers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub seed: Option<u64>,
    /// Persistent ids made, changed and removed, in the write's order.
    pub created: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
    /// Whether a list was cut at [`JOURNAL_IDS`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub truncated: bool,
    /// The write relaxed the constraints (docs/adr/0337 §1); a replay relaxes them too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub relax_constraints: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CommandWarning>>", optional))]
    pub warnings: Vec<CommandWarning>,
    /// The project's metadata parts it changed, including a grouped paper book (ADR 0590).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProjectPart>>", optional))]
    pub parts: Vec<ProjectPart>,
    /// The files it wrote.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<FileEffect>>", optional))]
    pub effects: Vec<FileEffect>,
    /// The undo step's number in this session; none for a write without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub step: Option<u64>,
    pub status: CallStatus,
}

/// The journal written out to be replayed elsewhere (`kentos.replay/1`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReplayJournal {
    /// [`REPLAY_FORMAT`].
    pub format: String,
    pub version: u32,
    /// Where the rows were run: the drawing's unit, system and type.
    pub context: CallContext,
    pub calls: Vec<CallRecord>,
}

/// What became of a row replayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReplayOutcome {
    /// The same counts, the same warnings' codes, as many made.
    Same,
    /// It ran, and something of that differs (`note` says what).
    Differs,
    /// It was refused (`error`).
    Failed,
    /// Not a command's (an edit), undone, or kept without its input.
    Skipped,
}

/// A row replayed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReplayStep {
    /// The row's `seq` in the journal replayed.
    pub seq: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub command: Option<String>,
    pub outcome: ReplayOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub error: Option<CommandError>,
    /// The ids it made here, in the order of the row's `created`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub created: Vec<String>,
}

/// A journal replayed (docs/adr/0253 §3).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReplayReport {
    pub steps: Vec<ReplayStep>,
    pub same: u32,
    pub differs: u32,
    pub failed: u32,
    pub skipped: u32,
    /// The journal's ids and the ids made here, as they were mapped.
    pub mapped: BTreeMap<String, String>,
}

/// A request a document executed (docs/adr/0253 §6), as its file keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RequestRecord {
    pub request_id: String,
    /// 32 hex digits: the call's fingerprint.
    pub fingerprint: String,
    pub command: String,
    pub version: u32,
    /// The document's revision after the write, decimal text.
    pub revision: String,
    pub status: CallStatus,
    /// The answer then, its JSON; none past [`REQUEST_ANSWER_LIMIT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub answer: Option<String>,
}

/// Where a request ledger breaks its rules (docs/specs/kcad-v2.md §6.13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestsFault {
    /// More than [`REQUEST_LEDGER`] requests.
    Count,
    /// The request at this place, and its field.
    Field(usize, &'static str),
}

/// What breaks the rules of a document's request ledger, when something
/// does: at most [`REQUEST_LEDGER`] requests, their ids by the rule and each
/// once, a fingerprint of 32 lowercase hex digits, a command named, a
/// version from 1, the revision decimal text, an answer of at most
/// [`REQUEST_ANSWER_LIMIT`] bytes. Readers and writers hold a file to it.
pub fn requests_problem(requests: &[RequestRecord]) -> Option<(RequestsFault, &'static str)> {
    if requests.len() > REQUEST_LEDGER {
        return Some((RequestsFault::Count, "en çok 256 istek yazılır"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, r) in requests.iter().enumerate() {
        if !request_id_valid(&r.request_id) {
            return Some((
                RequestsFault::Field(i, "requestId"),
                "istek kimliği 1–128 karakter, yalnız harf, rakam ve . _ : - olmalı",
            ));
        }
        if !seen.insert(r.request_id.as_str()) {
            return Some((
                RequestsFault::Field(i, "requestId"),
                "aynı istek iki kez yazılmış",
            ));
        }
        if r.fingerprint.len() != 32
            || !r
                .fingerprint
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Some((
                RequestsFault::Field(i, "fingerprint"),
                "parmak izi 32 küçük onaltılık hane olmalı",
            ));
        }
        if r.command.is_empty() {
            return Some((RequestsFault::Field(i, "command"), "komut adı boş"));
        }
        if r.version == 0 {
            return Some((RequestsFault::Field(i, "version"), "sürüm 1'den başlar"));
        }
        if r.revision.is_empty() || !r.revision.bytes().all(|b| b.is_ascii_digit()) {
            return Some((
                RequestsFault::Field(i, "revision"),
                "sürüm ondalık bir sayı metni olmalı",
            ));
        }
        if r.answer
            .as_ref()
            .is_some_and(|a| a.len() > REQUEST_ANSWER_LIMIT)
        {
            return Some((
                RequestsFault::Field(i, "answer"),
                "cevap en çok 65536 bayt olabilir",
            ));
        }
    }
    None
}

/// A request's id as the protocol takes it: 1–128 of `A–Z a–z 0–9 . _ : -`.
pub fn request_id_valid(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

/// A job's phase (docs/adr/0253 §9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum JobPhase {
    Queued,
    /// On a reading copy of the document, on another thread.
    Computing,
    /// Computed, not written yet.
    Staged,
    /// Being written; short and not to be stopped.
    Committing,
    Done,
    Failed,
    Conflict,
    Cancelled,
}

impl JobPhase {
    /// Whether the job is over.
    pub fn finished(self) -> bool {
        matches!(
            self,
            JobPhase::Done | JobPhase::Failed | JobPhase::Conflict | JobPhase::Cancelled
        )
    }
}

/// A job as its host reports it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JobStatus {
    pub id: String,
    pub command: String,
    pub phase: JobPhase,
    /// How far, 0 to 1, when the command says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub progress: Option<f64>,
    /// The answer once it is over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub answer: Option<CallAnswer>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_envelope_reads_with_its_defaults() {
        let call: CallEnvelope = serde_json::from_value(json!({
            "command": "cad.entities.delete",
            "input": { "uids": [] }
        }))
        .expect("reads");
        assert_eq!(call.op, CallOp::Execute);
        assert!(call.expected.is_none());
        let call: CallEnvelope = serde_json::from_value(json!({
            "command": "cad.entities.delete", "op": "plan", "requestId": "a-1",
            "expected": { "revision": "3", "objects": { "u": "0011223344556677" } }
        }))
        .expect("reads");
        assert_eq!(call.op, CallOp::Plan);
        assert_eq!(call.request_id.as_deref(), Some("a-1"));
        let expected = call.expected.expect("expected");
        assert_eq!(expected.objects["u"], "0011223344556677");
        assert!(!expected.is_empty());
    }

    #[test]
    fn an_answer_is_the_result_with_the_summary_beside_it() {
        let answer = CallAnswer {
            result: CommandResult::Completed {
                output: json!({ "revision": "4" }),
                warnings: vec![],
            },
            summary: None,
            plan_id: Some("ab".into()),
            replayed: true,
            undone: false,
            call: Some(7),
        };
        let v = serde_json::to_value(&answer).expect("writes");
        assert_eq!(
            v,
            json!({ "status": "completed", "output": { "revision": "4" }, "warnings": [],
                    "planId": "ab", "replayed": true, "call": 7 })
        );
        let back: CallAnswer = serde_json::from_value(v).expect("reads");
        assert_eq!(back, answer);
    }

    #[test]
    fn request_ids_are_short_and_plain() {
        assert!(request_id_valid("4f1c-a.b_c:d"));
        assert!(!request_id_valid(""));
        assert!(!request_id_valid("boşluk var"));
        assert!(!request_id_valid(&"x".repeat(129)));
    }
}
