//! The area operations as one product command (docs/adr/0622 §9, TODOS.md `UX-14`): Alan birleştir, kesiştir, çıkar and
//! böl, Alana and Çizgiye çevir, Parçaları birleştir and Parçalara ayır, Delik ekle, Deliği sil and Deliği doldur. The
//! caller gives the objects by persistent id and what the user gives (a cut line, a hole's ring, a point in a hole);
//! the command computes the shapes with the shared core's exact area algebra and writes them as `cad.entities.edit`
//! writes, one undo step named after the tool. The tools write through it; Python and an agent ask the same.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Edits areas.
pub const CAD_AREAS_EDIT: &str = "cad.areas.edit";
pub const CAD_AREAS_EDIT_VERSION: u32 = 1;

/// The operation, with what it works on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AreaWork {
    /// Alan birleştir: the areas go, their union comes from the first (its layer, colour and data); areas that do not
    /// touch stay apart, or with `single` make one multi-part area. A multi-part area takes part whole.
    Union {
        ids: Vec<String>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        single: bool,
    },
    /// Alan kesiştir: the areas' common part, a new blank area; with `erase` the areas go and it takes the first one's
    /// data.
    Intersect {
        ids: Vec<String>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        erase: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        single: bool,
    },
    /// Alan çıkar: the areas `cut` taken out of the areas `from`, each piece left with its source's data; one wholly
    /// inside leaves a hole; with `erase` the cutters go too (but not those on a locked layer).
    Subtract {
        from: Vec<String>,
        cut: Vec<String>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        erase: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        single: bool,
    },
    /// Alan böl: the areas split along a cut line through `line`'s points, or along the object `cutter` (Çizgiyle
    /// kes); the first piece is the area itself, the others new with its data.
    Split {
        ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        line: Option<Vec<Vec2>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        cutter: Option<String>,
    },
    /// Alana çevir: closed objects become areas in their places; the regions the line work among them closes become
    /// new blank areas on the first line's layer (the lines stay).
    ToArea { ids: Vec<String> },
    /// Çizgiye çevir: each area's outer ring becomes a closed polyline in its place; its holes new blank polylines.
    ToPolyline { ids: Vec<String> },
    /// Parçaları birleştir: areas, lines or points (one kind) become one multi-part object in the first one's place.
    PartsJoin { ids: Vec<String> },
    /// Parçalara ayır: multi-part objects come apart, the first part keeping the place, the others new with its data.
    PartsSplit { ids: Vec<String> },
    /// Delik ekle: the ring cut from the area as a hole; one touching a hole merges with it.
    HoleAdd {
        id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        ring: Vec<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        bulges: Option<Vec<f64>>,
    },
    /// Deliği sil: the area's hole around `at` removed.
    HoleRemove {
        id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
    /// Deliği doldur: the area's hole around `at` filled with a new area from it, the hole kept.
    HoleFill {
        id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
}

/// `cad.areas.edit`'s input.
///
/// Refusals, in this order: `not_finite`, `entity_not_found`, `too_few_objects` (fewer areas than the operation needs),
/// `not_an_area` (an object that encloses no area where one must), `no_overlap` (nothing in common, nothing cut),
/// `degenerate_shape` (a cut line or a ring of too few points), `no_hole` (no hole at the point), `invalid_hole` (the
/// ring not inside the area), then `cad.entities.edit`'s (`layer_locked` …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AreasEdit {
    pub work: AreaWork,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `cad.areas.edit` wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AreasEdited {
    /// The objects updated or replaced in place.
    pub changed: Vec<String>,
    /// The new objects' persistent ids.
    pub created: Vec<String>,
    /// The objects deleted.
    pub removed: Vec<String>,
    /// The objects the result is: what the tool selects, in order.
    pub result: Vec<String>,
    /// How many objects the operation acted on (areas joined, cut, split, converted).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// How many pieces the result has (areas, parts, polylines, points).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pieces: u32,
    /// The result's net area in square metres: the union, the overlap, what is left, the pieces, the regions; the
    /// area's after a hole changes; a joined line's length in metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub size: f64,
    /// Alan çıkar: areas wholly cut away. Parçaları birleştir: 1 when overlapping areas merged. Delik ekle: 1 when the
    /// ring merged with a hole. Alana çevir: the regions the lines closed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub more: u32,
    /// Each piece's net area in square metres, for Alan böl and Alana çevir's regions.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub sizes: Vec<f64>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
