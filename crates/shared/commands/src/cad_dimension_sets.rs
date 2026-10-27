//! Persistent wall dimension recipes: source membership and annotation layout are separate.
#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{
    DimensionSet, DimensionSetFace, DimensionSetId, DimensionSetOpenings, DimensionSetRow,
    DimensionSetSegment, Entity, EntityId, Vec2,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_DIMENSION_SETS_CREATE: &str = "cad.dimensionsets.create";
pub const CAD_DIMENSION_SETS_EDIT: &str = "cad.dimensionsets.edit";
pub const CAD_DIMENSION_SETS_QUERY: &str = "cad.dimensionsets.query";
pub const CAD_DIMENSION_SETS_VERSION: u32 = 1;

fn default_rows() -> Vec<DimensionSetRow> {
    vec![DimensionSetRow::Openings, DimensionSetRow::Overall]
}
fn default_gap() -> f64 {
    7.5
}

/// Create a live chain on one straight wall face. Wire geometry is metres;
/// gapMm is paper millimetres and uses the existing project plotScale once.
/// A wall of more than one edge needs `edge`, or `around` for a chain on each straight edge (docs/adr/0700).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetsCreateInput {
    pub layer_id: String,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub wall: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// The wall path's straight edge measured (vertex `edge` to the next; a closed wall's last edge wraps to the
    /// first vertex). Absent: a two-point wall's only edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edge: Option<u32>,
    /// Every straight edge, in one step: a closed wall's outer face, an open wall's `face`; each first row as far
    /// out from its face as `at` is from `edge`'s (else the first straight edge's).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub around: bool,
    #[serde(default)]
    pub face: DimensionSetFace,
    #[serde(default = "default_rows")]
    pub rows: Vec<DimensionSetRow>,
    #[serde(default)]
    pub openings: DimensionSetOpenings,
    #[serde(default = "default_gap")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub gap_mm: f64,
    /// Existing project style name; absent means Standart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dimension_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Explicit group/row operations; an individual dimension's source geometry
/// cannot silently escape or overwrite its owning recipe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionSetEdit {
    Move {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        row: Option<DimensionSetRow>,
    },
    RowOffset {
        row: DimensionSetRow,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        offset: f64,
    },
    Set {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        face: Option<DimensionSetFace>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        rows: Option<Vec<DimensionSetRow>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        openings: Option<DimensionSetOpenings>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
        gap_mm: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        dimension_style: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        layer_id: Option<String>,
    },
    /// Restore only explicitly suppressed intervals; preserve manual text placement.
    RestoreHidden,
    /// Remove the recipe, keeping its ordinary, individually associative dimensions.
    Detach,
    /// Remove the recipe and all its active dimensions together.
    Delete,
    ResetText {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        row: Option<DimensionSetRow>,
    },
    /// Clear manual row placements; return to the group's original first-row placement.
    ResetRows,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetsEditInput {
    pub id: DimensionSetId,
    pub operation: DimensionSetEdit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetsQueryInput {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DimensionSetId>>", optional))]
    pub ids: Vec<DimensionSetId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wall: Option<EntityId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetsPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set: Option<DimensionSetId>,
    pub entities: Vec<Entity>,
    pub spans: Vec<DimensionSetSegment>,
    pub removed: Vec<String>,
    /// Existing active member ids in this proposal; new members receive ids only on write.
    pub uids: Vec<String>,
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetsChanged {
    pub set: DimensionSetId,
    /// Every set `around` wrote, in edge order (`set` is the first); absent for one set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DimensionSetId>>", optional))]
    pub sets: Vec<DimensionSetId>,
    pub uids: Vec<String>,
    pub created: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetsQueried {
    pub sets: Vec<DimensionSet>,
    pub revision: String,
}
