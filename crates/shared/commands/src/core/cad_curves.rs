//! The curve constructions of the product command catalog (docs/adr/0289; plan/cad.md CAD-EDIT-03): Daire's,
//! Yay's and Elips's methods asked for by what they are made of (two or three points, tangent objects, a start,
//! a centre, an angle, a chord, a radius, a direction, an object's end to continue from, axis ends), so that
//! Python and an agent take the same circle, arc or ellipse the tools take without working the geometry out
//! themselves. It writes nothing: the answer is what `cad.circle.create`, `cad.arc.create` and
//! `cad.entities.create`'s ellipse take.
//!
//! Points are metres on the wire, x east (Y), y north (X), whatever unit the project shows (docs/adr/0165 §2);
//! an angle is radians counter-clockwise from east (a geometry angle, not a surveying bearing).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::cad_dimensions::{EdgeNear, MeasuredPoint};
use crate::cad_points::ObjectEnd;
use crate::entity::Vec2;

/// Constructs circles, arcs and ellipses from what they are made of.
pub const CAD_CURVES_CONSTRUCT: &str = "cad.curves.construct";
pub const CAD_CURVES_CONSTRUCT_VERSION: u32 = 1;

/// One curve to construct: Daire's, Yay's and Elips's methods (AutoCAD's CIRCLE, ARC and ELLIPSE). A circle by
/// its centre and radius or diameter needs no construction: `cad.circle.create` takes it (r = diameter / 2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CurveConstruction {
    /// Daire, 2 nokta: the circle on the diameter from `a` to `b`.
    CircleTwoPoints { a: MeasuredPoint, b: MeasuredPoint },
    /// Daire, 3 nokta: the circle through three points.
    CircleThreePoints {
        a: MeasuredPoint,
        b: MeasuredPoint,
        c: MeasuredPoint,
    },
    /// Daire, teğet-teğet-yarıçap: the circle of radius `r` touching two objects' edges nearest their near
    /// points (a straight edge as its whole line, an arc as its whole circle), the one whose touching points are
    /// nearest the near points. Two parallel lines `2r` apart give the one across from the near points' middle.
    CircleTangentTangentRadius {
        first: EdgeNear,
        second: EdgeNear,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        r: f64,
    },
    /// Daire, teğet-teğet-teğet: the circle touching three objects' edges (Apollonius), the one whose touching
    /// points are nearest the near points.
    CircleTangents {
        first: EdgeNear,
        second: EdgeNear,
        third: EdgeNear,
    },
    /// Yay, 3 nokta: from `start` through `mid` to `end`.
    ArcThreePoints {
        start: MeasuredPoint,
        mid: MeasuredPoint,
        end: MeasuredPoint,
    },
    /// Yay, başlangıç–merkez–bitiş (and merkez–başlangıç–bitiş, başlangıç–bitiş–merkez): counter-clockwise
    /// from `start` round `centre` to the ray through `end`; the radius is the start's.
    ArcStartCentreEnd {
        start: MeasuredPoint,
        centre: MeasuredPoint,
        end: MeasuredPoint,
    },
    /// Yay, başlangıç–merkez–açı (and merkez–başlangıç–açı): `angle` radians round `centre` from `start`,
    /// counter-clockwise when positive; less than a whole turn.
    ArcStartCentreAngle {
        start: MeasuredPoint,
        centre: MeasuredPoint,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        angle: f64,
    },
    /// Yay, başlangıç–merkez–uzunluk (and merkez–başlangıç–uzunluk): counter-clockwise from `start` round
    /// `centre` until the chord is `chord` metres; a negative chord gives the major arc.
    ArcStartCentreChord {
        start: MeasuredPoint,
        centre: MeasuredPoint,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        chord: f64,
    },
    /// Yay, başlangıç–bitiş–açı: from `start` to `end` turning `angle` radians, counter-clockwise when positive.
    ArcStartEndAngle {
        start: MeasuredPoint,
        end: MeasuredPoint,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        angle: f64,
    },
    /// Yay, başlangıç–bitiş–yön: from `start`, leaving it at `direction` radians, to `end`.
    ArcStartEndDirection {
        start: MeasuredPoint,
        end: MeasuredPoint,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        direction: f64,
    },
    /// Yay, başlangıç–bitiş–yarıçap: from `start` to `end` with radius `|r|`, counter-clockwise; a positive
    /// radius gives the minor arc, a negative one the major.
    ArcStartEndRadius {
        start: MeasuredPoint,
        end: MeasuredPoint,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        r: f64,
    },
    /// Yay, Devam: tangent to a line, an arc or a polyline at its end (`place` end: on the way it runs; start:
    /// on backwards from its start), to `end`.
    ArcContinue { from: ObjectEnd, end: MeasuredPoint },
    /// Elips, eksen–uç: one axis from `a` to `b`, the other half-axis `other` metres long or the first axis seen
    /// tilted by `rotation` radians (0 to 89.4°): exactly one of the two. With `startAngle` and `endAngle`
    /// (both or neither; radians counter-clockwise from the major axis, as seen from the centre), an elliptical
    /// arc.
    EllipseAxis {
        a: MeasuredPoint,
        b: MeasuredPoint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        other: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        rotation: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        start_angle: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        end_angle: Option<f64>,
    },
    /// Elips, merkez: the centre, one axis's end, then as [`CurveConstruction::EllipseAxis`].
    EllipseCentre {
        centre: MeasuredPoint,
        axis_end: MeasuredPoint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        other: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        rotation: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        start_angle: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        end_angle: Option<f64>,
    },
}

/// A constructed curve, as the create commands take it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ConstructedCurve {
    /// `cad.circle.create`'s `c` and `r`.
    Circle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        c: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        r: f64,
    },
    /// `cad.arc.create`'s `c`, `r`, `a0` and `a1`: counter-clockwise from `a0` to `a1`, both in [0, 2π).
    Arc {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        c: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        r: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        a0: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        a1: f64,
    },
    /// `cad.entities.create`'s ellipse geometry: the centre, the major half-axis as a vector, the minor to major
    /// ratio (above zero, at most 1) and the parameters it runs between (equal: a whole ellipse).
    Ellipse {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        c: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        major: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        ratio: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        t0: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        t1: f64,
    },
}

/// Input of `cad.curves.construct` v1: the curves the constructions give, in their order. It reads no
/// selection, view or snap setting (TODOS.md CMD-07) and writes nothing.
///
/// Refusals (`CommandError.code`, at `constructions[i]…`), checked construction by construction in order:
/// `no_constructions` (none, or more than 10 000), `not_finite` (a coordinate, a near point, an angle, a
/// length), `invalid_uid`, `entity_not_found`, `no_such_place`, `piece_gone`; then by kind:
/// `same_points` (two points of it in one place), `collinear_points` (three points on a line, an end on the
/// start's tangent), `invalid_angle` (an angle of zero or a whole turn or more, an end on the start's ray, an
/// elliptical arc's start and end at one place, a rotation outside 0 to 89.4°, one of the arc's angles
/// alone), `invalid_radius` (not above zero), `radius_too_small` (less than half the chord), `invalid_distance`
/// (a chord of zero), `chord_too_long` (more than the diameter), `invalid_half_axis` (the other half-axis not
/// above zero, or both or neither of it and the rotation), `not_tangent_kind` (an object a circle cannot touch),
/// `no_tangent_circle` (no circle touches them so), `not_an_end` (an object Devam cannot continue).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CurvesConstruct {
    /// One to 10 000 constructions.
    pub constructions: Vec<CurveConstruction>,
}

/// Output of `cad.curves.construct` v1, and its plan: the curves, one per construction, in their order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CurvesConstructed {
    pub curves: Vec<ConstructedCurve>,
}
