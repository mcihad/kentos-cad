//! Measured partition: source and limits are explicit persistent wall edges, straight or arc (docs/adr/0734).
use crate::{Vec2, WallsCreate, WallsPlan};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WallEdgeRef {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::cad::UID_TEXT)))]
    pub wall: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub edge: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PartitionSide {
    #[default]
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PartitionMeasure {
    #[default]
    Clear,
    Axis,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PartitionDirection {
    #[default]
    Forward,
    Backward,
    Both,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PartitionExtent {
    #[default]
    Source,
    Between {
        start: WallEdgeRef,
        end: WallEdgeRef,
    },
    Length {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        anchor: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        length: f64,
        #[serde(default)]
        direction: PartitionDirection,
    },
}

/// Metres throughout. `wall` holds ordinary wall appearance/style/defaults; its path is generated,
/// so it must contain exactly one wall with an empty `pts`. Sources are read, never edited/deleted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WallsPartition {
    pub source: WallEdgeRef,
    #[serde(default)]
    pub side: PartitionSide,
    #[serde(default)]
    pub measure: PartitionMeasure,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
    #[serde(default)]
    pub extent: PartitionExtent,
    /// Same layer/style/appearance/attributes/revision rules as `cad.walls.create`.
    pub wall: WallsCreate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WallPartitionPlan {
    pub wall: WallsPlan,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub source_face: [Vec2; 2],
    /// The source face's arc between its two points on an arc source edge (tan of a quarter of its sweep,
    /// counter-clockwise positive); absent on a straight one (docs/adr/0734).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub source_face_bulge: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dimension: [Vec2; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub clear_distance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub axis_distance: f64,
}
