//! `cad.drawing.take` (Farkları al, docs/adr/1000 §5): chosen objects of another state of the drawing taken into the
//! open drawing, by their persistent ids: an object only the other state has comes back with its id, one only the
//! open drawing has goes, one that differs is replaced by its other state. All or nothing, one undo step
//! “Farkları al”; with a common base, an object both sides changed apart is a conflict.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::CompareAgainst;

pub const CAD_DRAWING_TAKE: &str = "cad.drawing.take";
pub const CAD_DRAWING_TAKE_VERSION: u32 = 1;
/// The undo step's name.
pub const DRAWING_TAKE_STEP: &str = "Farkları al";
/// The most ids one call takes.
pub const MAX_TAKE_UIDS: usize = 10_000;

/// Input of `cad.drawing.take` v1. Refusals, in order: `invalid_input`, `invalid_uid`, `revision_conflict`,
/// `no_such_step`, `invalid_file_path`, `file_not_found`, `import_unreadable`, `entity_not_found`, `layer_not_found`,
/// `unknown_block`, `layer_locked`, `take_conflict`. Warnings: `block_definition_differs`, `layer_style_differs`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingTake {
    /// The state taken from, as `cad.drawing.compare` names it.
    pub against: CompareAgainst,
    /// The objects' persistent ids, 1 to 10 000, none twice.
    pub uids: Vec<String>,
    /// The common state both sides came from: an object both changed apart from it is a conflict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base: Option<CompareAgainst>,
    /// Take a conflicting object anyway.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub overwrite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.drawing.take` v1: the ids by what happened, each in the input's order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingTaken {
    /// Brought back with their ids.
    pub added: Vec<String>,
    /// Replaced by their other state.
    pub replaced: Vec<String>,
    /// Taken out.
    pub removed: Vec<String>,
    /// The same in both: untouched.
    pub unchanged: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
