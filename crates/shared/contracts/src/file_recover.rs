//! Kurtar (docs/adr/1013; plan/cad-advanced.md CAD-ADV-18): a `.kcad` the reader refuses, taken apart by the recovery
//! (`kentos_kcad::salvage`) and written as a separate copy beside it; the source is never written over. What came
//! back passed the strict reader; every part left out is listed with the reader's own words, and what the container
//! lacked (a checksum, a cut) is said. A file that reads whole needs no copy: none is written.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const FILE_RECOVER: &str = "file.recover";
pub const FILE_RECOVER_VERSION: u32 = 1;

/// Input of `file.recover` v1. Refusals, in order: `invalid_file_path` (`path` or `target` not an absolute `.kcad`
/// path, or `target` the source itself), `file_not_found` (`path`), `file_exists` (`target`), `kcad_unrecoverable`
/// (nothing of a drawing can be read), `file_unwritable`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecoverKcad {
    /// The damaged file: an absolute `.kcad` path.
    pub path: String,
    /// Where the recovered copy goes: an absolute `.kcad` path that is not there yet; absent: `<name>-kurtarilan.kcad`
    /// beside the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub target: Option<String>,
}

/// One part the recovery left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecoveredLoss {
    /// `document.<key>` for a part of the drawing, `entities[i]` for an object at its place in the file.
    pub part: String,
    /// The object's persistent id (32 hex digits) when it could be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    /// Why: the strict reader's code and words, or what was put in place.
    pub reason: String,
}

/// Output of `file.recover` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KcadRecovered {
    /// The copy's path (written by an execute, unless the file was whole).
    pub target: String,
    /// The file read whole: nothing to recover, no copy written.
    pub whole: bool,
    /// Whether this call wrote the copy.
    pub written: bool,
    /// What was wrong with the container, in words.
    pub container: Vec<String>,
    /// The objects the file held (as far as it could be read), and how many came back.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects_read: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects_kept: u32,
    /// Each part left out or put in place, the drawing's parts first, then the objects in the file's order.
    pub lost: Vec<RecoveredLoss>,
}
