//! The point constructions of the product command catalog (docs/adr/0270; plan/cad.md CAD-INPUT-06): the places
//! the drafting aids give a click (İki nokta ortası, Uzantı, Paralel, Nesne izleme, a polar point, a distance
//! along) asked for by what they are made of, so that Python and an agent take the same point the tools take
//! without working the geometry out themselves. It writes nothing.
//!
//! Points are metres on the wire, x east (Y), y north (X), whatever unit the project shows (docs/adr/0165 §2);
//! an angle is radians counter-clockwise from the drawing's UCS x axis (the world's east without one,
//! docs/adr/0261), as the catalog's `conventions` say.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::UID_TEXT;
use crate::cad_dimensions::{EdgeNear, MeasuredPoint};
use crate::entity::Vec2;
use crate::subref::SubPlace;

/// Constructs points from what they are made of.
pub const CAD_POINTS_CONSTRUCT: &str = "cad.points.construct";
pub const CAD_POINTS_CONSTRUCT_VERSION: u32 = 1;

/// The most constructions one call takes.
pub const MAX_CONSTRUCTIONS: usize = 10_000;

/// An object's end an extension starts from: a line's or an arc's start or end, an open path's first or last
/// vertex (as a place, docs/adr/0249); inside an insert, the piece's (docs/adr/0263 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ObjectEnd {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub place: SubPlace,
    /// When `uid` is an insert, the piece inside it the end is on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub path: Vec<u32>,
}

/// One point to construct.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PointConstruction {
    /// İki nokta ortası: halfway from `a` to `b`.
    Mid { a: MeasuredPoint, b: MeasuredPoint },
    /// Uzantı: `distance` metres (zero or more) past an object's end along its edge: a straight edge's line,
    /// an arc's circle (the distance along the circle).
    Extension {
        end: ObjectEnd,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
    },
    /// Paralel: from `through`, `distance` metres (signed) along the object's straight edge nearest
    /// `edge.near`, the way the edge runs from its first point to its second.
    Parallel {
        through: MeasuredPoint,
        edge: EdgeNear,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
    },
    /// Nesne izleme: the UCS x of `x`'s point and the UCS y of `y`'s, where the alignments through the two
    /// cross (AutoCAD's .X and .Y point filters).
    Track { x: MeasuredPoint, y: MeasuredPoint },
    /// From `from`, `distance` metres at `angle`.
    Polar {
        from: MeasuredPoint,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        angle: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
    },
    /// Hat üzerinde: `distance` metres (signed) from `a` towards `b`.
    Along {
        a: MeasuredPoint,
        b: MeasuredPoint,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
    },
}

/// Input of `cad.points.construct` v1: the points the constructions give, in their order. It reads no
/// selection, view or snap setting (TODOS.md CMD-07) and writes nothing.
///
/// Refusals (`CommandError.code`, at `constructions[i]…`), checked construction by construction in order:
/// `no_constructions` (none, or more than 10 000), `not_finite` (a coordinate, the near point, an angle or a
/// distance), `invalid_distance` (an extension's below zero), `invalid_uid`, `entity_not_found`,
/// `no_such_place`, `piece_gone`, `not_an_end` (an extension's place is not an open end: a closed path's vertex,
/// an inner vertex, a circle), `no_straight_edge` (Paralel's object has none), `same_points` (Hat üzerinde's two
/// points in one place).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointsConstruct {
    /// One to 10 000 constructions.
    pub constructions: Vec<PointConstruction>,
}

/// Output of `cad.points.construct` v1, and its plan: the points, one per construction, in their order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointsConstructed {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub points: Vec<Vec2>,
}
