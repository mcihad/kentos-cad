//! İki revizyonu karşılaştır (docs/adr/0576 §3; plan/gis.md `CBS-19`): what changed between a drawing as it is and
//! the same drawing as it was.
//!
//! `cad.drawing.compare` pairs the objects of the drawing and of another state of it by their persistent ids and
//! says which were added, removed or changed, geometry and attributes by the data compare core's rules (ADR 0179).
//! The other state is the drawing some undo steps back, or a drawing file (a revision downloaded, a copy). It writes
//! nothing.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_DRAWING_COMPARE: &str = "cad.drawing.compare";
pub const CAD_DRAWING_COMPARE_VERSION: u32 = 1;

/// The geometry tolerance when none is given, metres.
pub const COMPARE_TOLERANCE: f64 = 0.001;
/// The largest tolerance, metres.
pub const MAX_COMPARE_TOLERANCE: f64 = 10.0;
/// The ids each list holds when no limit is given.
pub const COMPARE_LIMIT: u32 = 1000;
/// The largest limit.
pub const MAX_COMPARE_LIMIT: u32 = 1_000_000;

/// The state the drawing is compared with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CompareAgainst {
    /// The drawing `steps` undo steps back (the open drawing's history, 1 to what it holds).
    #[serde(rename_all = "camelCase")]
    History {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        steps: u32,
    },
    /// A drawing file: its absolute path (`.kcad`, v2 or a v1 JSON file).
    #[serde(rename_all = "camelCase")]
    File { path: String },
}

/// Input of `cad.drawing.compare` v1. The drawing as it is, is the new side; `against`, the old. Within `tolerance`
/// metres two objects' geometry is the same. Refusals: `invalid_tolerance`, `no_such_step`, `invalid_file_path`,
/// `file_not_found`, `import_unreadable`, `invalid_limit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingCompare {
    pub against: CompareAgainst,
    /// How far points may lie apart and the geometry still be the same, metres, 0 to 10; absent: `COMPARE_TOLERANCE`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
    /// The most ids each list holds, 1 to 1 000 000; absent: `COMPARE_LIMIT`. A cut list says so (`truncated`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
}

/// An object that is in both and differs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ChangedObject {
    /// The object's persistent id.
    pub uid: String,
    /// `geometry`, `attributes` or `both`; the layer counts as an attribute (`layer` among the fields).
    pub status: String,
    /// The attributes that differ (`layer` for the layer), by their codes' order.
    pub fields: Vec<String>,
    /// How far the objects lie apart, metres, when their kinds agree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: Option<f64>,
}

/// Output of `cad.drawing.compare` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingCompared {
    /// What it was compared with, as the history or the file says it.
    pub against: String,
    /// Objects equal in both.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub same: u32,
    /// Objects in both that differ.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    /// Objects the drawing has now and the other state had not.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    /// Objects the other state had and the drawing has not now.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    /// The added objects' persistent ids, in the drawing's order.
    pub added_uids: Vec<String>,
    /// The removed objects' persistent ids, in the other state's order.
    pub removed_uids: Vec<String>,
    /// The changed objects, in the drawing's order.
    pub changed_objects: Vec<ChangedObject>,
    /// A list was cut at `limit`.
    pub truncated: bool,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
