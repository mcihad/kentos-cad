//! Read-only line, grid and full-circle candidates, never implicit array conversion.
use crate::ArraysAdopt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ARRAYS_FIND: &str = "cad.arrays.find";
pub const CAD_ARRAYS_FIND_VERSION: u32 = 1;
pub const ARRAY_FIND_SCOPE_MAX: usize = 2048;

/// Requested pattern family. Omitted kind preserves the original linear search.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub enum ArrayFindKind {
    Line,
    /// World X/Y rectangular rows and columns; rotated/skew grids are not inferred.
    Grid,
    /// Rotated or skew regular rows/columns; both observed axes remain explicit in the proposal.
    Lattice,
    /// Explicit full-circle interpretation; at least four occupied places and an observed adjacent pair.
    Polar,
    /// Fixed spacing along an explicitly selected 2D guide, in its stored direction.
    Path,
}

/// Finds requested patterns containing the seed insert: static, data-free inserts
/// of the same block, appearance, mirror and scale. Line/grid require the same rotation.
/// At least three occupied places and an observed adjacent pair are required. Every second
/// place being occupied is indistinguishable from twice the spacing: no hidden places are invented.
/// With kind=grid, searches world-X/Y rectangular patterns with at least two rows,
/// two columns, four occupied places and observed adjacent pairs in both directions.
/// A bounding corner must be occupied to reuse it as the existing array source.
/// With kind=polar, searches full circles around placement base points; fixed or rotating item
/// orientation is verified by the existing adoption engine. Partial-arc intent is not inferred.
/// With kind=path, the selected seed must be at the guide start. Observed adjacent arc-length
/// gaps suggest fixed spacing; trailing unobserved places and reversed guide intent are not inferred.
/// With kind=lattice, independently directed regular axes are inferred from observed displacements.
/// Items keep their common direction; source-to-slot identity and gaps use the same adoption gate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArraysFind {
    /// Existing seed insert UUID. May be an interior place of a candidate.
    pub source: String,
    /// Omitted: line. Grid is world-X/Y; polar is an explicit full-circle interpretation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<ArrayFindKind>,
    /// Existing line, arc, circle or single-part 2D polyline UUID; required only with kind=path.
    /// The guide is retained and followed by the adopted array. Its stored start/direction are used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// Absent searches the document; empty searches only the seed. At most 2048 objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    /// Absolute position tolerance in metres, above zero to 0.001; absent: 0.000001.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
    /// Candidate limit, 1 to 100; absent: 100. Work/result limits report `truncated`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ArrayPattern {
    /// Review this exact proposal before calling cad.arrays.adopt. Never executed by the search.
    /// Source is the occupied line endpoint/grid corner, or the selected polar seed.
    /// Missing places stay empty; kept is never inferred.
    pub adoption: ArraysAdopt,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub present: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub missing: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub deviation: f64,
    /// Shares an existing insert with another returned proposal. They cannot both be adopted.
    pub overlaps: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ArraysFound {
    pub matches: Vec<ArrayPattern>,
    /// Compatible scope members, sorted by UUID. Not being in a proposal means unmatched, not kept.
    pub eligible: Vec<String>,
    /// Scope UUID -> explicit exclusion reason: not_insert, template_differs, layer_locked or array_member.
    pub excluded: BTreeMap<String, String>,
    pub truncated: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub comparisons: u64,
    pub revision: String,
}
