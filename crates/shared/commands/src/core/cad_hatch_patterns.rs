//! `cad.hatchpatterns.edit` v1 (docs/adr/0331 §1): a project's own hatch patterns: a PAT file's taken in, one added
//! or replaced, one taken away, the project's written as a PAT file's text. The table is a project setting: no undo
//! step. A hatch carries its pattern's definition with it, so taking one away changes no hatch.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{HatchPatternDef, HatchPatternUnit};

pub const CAD_HATCH_PATTERNS_EDIT: &str = "cad.hatchpatterns.edit";
pub const CAD_HATCH_PATTERNS_EDIT_VERSION: u32 = 1;

/// What `cad.hatchpatterns.edit` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HatchPatternsOperation {
    /// The patterns of a PAT file's text taken in.
    Load,
    /// One pattern added, or one of its name replaced.
    Add,
    /// One pattern taken away.
    Remove,
    /// The project's patterns as a PAT file's text; writes nothing.
    Export,
}

/// Input of `cad.hatchpatterns.edit` v1. Refusals: `no_pat`, `no_definition`, `invalid_hatch_pattern`, `empty_name`,
/// `hatch_pattern_not_found`, `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchPatternsEdit {
    pub operation: HatchPatternsOperation,
    /// load: a PAT file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pat: Option<String>,
    /// load: what the file's numbers are in (a PAT file says not); absent: millimetres on paper.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<HatchPatternUnit>,
    /// load: the file's name, each pattern's source; absent: `dosya.pat`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    /// load: the patterns to take, by name; export: those to write. Absent: all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub names: Option<Vec<String>>,
    /// load, add: a pattern of a name the project has replaces it; else load skips it and add refuses.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub replace: bool,
    /// add: the pattern.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pattern: Option<HatchPatternDef>,
    /// remove: its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A PAT file's line that did not read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PatLineProblem {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub line: u32,
    pub message: String,
}

/// What `cad.hatchpatterns.edit` did.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchPatternsEdited {
    pub changed: bool,
    /// The patterns added, by name, in their order.
    pub added: Vec<String>,
    /// The project's patterns a load or an add replaced.
    pub replaced: Vec<String>,
    /// load: the file's patterns of a name the project has, kept as they are.
    pub skipped: Vec<String>,
    pub removed: Vec<String>,
    /// load: the file's lines that did not read.
    pub problems: Vec<PatLineProblem>,
    /// export: the PAT file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pat: Option<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
