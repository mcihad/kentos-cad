//! The topology rules' commands of the product command catalog (docs/adr/0202, 0524, 0589): `cad.topology.check`
//! runs the project's rules over the drawing, read only, with the rows round a bound layer's part read from its source
//! as neighbours, says whether the check is complete, gives each finding's lasting key and the state it saw, and
//! previews a fix's effect; `cad.topology.exceptions` marks findings as left on purpose (bound to their objects'
//! geometry) or takes the mark away. The desktop's Topoloji tab answers through them (`crates/native/application`);
//! the shared cases are in `fixtures/commands/v1`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Vec2;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Runs the project's topology rules (docs/adr/0589 §1): read only.
pub const CAD_TOPOLOGY_CHECK: &str = "cad.topology.check";
pub const CAD_TOPOLOGY_CHECK_VERSION: u32 = 1;

/// Marks findings as exceptions or takes the mark away (docs/adr/0589 §3).
pub const CAD_TOPOLOGY_EXCEPTIONS: &str = "cad.topology.exceptions";
pub const CAD_TOPOLOGY_EXCEPTIONS_VERSION: u32 = 1;

/// The most findings `cad.topology.check` lists unless asked for fewer.
pub const TOPOLOGY_CHECK_LIMIT: u32 = 1000;
/// The most it lists when asked.
pub const TOPOLOGY_CHECK_LIMIT_MAX: u32 = 100_000;

/// A fix asked to be previewed: the finding's lasting key and the fix's key (`subtractFirst`, `mergeNeighbour` …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyFixAsk {
    pub key: String,
    pub fix: String,
}

/// Input of `cad.topology.check` v1: the project's rules (`rules`: some of them by id; none: all) over the objects of
/// the layers they name. A bound layer's part (docs/adr/0510) takes the rows of its source round the area as
/// neighbours (the band, docs/adr/0578). The findings listed are the first `limit` (1–100 000, default 1 000) in the
/// rules' order; `preview` previews one fix of one finding without writing it.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_limit`, `topology_rule_not_found` (`rules[i]`),
/// `finding_not_found` (`preview.key`), `fix_unavailable` (`preview.fix`: the finding does not offer it, the fix has
/// no answer, or it would change a row outside the part). Warnings: `topology_incomplete` (a part's neighbours could
/// not all be read: the check is not complete, no layer of it is said to be clean), `exception_lapsed` (an exception
/// whose objects' geometry changed: its finding is open again).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub preview: Option<TopologyFixAsk>,
}

/// One finding (docs/adr/0202 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyFinding {
    /// Its lasting key (docs/adr/0524 §1): the same finding found again keeps it.
    pub key: String,
    /// The rule's id, and its kind's key.
    pub rule: String,
    pub kind: String,
    /// The problem's key (`overlap`, `gap`, `sliver` …).
    pub problem: String,
    /// Its objects' persistent ids, in the order the finding says them.
    pub uids: Vec<String>,
    /// Of those, the rows outside a bound layer's part (read from its source, not in the drawing).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub outside: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// Its measure (an area, a length, a distance, an angle; `measureKind` says which), in metres, square metres or
    /// radians.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub measure: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub measure_kind: Option<String>,
    /// The fixes it offers, by key.
    pub fixes: Vec<String>,
    /// Marked as left on purpose.
    pub exception: bool,
    /// An exception was marked on it, but its objects' geometry changed since: open again (docs/adr/0589 §3).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub lapsed: bool,
}

/// A bound layer's part whose neighbours could not all be read, and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyGap {
    pub layer: String,
    pub reason: String,
}

/// A bound layer's source as the check saw it: the revision the layer last matched and, of a part, the digest of the
/// band's rows read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologySourceStamp {
    pub layer: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub band: Option<String>,
}

/// The state a check saw (docs/adr/0589 §2): the drawing's revision, a digest of the rules, the tolerance and the
/// exceptions, and each bound layer's source. A later check with the same stamp would find the same.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyStamp {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
    pub rules: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TopologySourceStamp>>", optional))]
    pub sources: Vec<TopologySourceStamp>,
}

/// An object a fix would change: its area or length before and after; `removed` when the fix deletes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyFixChange {
    pub uid: String,
    pub layer: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub removed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area_before: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area_after: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length_before: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length_after: Option<f64>,
}

/// A bound layer a fix would leave with changes to write to its source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyFixSource {
    pub layer: String,
    pub table: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
}

/// What a fix would do (docs/adr/0589 §4): the objects it changes and the bound layers' sources it touches. Nothing
/// is written; `cad.entities.edit`'s `topologyFix` writes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyFixPreview {
    pub key: String,
    pub fix: String,
    pub changes: Vec<TopologyFixChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TopologyFixSource>>", optional))]
    pub sources: Vec<TopologyFixSource>,
}

/// Output of `cad.topology.check` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyChecked {
    pub findings: Vec<TopologyFinding>,
    /// All the findings, the open ones, the exceptions and the lapsed exceptions.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub total: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub open: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub exceptions: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lapsed: u32,
    /// The rules checked, and those whose layer (or other layer) the project does not have.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rules: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub missing: u32,
    /// The drawing's objects the rules looked at, and the rows read round bound parts.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub neighbours: u32,
    /// Whether every neighbour a rule needs was read: false, no layer of `incomplete` is said to be clean.
    pub complete: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TopologyGap>>", optional))]
    pub incomplete: Vec<TopologyGap>,
    pub stamp: TopologyStamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub preview: Option<TopologyFixPreview>,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// What `cad.topology.exceptions` does to the findings it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TopologyExceptionOperation {
    /// İstisna yap: the finding left on purpose, bound to its objects' geometry as it is now (a lapsed exception is
    /// marked again, on the new geometry).
    Mark,
    /// İstisnayı kaldır.
    Unmark,
}

/// Input of `cad.topology.exceptions` v1: the findings of `keys` (lasting keys a check gave) marked or unmarked. The
/// exceptions are the project's settings: writing them makes the drawing dirty and is no undo step.
///
/// Refusals, in this order: `invalid_input` (no key, or one twice), `finding_not_found` (`keys[i]`: no finding of the
/// drawing as it is now has it). `expectedRevision` given and the drawing not at it: nothing is written, a conflict.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyExceptions {
    pub operation: TopologyExceptionOperation,
    pub keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.topology.exceptions` v1, and its plan: how many findings were marked or unmarked (those already so
/// are not counted), the project's exceptions after it and the revision after it (a plan: before it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyExceptionsEdited {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub exceptions: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
