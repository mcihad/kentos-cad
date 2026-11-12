//! Rectangles and regular polygons as product commands (docs/adr/0290; plan/cad.md CAD-EDIT-04): the caller gives
//! what the user gives — two corners, a corner with a size or an area, an edge with a width; a centre and a corner or
//! an edge's middle, or an edge — and the command computes the ring with the shared core and writes it as
//! `cad.polygon.create` would, one closed area, one undo step. The Dikdörtgen, Döndürülmüş dikdörtgen and Düzgün
//! çokgen tools write through them; Python and an agent ask the same.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Writes a rectangle.
pub const CAD_RECTANGLE_CREATE: &str = "cad.rectangle.create";
pub const CAD_RECTANGLE_CREATE_VERSION: u32 = 1;
/// Writes a regular polygon.
pub const CAD_POLYGON_REGULAR: &str = "cad.polygon.regular";
pub const CAD_POLYGON_REGULAR_VERSION: u32 = 1;

/// Writes a pie slice (Daire dilimi, docs/adr/0622 §9).
pub const CAD_SECTOR_CREATE: &str = "cad.sector.create";
pub const CAD_SECTOR_CREATE_VERSION: u32 = 1;

/// Writes a revision cloud (Revizyon bulutu, docs/adr/0622 §9).
pub const CAD_REVCLOUD_CREATE: &str = "cad.revcloud.create";
pub const CAD_REVCLOUD_CREATE_VERSION: u32 = 1;

/// Writes the region a path closes with the neighbouring areas (Bitişik alan, docs/adr/0162 §3, 0622 §9).
pub const CAD_ENTITIES_ADJOIN: &str = "cad.entities.adjoin";
pub const CAD_ENTITIES_ADJOIN_VERSION: u32 = 1;

/// Writes a parcel on the parcel layer with its number (Parsel oluştur, docs/adr/0067, 0622 §9).
pub const CAD_PARCELS_CREATE: &str = "cad.parcels.create";
pub const CAD_PARCELS_CREATE_VERSION: u32 = 1;
/// The layer parcels go on, opened as a new project has it when the drawing lacks it.
pub const PARCEL_LAYER: &str = "parsel";

/// The most arcs a revision cloud has.
pub const MOST_CLOUD_ARCS: usize = 10_000;

/// The most sides a regular polygon has.
pub const MOST_SIDES: u32 = 1024;

/// Where a rectangle lies. Its sides run along `rotation` and across it, but for `edge`, whose edge gives them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RectangleFrame {
    /// Two opposite corners (AutoCAD RECTANG's two points).
    Corners {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
    },
    /// A corner, the side along the rotation (`length`) and across it (`width`), opening toward `toward`'s quarter
    /// (RECTANG's Dimensions).
    Size {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        corner: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        length: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        width: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        toward: Vec2,
    },
    /// A corner, the area and the side along the rotation; the other side is the area over it (RECTANG's Area).
    Area {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        corner: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
        area: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        length: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        toward: Vec2,
    },
    /// An edge from `a` to `b` and the width to its left (a minus sign: to its right): Döndürülmüş dikdörtgen.
    Edge {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        width: f64,
    },
}

/// A rectangle's corners: sharp, rounded or cut, every one alike.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RectangleCorners {
    Sharp,
    /// Köşe yuvarla: an arc of this radius at every corner.
    Fillet {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        radius: f64,
    },
    /// Pah: every corner cut this far along both its sides.
    Chamfer {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
    },
}

/// Input of `cad.rectangle.create` v1. The ring is the shared core's (`rect_from_corners`, `rect_from_size`,
/// `rect_from_edge`, `corners_of_ring`); it is written as `cad.polygon.create` writes, its checks after the
/// rectangle's own.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite` (a point, a size, the rotation, a corner
/// size), `invalid_distance` (a size, an area, a width, a radius or a chamfer not above zero: none of them may be
/// zero but a sharp rectangle's), `degenerate_shape` (a side of no length: two corners on a line along the
/// rotation, an edge of no length), `invalid_corners` (the radius or the chamfer does not fit the sides), then
/// `cad.polygon.create`'s: `invalid_line_weight`, `invalid_revision`, `revision_conflict`, `layer_not_found`,
/// `not_a_layer`, `layer_locked`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RectangleCreate {
    /// The layer it goes on: a layer's id, not a group's.
    pub layer_id: String,
    pub frame: RectangleFrame,
    /// The direction of the sides along the rectangle, radians from east counter-clockwise; 0 when absent.
    /// Not read for `edge`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: Option<f64>,
    /// Sharp when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corners: Option<RectangleCorners>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<crate::layer::LineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    /// Çakışma and Topoloji, as `cad.polygon.create`'s (docs/adr/0622 §9). Absent: written as made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<crate::AreaRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Where a regular polygon lies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RegularFrame {
    /// Çember içinde: the centre and the first corner (AutoCAD POLYGON's Inscribed).
    Inscribed {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        center: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        corner: Vec2,
    },
    /// Çember dışında: the centre and the first edge's middle (POLYGON's Circumscribed).
    Circumscribed {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        center: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        middle: Vec2,
    },
    /// Kenardan: an edge from `a` to `b`, the polygon to its left, counter-clockwise (POLYGON's Edge).
    Edge {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
    },
}

/// Input of `cad.polygon.regular` v1. The ring is the shared core's (`regular_polygon`, `regular_polygon_on_edge`);
/// it is written as `cad.polygon.create` writes.
///
/// Refusals, in this order: `not_finite` (a point), `invalid_count` (sides out of 3–1024), `degenerate_shape` (the
/// two points meet), then `cad.polygon.create`'s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RegularPolygonCreate {
    pub layer_id: String,
    /// How many sides, 3–1024.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub sides: u32,
    pub frame: RegularFrame,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<crate::layer::LineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    /// Çakışma and Topoloji, as `cad.polygon.create`'s (docs/adr/0622 §9). Absent: written as made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<crate::AreaRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// `cad.sector.create`'s input (Daire dilimi): a closed pie slice by its centre, radius and the arc swept
/// counter-clockwise from `a0` to `a1`. The command makes the ring (the centre, the arc's start and end, the arc as the
/// middle edge) and writes it as `cad.polygon.create` writes an area; a sweep of nothing or of a whole turn, or a radius
/// not above zero, is refused (`degenerate_shape`, `invalid_distance`). No active layer, unit or remembered tool option
/// is read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectorCreate {
    pub layer_id: String,
    /// The centre: x east (Y), y north (X), in the project's units (m).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub c: Vec2,
    /// The radius, above zero.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub r: f64,
    /// Where the arc starts: radians counter-clockwise from east (a geometry angle, not a bearing).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub a0: f64,
    /// Where it ends: the slice is swept counter-clockwise from `a0` to `a1`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub a1: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<crate::layer::LineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    /// Çakışma and Topoloji, as `cad.polygon.create`'s (docs/adr/0622 §9). Absent: written as made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<crate::AreaRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What a revision cloud goes round (AutoCAD REVCLOUD's Rectangular, Polygonal and Object).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RevcloudFrame {
    /// Two opposite corners of a rectangle whose sides run east and north.
    Rectangle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
    },
    /// A polygon's corners in order, at least three; a corner within a nanometre of the one before it (or the last of
    /// the first) is dropped.
    Polygon {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        pts: Vec<Vec2>,
    },
    /// The outer outline of a closed object (an area, a closed polyline, a circle, a closed curve, a full ellipse) by
    /// its persistent id; its curves at 64 chords a turn. The object stays.
    Object { uid: String },
}

/// `cad.revcloud.create`'s input (Revizyon bulutu): the ring of `frame`, counter-clockwise, each side divided into
/// chords of about `arc` (`max(1, round(side / arc))` of them, equal), every chord an outward arc of bulge 0.5; written
/// as `cad.polygon.create` writes an area, one undo step. No active layer, plot scale or remembered tool option is
/// read: the tool turns its paper millimetres into `arc`.
///
/// Refusals, checked in this order: `not_finite` (a corner, `arc`), `invalid_distance` (`arc` not above zero),
/// `degenerate_shape` (a rectangle of no width or height, corners with no area), `too_few_corners` (fewer than three
/// corners), `entity_not_found` (the object), `not_closed` (an open object), `too_many_points` (more than 10 000
/// arcs), then `cad.polygon.create`'s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RevcloudCreate {
    pub layer_id: String,
    pub frame: RevcloudFrame,
    /// The arcs' chord, in the drawing's length (m), above zero.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub arc: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<crate::layer::LineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    /// Çakışma and Topoloji, as `cad.polygon.create`'s (docs/adr/0622 §9). Absent: written as made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<crate::AreaRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A box on the drawing: its south-west and north-east corners.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AdjoinWindow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max: Vec2,
}

/// `cad.entities.adjoin`'s input (Bitişik alan): only the new boundary is given, an open path from inside or on a
/// neighbouring area to another; the region it closes with the shown areas of `neighbours` (their boundaries taken
/// as its own) is the new area, written as `cad.polygon.create` writes one, its parts and holes as they are, one undo
/// step “Bitişik alan”. `within` keeps the neighbours to those touching a box (the tool's view); absent, every shown
/// area of the layers.
///
/// Refusals, in this order: `too_few_points` (fewer than two), `not_finite` (a point, a bulge, the box),
/// `bulge_count` (not one a segment), `layer_not_found` and `not_a_layer` (a neighbours' layer), `no_region` (the
/// path closes nothing), then `cad.polygon.create`'s (its layer, revision, line weight, attributes and `rules`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesAdjoin {
    pub layer_id: String,
    /// The new boundary's points, at least two.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// One bulge a segment (DXF's: tan of a quarter of the arc's angle); absent: every segment straight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    /// The layers whose shown areas close the region; empty: `layerId`'s own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub neighbours: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub within: Option<AdjoinWindow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<crate::layer::LineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    /// Çakışma (beyond the neighbours) and Topoloji, as `cad.polygon.create`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<crate::AreaRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// `cad.parcels.create`'s input (Parsel oluştur): a closed area on the `parsel` layer, whatever layer is active,
/// numbered by the command: `number` when given (its spaces trimmed; one another parcel has is warned of,
/// `parcel_number_taken`), else one above the greatest whole number the parcels' Parsel values start with. Its
/// attributes are Ada, Parsel, Mahalle, Nitelik “Arsa”, Tapu alanı (m²) and Pafta, the given ones over them, Parsel
/// the number; its label the number. The deed area stays empty: it is the title deed's, not the drawing's. A drawing
/// without the layer gets it in the parcel's undo step. Written as `cad.polygon.create` writes an area, `rules`
/// included.
///
/// Refusals, in this order: `invalid_parcel_number` (an empty number), `layer_locked` (the parcel layer), then
/// `cad.polygon.create`'s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsCreate {
    /// Its corners, at least three.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// One bulge an edge, the closing one too; absent: every edge straight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<crate::layer::LineType>,
    /// Çakışma and Topoloji, as `cad.polygon.create`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<crate::AreaRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// `cad.parcels.create`'s output: the parcel written, its number and whether the parcel layer was opened for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelCreated {
    pub uid: String,
    /// Its slot in the open document (`Entity.id`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub id: u32,
    pub number: String,
    pub opened: bool,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// `cad.parcels.create`'s plan: the parcel as it would be stored (id 0), its number and whether the layer would be
/// opened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelPlan {
    pub entity: crate::Entity,
    pub number: String,
    pub opened: bool,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
