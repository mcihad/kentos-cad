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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}
