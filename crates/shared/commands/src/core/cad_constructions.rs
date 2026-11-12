//! The construction helpers as one product command (docs/adr/0622 §9, TODOS.md `UX-14`): Paralel çizgi, Dik in, Dik
//! çık, İki daireye teğet, Dördüncü köşe, Menzil halkaları, Ara nokta, Kesişim noktası and the construction line's
//! Açıortay and Paralel. The caller gives what the user gives (points, distances, bearings, circles); the command
//! computes the objects with the shared core and writes them as `cad.entities.create` writes, one undo step named
//! after the tool. The tools write through it; Python and an agent ask the same.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Writes construction objects.
pub const CAD_CONSTRUCTIONS_CREATE: &str = "cad.constructions.create";
pub const CAD_CONSTRUCTIONS_CREATE_VERSION: u32 = 1;

/// What a perpendicular is dropped onto (Dik in): a straight edge from `a` (its start) to `b`, taken as its whole
/// line, or a circle, the perpendicular then running through its centre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PerpendicularReference {
    Line {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
    },
    Circle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        c: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        r: f64,
    },
}

/// Dik çık's foot along the line (dik ayak, from its start) and the length square to it (dik boy, right plus).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PerpendicularStation {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub along: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: f64,
}

/// A circle, or an arc of it (`a0` to `a1`, radians counter-clockwise from east).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstructionRound {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub c: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub r: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub a0: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub a1: Option<f64>,
}

/// How Kesişim noktası finds its point.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "method", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Meeting {
    /// İki uzaklık: `da` from `a` and `db` from `b`; of two points the one nearest `near`, else the one right of a→b.
    Distances {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        da: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        db: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        near: Option<Vec2>,
    },
    /// İki doğrultu: from `a` along the survey bearing `ta` and from `b` along `tb` (radians from grid north,
    /// clockwise); the meeting ahead of both.
    Bearings {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        ta: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        tb: f64,
    },
    /// İki doğru: the line through `a` and `b` and the one through `c` and `d`, wherever they extend to.
    Lines {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        c: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        d: Vec2,
    },
}

/// The construction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Construction {
    /// Paralel çizgi: the sides of an axis at `left` and `right` (miter joins), as lines or (`area`) as the corridor
    /// between them; with `withAxis` the axis too. A closed axis makes closed sides.
    #[serde(rename_all = "camelCase")]
    Parallel {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        axis: Vec<Vec2>,
        #[serde(default)]
        closed: bool,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        left: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        right: f64,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        with_axis: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        area: bool,
    },
    /// Dik in: from each point a line to its foot on the reference.
    PerpendicularIn {
        reference: PerpendicularReference,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        points: Vec<Vec2>,
    },
    /// Dik çık: on the line from `a` to `b`, a line from each station's foot square to it.
    PerpendicularOut {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
        stations: Vec<PerpendicularStation>,
    },
    /// İki daireye teğet: the common tangent of two circles or arcs nearest the two points.
    #[serde(rename_all = "camelCase")]
    TangentLine {
        first: ConstructionRound,
        second: ConstructionRound,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        near_first: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        near_second: Vec2,
    },
    /// Dördüncü köşe: the parallelogram's corner opposite `b`, a point, or (`area`) the parallelogram.
    FourthCorner {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        c: Vec2,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        area: bool,
    },
    /// Menzil halkaları: `count` rings at `spacing` round `centre`, and `rays` from it to the outer one, the first due
    /// north.
    RangeRings {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        centre: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        spacing: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<u32>"))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        rays: u32,
    },
    /// Ara nokta: points on the line from `a` to `b`: in `parts` equal parts, at `distances` from `a`, or at `ratios`
    /// of the way (one of them).
    PointsBetween {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        parts: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distances: Option<Vec<f64>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        ratios: Option<Vec<f64>>,
    },
    /// Kesişim noktası: one point.
    Intersection { meeting: Meeting },
    /// Yardımcı çizgi, Açıortay: the construction line through `vertex` halving the angle between the arms to `p1` and
    /// `p2` (square to opposite arms).
    Bisector {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        vertex: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        p1: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        p2: Vec2,
    },
    /// Yardımcı çizgi, Paralel: the construction line parallel to the edge `a`→`b`, `distance` from it on `side`'s
    /// side, or through `side` without a distance.
    XlineOffset {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        side: Vec2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: Option<f64>,
    },
}

/// `cad.constructions.create`'s input: the construction, the layer and the look its objects take.
///
/// Refusals, in this order: `not_finite` (a point or a number), `invalid_distance` (a side, a radius, a spacing not
/// above zero where one is needed), `invalid_count` (parts, rings or rays out of range), `degenerate_shape` (points
/// that fall together or lie on a line where they must not, a reference of no length, a centre given as the point),
/// `no_intersection` (no tangent, no meeting), then `cad.entities.create`'s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstructionsCreate {
    pub layer_id: String,
    pub construction: Construction,
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
    /// Each object's label (an object template's, docs/adr/0176 §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// Each object's symbol from the style library (an object template's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}
