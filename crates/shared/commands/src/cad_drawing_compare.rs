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
    /// Optional suggestions for unmatched objects with different UUIDs. Suggestions never change identity or counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub match_search: Option<DrawingMatchSearch>,
}

/// Explicit search for identity candidates, not automatic pairing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingMatchSearch {
    /// Maximum geometric location difference, metres, 0 to 10. Equal source keys may suggest a moved object beyond it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub search: f64,
    /// Attribute containing a source-scoped id/handle. Unequal nonempty keys never match by proximity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    /// Old-side unmatched UUID scope. Absent: all removed objects; empty: none. At most 5000 objects per side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub old_uids: Option<Vec<String>>,
    /// New-side unmatched UUID scope. Absent: all added objects; empty: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub new_uids: Option<Vec<String>>,
}

/// An explicit proposal between two unmatched UUIDs. Neither evidence type guarantees identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingMatchCandidate {
    pub old_uid: String,
    pub new_uid: String,
    /// `sourceKey` for an equal nonempty attribute, otherwise `geometry`.
    pub evidence: String,
    /// Either object has more than one proposal, including proposals cut from the output by `limit`.
    pub ambiguous: bool,
    pub same_geometry: bool,
    /// The existing comparison core's location difference, metres; absent for different geometry kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: Option<f64>,
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

/// A non-model change, compared by the sheet/view's stable id rather than an entity slot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingProjectChange {
    /// `sheet`, `viewport`, `namedView`, `viewTemplate` or `sheetBook`.
    pub kind: String,
    /// Stable id; a viewport uses its sheet id and item id, separated by `/`.
    pub id: String,
    pub name: String,
    /// `added`, `removed` or `changed`.
    pub status: String,
    /// Changed JSON field paths; empty for an addition or removal. Array changes name the array.
    pub fields: Vec<String>,
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
    /// Non-model differences: sheets, viewports, named views, view templates and book settings.
    /// These are counted separately from model entities and do not create false geometry highlights.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<DrawingProjectChange>>"))]
    pub project_changes: Vec<DrawingProjectChange>,
    /// Proposed identities, in old/new document order. Original added/removed counts remain unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(optional, as = "Option<Vec<DrawingMatchCandidate>>")
    )]
    pub match_candidates: Vec<DrawingMatchCandidate>,
    /// A list was cut at `limit`.
    pub truncated: bool,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
