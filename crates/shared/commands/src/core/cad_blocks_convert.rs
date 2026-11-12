//! Chosen repeat sets become one definition, with explicit root-to-piece and dimension maps.
use crate::{BlockDefinition, BlockId, Entity, SubRef};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_BLOCKS_CONVERT: &str = "cad.blocks.convert";
pub const CAD_BLOCKS_CONVERT_VERSION: u32 = 1;

/// Converts only the chosen, non-overlapping groups after re-verifying their geometry and relations.
/// Plain analytic 2D CAD objects, representable single-line text and static nested blocks on one
/// untyped, data-free layer. Ordinary bound dimensions retain full nested paths. Explicit
/// text_to_attributes preserves differing words as per-insert values, with named mappings.
/// Unsupported appearance/data/relations are refused before any write, never detached.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlocksConvert {
    pub name: String,
    /// Source UUIDs. Definition pieces follow their document drawing order.
    pub source: Vec<String>,
    /// Chosen candidate member sets, at most 1000. Order within a set is immaterial.
    /// Each must have exactly as many distinct objects as the source. Sets cannot share objects.
    pub groups: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rotations: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mirrors: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scales: Option<bool>,
    /// Preserve trailing plain single-line text variations as named block attributes.
    /// Requires representable font/appearance/order and verification of each actual placed text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_to_attributes: Option<bool>,
    /// Absolute model tolerance in metres, as cad.blocks.find; absent: 0.000001.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct RepeatMemberMap {
    pub from: String,
    /// The first member in drawing order retains its root UUID as the insert.
    pub insert: String,
    /// Zero-based definition-piece path, NOT the definition's one-based local object id.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub path: Vec<u32>,
    /// For a converted text value, the destination attribute tag; path is then empty, not a SubRef leaf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attribute: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct RepeatDimensionMap {
    pub uid: String,
    /// a, b or c.
    pub point: String,
    pub before: SubRef,
    pub after: SubRef,
    /// Displacement caused by the chosen absolute matching tolerance, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub deviation: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct RepeatPlacement {
    pub uid: String,
    pub entity: Entity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct BlocksConvertPlan {
    /// Nil block UUID in a read-only plan; allocated only at execution.
    pub block: BlockDefinition,
    pub placements: Vec<RepeatPlacement>,
    pub mapping: Vec<RepeatMemberMap>,
    pub dimensions: Vec<RepeatDimensionMap>,
    /// Complete proposed dimension records for review; execution follows the same geometry.
    pub rebound: Vec<Entity>,
    /// Removed roots, excluding the identities retained as inserts.
    pub removed: Vec<String>,
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct BlocksConverted {
    pub block: BlockId,
    pub inserts: Vec<String>,
    pub mapping: Vec<RepeatMemberMap>,
    pub dimensions: Vec<RepeatDimensionMap>,
    pub removed: Vec<String>,
    pub revision: String,
}
