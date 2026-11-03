//! Read-only, deterministic candidate recognition (CAD-ADV-07). No conversion is implied by a match.
use crate::{Bounds, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_BLOCKS_FIND: &str = "cad.blocks.find";
pub const CAD_BLOCKS_FIND_VERSION: u32 = 1;
pub const RECOGNITION_SOURCE_MAX: usize = 128;
pub const RECOGNITION_SCOPE_MAX: usize = 50_000;
pub const RECOGNITION_TOLERANCE: f64 = 0.000_001;

/// Finds whole collections equivalent to `source`; writes no document or selection state.
/// Supports analytic line/circle/arc and single-part 2D polyline sources (up to 4096 vertices each).
/// Polylines retain closure, analytic bulges and taper widths; paths with Z/M, holes, extra parts or
/// a room-separator role are refused. Source plus search scope is limited to 200000 polyline vertices.
/// Unsupported members refuse the whole source.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksFind {
    /// Ordered source object UUIDs, at most 128; repeated UUIDs are taken once.
    pub source: Vec<String>,
    /// Search object UUIDs, at most 50 000; absent searches the document, empty searches nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    /// Also recognize rotated collections; absent: true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rotations: Option<bool>,
    /// Also recognize reflected collections; absent: false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mirrors: Option<bool>,
    /// Also recognize uniformly scaled collections; absent: false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scales: Option<bool>,
    /// Explicitly ignore layer, appearance and object data in comparisons; absent: false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry_only: Option<bool>,
    /// Absolute model tolerance, metres, above zero to 0.001; absent: 0.000001 (0.001 mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
    /// Result count limit, 1 to 1000; absent: 1000. `truncated` means the scan is incomplete.
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
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlockMatch {
    /// Target UUID for each source UUID, in `BlocksFound.source` order: the source-to-target identity map.
    pub members: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub origin: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: f64,
    pub mirror: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bounds: Bounds,
    /// Shares objects with another candidate. Selection is harmless; future conversion must resolve overlap.
    pub overlaps: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlocksFound {
    pub source: Vec<String>,
    pub matches: Vec<BlockMatch>,
    /// Result or work budget reached; never claim that partial results cover the full search.
    pub truncated: bool,
    /// Analytic comparisons/transform trials performed, for reproducible performance evidence.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub comparisons: u64,
    /// Root objects linked to any matched member, counted once by kind. No relation is changed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub related: BTreeMap<String, u32>,
    pub revision: String,
}
