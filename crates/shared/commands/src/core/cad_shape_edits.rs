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

/// How Parçala cuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SplitMethod {
    /// Kesişimlerden: where the given objects cross one another.
    Crossings,
    /// Eşit parçalara: into `parts` pieces of equal length.
    Equal,
    /// Uzunluktan: into pieces `length` long, the last one what remains.
    Length,
}

/// Paralel kaydır's reference (docs/adr/2580): the straight segment nearest `at` of object `id` (a line, or a path's
/// or an area's straight edge; one on a locked layer too, it is only read) that the edge keeps its distance from. With
/// it, `distance` is the distance to hold between the reference's line and the edge, on the side the edge is now (not
/// a shift); the edge must be straight and parallel to the reference within 0.001°. `keep` (Bu uzaklığı koru): the
/// distance written as a lasting bond in the same step, a distance constraint from the reference to the edge with its
/// parameter, so that the edge follows when the reference moves; without `distance` the bond keeps the distance now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShiftReference {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub keep: bool,
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
    /// lengthen or shorten to meet it. With `topology` (Topolojik düzenleme, docs/adr/0160, 1960) the shown objects
    /// sharing the edge's moved corners and edges move with them in the same step: a neighbour parcel gives up what
    /// the edited one takes; one on a locked layer stays, the boundary split and said (`neighbour_locked`), or with
    /// `stopOnLocked` nothing is written (`shared_boundary_locked`); `points`: the points at the corners too. The
    /// neighbours' vertices inside the shared edge (a T junction, a neighbour's own vertex) go with it square to it
    /// (docs/adr/2600), refused when the shortened edge no longer reaches one or the edge is an arc with one inside. With
    /// `reference` (docs/adr/2580) `distance` is the distance from the reference's line, and `keep` writes it as a bond.
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        topology: Option<crate::StretchTopology>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        reference: Option<ShiftReference>,
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
    /// Parçala (docs/adr/0140): lines, open polylines, arcs and circles cut into separate objects, by `method`: where
    /// the given objects cross, into `parts` equal pieces (2 to 10 000), or into pieces `length` long from the start
    /// (`fromEnd`: from the end; a circle from its east point). The first piece keeps the object's place and id.
    #[serde(rename_all = "camelCase")]
    Split {
        ids: Vec<String>,
        method: SplitMethod,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        parts: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        length: Option<f64>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        from_end: bool,
    },
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
    /// Sürdür: the vertices added. Sadeleştir: the vertices dropped. Okunur yap: the texts turned. Parçala: the pieces.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    /// Paralel kaydır: the distance the edge moved. Sadeleştir: the largest deviation. Both in the drawing's unit.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
    /// Parçala: the new objects, the pieces after each object's first, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub created: Vec<String>,
    /// Paralel kaydır with `topology` (docs/adr/1960): the neighbours that moved with the edge, in the drawing's order,
    /// each with its size before and after (as `before` and `after`: an area's net area, a path's length, a point 0).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ShapeNeighbour>>"))]
    pub neighbours: Vec<ShapeNeighbour>,
    /// Paralel kaydır with a kept reference (docs/adr/2580): the bonds written (constraint ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub bonds: Vec<String>,
    /// Paralel kaydır with `topology` (docs/adr/2600): the places inside the shared edge where a neighbour had a vertex
    /// (a third object's corner on it, a neighbour's own vertex along it), carried with the edge.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub carried: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A neighbour that moved with Paralel kaydır's edge (docs/adr/1960): its id and its size before and after.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShapeNeighbour {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub before: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub after: f64,
}
