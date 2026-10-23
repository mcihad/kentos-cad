//! The shape edits as one product command (docs/adr/0622 §9, TODOS.md `UX-14`): Biçim değiştir, Sürdür, Paralel
//! kaydır, Sadeleştir, Kot ver and Okunur yap. The caller gives the objects by persistent id and what the user gives (a
//! sketched line, the drawn continuation, an edge and a distance or a target area, a tolerance, an elevation); the
//! command computes the shapes with the shared core and writes them as `cad.entities.edit` writes, one undo step named
//! after the tool. The tools write through it; Python and an agent ask the same.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Edits shapes.
pub const CAD_SHAPES_EDIT: &str = "cad.shapes.edit";
pub const CAD_SHAPES_EDIT_VERSION: u32 = 1;

/// Kot ver's elevation: one value for every vertex, a change to the ones there, or none.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "mode", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ShapeElevation {
    /// Sabit: every vertex at `z`.
    Set {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        z: f64,
    },
    /// Artır: every elevation there raised by `by` (lowered when negative); vertices without one stay without.
    Raise {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        by: f64,
    },
    /// Sıfırla: the elevations gone.
    Clear,
}

/// The edit, with what it works on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ShapeWork {
    /// Biçim değiştir: an area cut or grown, or a path's stretch redrawn, by the straight line through `line`'s
    /// points; a line becomes a polyline in its place.
    Reshape {
        id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        line: Vec<Vec2>,
    },
    /// Sürdür: a line or a polyline continued from its start (`fromStart`) or its end through `pts`, each drawn segment
    /// with its bulge (`bulges`, 0 straight); its own vertices keep their elevations.
    #[serde(rename_all = "camelCase")]
    Continue {
        id: String,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        from_start: bool,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        pts: Vec<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        bulges: Option<Vec<f64>>,
    },
    /// Paralel kaydır: the straight edge nearest `at` moved parallel to itself by `distance` (along its outward
    /// normal; an area's outward is away from the area), or until the area is `area` square metres; its neighbours
    /// lengthen or shorten to meet it.
    EdgeShift {
        id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        area: Option<f64>,
    },
    /// Sadeleştir: the paths' vertices within `tolerance` of their outline dropped.
    Simplify {
        ids: Vec<String>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        tolerance: f64,
    },
    /// Kot ver: the objects' vertices given elevations (lines, polylines, areas and points take them).
    Elevation {
        ids: Vec<String>,
        elevation: ShapeElevation,
    },
    /// Okunur yap: the texts that read upside down turned half round about their box's middle.
    Readable { ids: Vec<String> },
}

/// `cad.shapes.edit`'s input.
///
/// Refusals, in this order: `not_finite`, `invalid_uid`, `entity_not_found`, then the edit's own (`wrong_kind`,
/// `no_crossing`, `invalid_distance`, `nothing_to_do` …), then `cad.entities.edit`'s (`layer_locked` …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShapesEdit {
    pub work: ShapeWork,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `cad.shapes.edit` wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShapesEdited {
    /// The objects updated or replaced in place.
    pub changed: Vec<String>,
    /// The size before and after, for one object: an area's net area in square metres, a path's length in metres
    /// (Biçim değiştir, Sürdür, Paralel kaydır); zero otherwise.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub before: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub after: f64,
    /// How many objects were edited, and how many the edit left as they were (a kind it does not take, nothing to
    /// change).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    /// Sürdür: the vertices added. Sadeleştir: the vertices dropped. Okunur yap: the texts turned.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    /// Paralel kaydır: the distance the edge moved. Sadeleştir: the largest deviation. Both in the drawing's unit.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
