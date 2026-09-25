//! Drawing objects (`Entity` in `apps/web/src/model/entities.ts`), version 1.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A point in world units: x = east (Y, sağa), y = north (X, yukarı).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

/// Fields every object has. `id` is the document's local id in v1; the
/// server's stable id is a UUID in PostgreSQL, carried by a separate
/// `FeatureRef` in Faz B (CLAUDE.md §15).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntityBase {
    pub id: u32,
    pub layer_id: String,
    /// Colour override; absent = the layer's colour ("katmana göre").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// GIS attributes; text in v1 (typed attributes: CLAUDE.md §15).
    pub attrs: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// Library symbol overriding the layer's style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub p: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub a: Vec2,
    pub b: Vec2,
}

/// A closed ring in vertex + bulge form (a polygon hole).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RingGeometry {
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bulges: Option<Vec<f64>>,
}

/// Polyline or polygon: vertices, DXF bulges (tan(θ/4), CCW positive) and, for polygons, holes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PathEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bulges: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub holes: Option<Vec<RingGeometry>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CircleEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub c: Vec2,
    pub r: f64,
}

/// Arc from `a0` to `a1` (radians), always counter-clockwise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArcEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub c: Vec2,
    pub r: f64,
    pub a0: f64,
    pub a1: f64,
}

/// DXF ELLIPSE: centre, major axis vector, minor/major ratio, parameters t0 → t1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EllipseEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub c: Vec2,
    pub major: Vec2,
    pub ratio: f64,
    pub t0: f64,
    pub t1: f64,
}

/// Construction line or ray: base point and unit direction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstructionEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub p: Vec2,
    pub dir: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SplineEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub pts: Vec<Vec2>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub p: Vec2,
    pub text: String,
    /// Metres.
    pub height: f64,
    /// Degrees, counter-clockwise from east.
    pub rotation: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionStyle {
    Aligned,
    Linear,
    Angular,
    Radius,
    Diameter,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub a: Vec2,
    pub b: Vec2,
    pub offset: f64,
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<DimensionStyle>,
    /// Linear: measured direction in degrees (0 = ΔY, 90 = ΔX).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub angle: Option<f64>,
    /// Angular: the vertex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub c: Option<Vec2>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HatchPatternType {
    Solid,
    Lines,
    Cross,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchPattern {
    #[serde(rename = "type")]
    pub kind: HatchPatternType,
    /// Degrees, counter-clockwise from east.
    pub angle: f64,
    /// Metres.
    pub spacing: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub ring: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub holes: Option<Vec<Vec<Vec2>>>,
    pub pattern: HatchPattern,
}

/// Any drawing object, tagged by `kind` as in the TypeScript model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Entity {
    Point(PointEntity),
    Line(LineEntity),
    Polyline(PathEntity),
    Polygon(PathEntity),
    Circle(CircleEntity),
    Arc(ArcEntity),
    Ellipse(EllipseEntity),
    Spline(SplineEntity),
    Xline(ConstructionEntity),
    Ray(ConstructionEntity),
    Text(TextEntity),
    Dimension(DimensionEntity),
    Hatch(HatchEntity),
}

impl Entity {
    /// The fields every object has (id, layer, colour, attributes, label, symbol).
    pub fn base(&self) -> &EntityBase {
        match self {
            Entity::Point(e) => &e.base,
            Entity::Line(e) => &e.base,
            Entity::Polyline(e) | Entity::Polygon(e) => &e.base,
            Entity::Circle(e) => &e.base,
            Entity::Arc(e) => &e.base,
            Entity::Ellipse(e) => &e.base,
            Entity::Spline(e) => &e.base,
            Entity::Xline(e) | Entity::Ray(e) => &e.base,
            Entity::Text(e) => &e.base,
            Entity::Dimension(e) => &e.base,
            Entity::Hatch(e) => &e.base,
        }
    }

    /// The `kind` tag as written in files and on the wire.
    pub fn kind(&self) -> &'static str {
        match self {
            Entity::Point(_) => "point",
            Entity::Line(_) => "line",
            Entity::Polyline(_) => "polyline",
            Entity::Polygon(_) => "polygon",
            Entity::Circle(_) => "circle",
            Entity::Arc(_) => "arc",
            Entity::Ellipse(_) => "ellipse",
            Entity::Spline(_) => "spline",
            Entity::Xline(_) => "xline",
            Entity::Ray(_) => "ray",
            Entity::Text(_) => "text",
            Entity::Dimension(_) => "dimension",
            Entity::Hatch(_) => "hatch",
        }
    }
}
