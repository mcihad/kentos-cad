//! Persistent wall dimension recipes: source membership and annotation layout are separate.
#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{
    DimensionSet, DimensionSetFace, DimensionSetId, DimensionSetOpenings, DimensionSetReference,
    DimensionSetRow, DimensionSetSegment, Entity, EntityId, Vec2,
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

/// Create a live chain on one straight wall face, or along a façade of walls or a space's outline (docs/adr/0729).
/// Wire geometry is metres; gapMm is paper millimetres and uses the existing project plotScale once.
/// A wall of more than one edge needs `edge`, or `around` for a chain on each straight edge (docs/adr/0700).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetsCreateInput {
    pub layer_id: String,
    /// The wall measured (its edge; docs/adr/0695, 0700); with `walls`, a façade's first wall.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub wall: Option<String>,
    /// A façade's walls (docs/adr/0729): measured along `direction` with the walls at their ends and meeting them,
    /// their openings and the grids across them; `wall` first when given too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub walls: Vec<String>,
    /// A space whose outline is measured, with the openings on it and its holes (docs/adr/0729).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub space: Option<String>,
    /// A façade's or a space's direction along; absent: its first source's longest straight edge's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub direction: Option<Vec2>,
    /// A façade's or a space's lines measured: the walls' faces or their cores.
    #[serde(default, skip_serializing_if = "DimensionSetReference::is_faces")]
    #[cfg_attr(feature = "ts", ts(as = "Option<DimensionSetReference>", optional))]
    pub reference: DimensionSetReference,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// The wall path's straight edge measured (vertex `edge` to the next; a closed wall's last edge wraps to the
    /// first vertex). Absent: a two-point wall's only edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edge: Option<u32>,
    /// Every straight edge, in one step: a closed wall's outer face, an open wall's `face`; each first row as far
    /// out from its face as `at` is from `edge`'s (else the first straight edge's). A façade's or a space's: its
    /// four sides, along `direction` and square to it, each as far out from the outermost body as `at`.
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
        /// A façade's or a space's lines measured (docs/adr/0729).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        reference: Option<DimensionSetReference>,
    },
    /// A façade's walls, replaced (docs/adr/0729 §5).
    Sources {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        walls: Vec<String>,
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
    /// A façade's or a space's rows (docs/adr/0729 §5): each one's points, sources and dimensions, and the lines it
    /// measures.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DimensionSetRowNote>>", optional))]
    pub rows: Vec<DimensionSetRowNote>,
    /// The walls near a façade not measured: an oblique or curved edge where they meet it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub skipped: Vec<String>,
    /// The dimensions (by their place in `entities`) whose value does not fit between their extension lines
    /// (docs/adr/0729 §6): said, never hidden.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub tight: Vec<u32>,
}

/// A façade's or a space's row in a plan (docs/adr/0729 §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetRowNote {
    pub row: DimensionSetRow,
    /// Its points along the direction.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    /// The objects they come from.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub sources: u32,
    /// Its dimensions drawn (a span another row draws first is not drawn again).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub dimensions: u32,
    pub reference: DimensionSetReference,
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
