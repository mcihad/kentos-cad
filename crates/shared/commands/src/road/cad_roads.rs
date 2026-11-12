//! Plan yolu çizimi as one product command (docs/adr/0198, 0622 §9, TODOS.md `UX-14`): Plan yolu, Kavşak temizle and
//! Refüj kapat. The caller gives the axis, the areas or the two kerbs by persistent id and the sizes the user sets;
//! the command computes the shapes with the shared core (`ops::road`) and writes them as `cad.entities.create` and
//! `cad.entities.edit` write, one undo step named after the tool. The tools write through it; Python and an agent
//! ask the same.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Draws and tidies plan roads.
pub const CAD_ROADS_EDIT: &str = "cad.roads.edit";
pub const CAD_ROADS_EDIT_VERSION: u32 = 1;

/// The kind of road Plan yolu draws: its `Tür`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoadKind {
    /// Yol: a road, with kerbs and a median when asked.
    Road,
    /// Yaya yolu.
    Footway,
    /// Bisiklet yolu.
    Cycleway,
}

/// What is drawn or tidied.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoadWork {
    /// Plan yolu: the road `width` wide round `axis` (an area with its `Tür` and `Genişlik`); a Yol's kerbs `kerb` wide
    /// leave a carriageway area inside (Taşıt yolu), its median `median` wide an area closed with half circles
    /// (Refüj); `axisLine` writes the axis too (Yol ekseni). On the input's layer.
    #[serde(rename_all = "camelCase")]
    Plan {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        axis: Vec<Vec2>,
        road: RoadKind,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        width: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        kerb: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        median: Option<f64>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        axis_line: bool,
    },
    /// Kavşak temizle: the areas `ids` (past Refüj and Yol ekseni) joined kind by kind (`Tür`), each kind's inner
    /// corners rounded, Taşıt yolu's with `kerbRadius`, the others' with `blockRadius` (0 rounds none); a kind's first
    /// area takes the joined one, the others go.
    #[serde(rename_all = "camelCase")]
    Junctions {
        ids: Vec<String>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        block_radius: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        kerb_radius: f64,
    },
    /// Refüj kapat: the lines `first` and `second` closed into a median, the ends half circles (straight without
    /// `round`); the first becomes the area, the second goes.
    Median {
        first: String,
        second: String,
        #[serde(default = "yes")]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        round: bool,
    },
}

fn yes() -> bool {
    true
}

/// `cad.roads.edit`'s input.
///
/// Refusals, in this order: `not_finite`, `invalid_uid`, `entity_not_found`, then the work's own (`too_few_points`,
/// `invalid_distance`, `invalid_radius`, `not_an_area`, `not_a_path`, `multi_part`, `degenerate_shape`,
/// `invalid_input` …), then `cad.entities.create`'s and `cad.entities.edit`'s (`layer_locked` …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadsEdit {
    pub work: RoadWork,
    /// Plan yolu's layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `cad.roads.edit` wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadsEdited {
    pub changed: Vec<String>,
    pub created: Vec<String>,
    pub removed: Vec<String>,
    /// Plan yolu: the objects written. Kavşak temizle: the corners rounded. Refüj kapat: 1.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    /// Kavşak temizle: the inner corners left (no room).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
