//! Objects as the shared geometry core takes them, and back
//! (docs/adr/0029, 0037): the contract's [`Entity`] to the core's [`Shape`],
//! and a shape the core computed for an object back into that object. The
//! desktop's geometry store (`kentos_interaction::spatial`) and the
//! transform command read objects through here. Nothing is computed: each
//! field is carried over as it is, float64 bit for bit.

use kentos_contracts::{
    ArcEntity, CircleEntity, ConstructionEntity, DimensionStyle, EllipseEntity, Entity, EntityBase,
    EntityGeometry, HatchPatternType, LineEntity, PathEntity, PointEntity, RingGeometry,
    SplineEntity, TextEntity, Vec2 as Point,
};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::{HatchPattern, Shape};
use kentos_geometry_core::geom::arrangement::Ring;

fn v(p: &Point) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn points(pts: &[Point]) -> Vec<Vec2> {
    pts.iter().map(v).collect()
}

fn ring(r: &RingGeometry) -> Ring {
    Ring {
        pts: points(&r.pts),
        bulges: r.bulges.clone(),
    }
}

/// A dimension style as the core names it (as files write it).
fn style_name(style: DimensionStyle) -> &'static str {
    match style {
        DimensionStyle::Aligned => "aligned",
        DimensionStyle::Linear => "linear",
        DimensionStyle::Angular => "angular",
        DimensionStyle::Radius => "radius",
        DimensionStyle::Diameter => "diameter",
    }
}

fn pattern_name(kind: HatchPatternType) -> &'static str {
    match kind {
        HatchPatternType::Solid => "solid",
        HatchPatternType::Lines => "lines",
        HatchPatternType::Cross => "cross",
    }
}

/// An object's geometry as the geometry core takes it.
pub fn shape(entity: &Entity) -> Shape {
    match entity {
        Entity::Point(p) => Shape::Point { p: v(&p.p), z: p.z },
        Entity::Line(l) => Shape::Line {
            a: v(&l.a),
            b: v(&l.b),
        },
        Entity::Polyline(p) => Shape::Polyline {
            pts: points(&p.pts),
            bulges: p.bulges.clone(),
            holes: p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
        },
        Entity::Polygon(p) => Shape::Polygon {
            pts: points(&p.pts),
            bulges: p.bulges.clone(),
            holes: p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
        },
        Entity::Circle(c) => Shape::Circle { c: v(&c.c), r: c.r },
        Entity::Arc(a) => Shape::Arc {
            c: v(&a.c),
            r: a.r,
            a0: a.a0,
            a1: a.a1,
        },
        Entity::Ellipse(e) => Shape::Ellipse {
            c: v(&e.c),
            major: v(&e.major),
            ratio: e.ratio,
            t0: e.t0,
            t1: e.t1,
        },
        Entity::Spline(s) => Shape::Spline {
            pts: points(&s.pts),
            closed: s.closed,
        },
        Entity::Xline(c) => Shape::Xline {
            p: v(&c.p),
            dir: v(&c.dir),
        },
        Entity::Ray(c) => Shape::Ray {
            p: v(&c.p),
            dir: v(&c.dir),
        },
        Entity::Text(t) => Shape::Text {
            p: v(&t.p),
            text: t.text.clone(),
            height: t.height,
            rotation: t.rotation,
        },
        Entity::Dimension(d) => Shape::Dimension {
            a: v(&d.a),
            b: v(&d.b),
            offset: d.offset,
            height: d.height,
            text: d.text.clone(),
            style: d.style.map(|s| style_name(s).to_owned()),
            angle: d.angle,
            c: d.c.as_ref().map(v),
        },
        Entity::Hatch(h) => Shape::Hatch {
            ring: points(&h.ring),
            holes: h
                .holes
                .as_ref()
                .map(|hs| hs.iter().map(|r| points(r)).collect()),
            pattern: HatchPattern {
                kind: pattern_name(h.pattern.kind).to_owned(),
                angle: h.pattern.angle,
                spacing: h.pattern.spacing,
            },
        },
    }
}

fn p(v: Vec2) -> Point {
    Point { x: v.x, y: v.y }
}

fn back(pts: Vec<Vec2>) -> Vec<Point> {
    pts.into_iter().map(p).collect()
}

fn ring_back(r: Ring) -> RingGeometry {
    RingGeometry {
        pts: back(r.pts),
        bulges: r.bulges,
    }
}

/// `entity` with `shape` as its geometry: the core's answer for it (a moved,
/// rotated, scaled or mirrored copy of its own shape). Every other field is
/// kept, as the web's `withGeometry` keeps them; the geometry fields are the
/// shape's, float64 bit for bit. None when the shape is of another kind: the
/// core's transforms never change a kind.
pub fn with_shape(entity: &Entity, shape: Shape) -> Option<Entity> {
    let mut out = entity.clone();
    match (&mut out, shape) {
        (Entity::Point(e), Shape::Point { p: at, z }) => {
            e.p = p(at);
            e.z = z;
        }
        (Entity::Line(e), Shape::Line { a, b }) => {
            e.a = p(a);
            e.b = p(b);
        }
        (Entity::Polyline(e), Shape::Polyline { pts, bulges, holes })
        | (Entity::Polygon(e), Shape::Polygon { pts, bulges, holes }) => {
            e.pts = back(pts);
            e.bulges = bulges;
            e.holes = holes.map(|hs| hs.into_iter().map(ring_back).collect());
        }
        (Entity::Circle(e), Shape::Circle { c, r }) => {
            e.c = p(c);
            e.r = r;
        }
        (Entity::Arc(e), Shape::Arc { c, r, a0, a1 }) => {
            e.c = p(c);
            e.r = r;
            e.a0 = a0;
            e.a1 = a1;
        }
        (
            Entity::Ellipse(e),
            Shape::Ellipse {
                c,
                major,
                ratio,
                t0,
                t1,
            },
        ) => {
            e.c = p(c);
            e.major = p(major);
            e.ratio = ratio;
            e.t0 = t0;
            e.t1 = t1;
        }
        (Entity::Spline(e), Shape::Spline { pts, closed }) => {
            e.pts = back(pts);
            e.closed = closed;
        }
        (Entity::Xline(e), Shape::Xline { p: at, dir })
        | (Entity::Ray(e), Shape::Ray { p: at, dir }) => {
            e.p = p(at);
            e.dir = p(dir);
        }
        (
            Entity::Text(e),
            Shape::Text {
                p: at,
                text,
                height,
                rotation,
            },
        ) => {
            e.p = p(at);
            e.text = text;
            e.height = height;
            e.rotation = rotation;
        }
        (
            Entity::Dimension(e),
            Shape::Dimension {
                a,
                b,
                offset,
                height,
                text,
                style: _,
                angle,
                c,
            },
        ) => {
            // The style is the object's own: the core carries its name through unchanged.
            e.a = p(a);
            e.b = p(b);
            e.offset = offset;
            e.height = height;
            e.text = text;
            e.angle = angle;
            e.c = c.map(p);
        }
        (
            Entity::Hatch(e),
            Shape::Hatch {
                ring,
                holes,
                pattern,
            },
        ) => {
            // The pattern's type is the object's own: the core carries its name through unchanged.
            e.ring = back(ring);
            e.holes = holes.map(|hs| hs.into_iter().map(back).collect());
            e.pattern.angle = pattern.angle;
            e.pattern.spacing = pattern.spacing;
        }
        _ => return None,
    }
    Some(out)
}

/// An object of `geometry` with the fields every object has from `base`:
/// what `cad.entities.edit` writes (docs/adr/0047). A polyline has no holes.
pub fn entity_of(geometry: &EntityGeometry, base: EntityBase) -> Entity {
    match geometry.clone() {
        EntityGeometry::Point { p, z } => Entity::Point(PointEntity { base, p, z }),
        EntityGeometry::Line { a, b } => Entity::Line(LineEntity { base, a, b }),
        EntityGeometry::Polyline { pts, bulges } => Entity::Polyline(PathEntity {
            base,
            pts,
            bulges,
            holes: None,
        }),
        EntityGeometry::Polygon { pts, bulges, holes } => Entity::Polygon(PathEntity {
            base,
            pts,
            bulges,
            holes,
        }),
        EntityGeometry::Circle { c, r } => Entity::Circle(CircleEntity { base, c, r }),
        EntityGeometry::Arc { c, r, a0, a1 } => Entity::Arc(ArcEntity { base, c, r, a0, a1 }),
        EntityGeometry::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => Entity::Ellipse(EllipseEntity {
            base,
            c,
            major,
            ratio,
            t0,
            t1,
        }),
        EntityGeometry::Spline { pts, closed } => {
            Entity::Spline(SplineEntity { base, pts, closed })
        }
        EntityGeometry::Xline { p, dir } => Entity::Xline(ConstructionEntity { base, p, dir }),
        EntityGeometry::Ray { p, dir } => Entity::Ray(ConstructionEntity { base, p, dir }),
        EntityGeometry::Text {
            p,
            text,
            height,
            rotation,
        } => Entity::Text(TextEntity {
            base,
            p,
            text,
            height,
            rotation,
        }),
    }
}

/// A shape the core computed as `cad.entities.edit` takes it: its kind and
/// geometry fields, float64 bit for bit. None for a dimension or a hatch,
/// which the command does not write; a polyline's holes are left out.
pub fn edit_geometry(shape: Shape) -> Option<EntityGeometry> {
    Some(match shape {
        Shape::Point { p: at, z } => EntityGeometry::Point { p: p(at), z },
        Shape::Line { a, b } => EntityGeometry::Line { a: p(a), b: p(b) },
        Shape::Polyline { pts, bulges, .. } => EntityGeometry::Polyline {
            pts: back(pts),
            bulges,
        },
        Shape::Polygon { pts, bulges, holes } => EntityGeometry::Polygon {
            pts: back(pts),
            bulges,
            holes: holes.map(|hs| hs.into_iter().map(ring_back).collect()),
        },
        Shape::Circle { c, r } => EntityGeometry::Circle { c: p(c), r },
        Shape::Arc { c, r, a0, a1 } => EntityGeometry::Arc { c: p(c), r, a0, a1 },
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => EntityGeometry::Ellipse {
            c: p(c),
            major: p(major),
            ratio,
            t0,
            t1,
        },
        Shape::Spline { pts, closed } => EntityGeometry::Spline {
            pts: back(pts),
            closed,
        },
        Shape::Xline { p: at, dir } => EntityGeometry::Xline {
            p: p(at),
            dir: p(dir),
        },
        Shape::Ray { p: at, dir } => EntityGeometry::Ray {
            p: p(at),
            dir: p(dir),
        },
        Shape::Text {
            p: at,
            text,
            height,
            rotation,
        } => EntityGeometry::Text {
            p: p(at),
            text,
            height,
            rotation,
        },
        Shape::Dimension { .. } | Shape::Hatch { .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(json: &str) -> Entity {
        serde_json::from_str(json).expect("an object")
    }

    /// Every kind to the core's shape and back is the same object, bit for
    /// bit; a shape of another kind is refused.
    #[test]
    fn every_kind_goes_to_the_core_and_back_unchanged() {
        let objects = [
            r#"{"kind":"point","id":1,"layerId":"a","attrs":{"Ad":"P1"},"label":"P1","p":{"x":486512.34,"y":-0.0},"z":850.5}"#,
            r##"{"kind":"line","id":2,"layerId":"a","attrs":{},"color":"#E5484D","a":{"x":1,"y":2},"b":{"x":3,"y":4}}"##,
            r#"{"kind":"polyline","id":3,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":0}],"bulges":[0.5]}"#,
            r#"{"kind":"polygon","id":4,"layerId":"a","attrs":{"Ada":"104"},"symbol":"s","pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[{"pts":[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}],"bulges":[0,0,-0.1]}]}"#,
            r#"{"kind":"circle","id":5,"layerId":"a","attrs":{},"c":{"x":1,"y":2},"r":3}"#,
            r#"{"kind":"arc","id":6,"layerId":"a","attrs":{},"c":{"x":1,"y":2},"r":3,"a0":0.5,"a1":2}"#,
            r#"{"kind":"ellipse","id":7,"layerId":"a","attrs":{},"c":{"x":1,"y":2},"major":{"x":10,"y":-3},"ratio":0.4,"t0":1,"t1":2}"#,
            r#"{"kind":"spline","id":8,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":1},{"x":2,"y":0}],"closed":true}"#,
            r#"{"kind":"xline","id":9,"layerId":"a","attrs":{},"p":{"x":1,"y":2},"dir":{"x":0.6,"y":0.8}}"#,
            r#"{"kind":"ray","id":10,"layerId":"a","attrs":{},"p":{"x":1,"y":2},"dir":{"x":-1,"y":0}}"#,
            r#"{"kind":"text","id":11,"layerId":"a","attrs":{},"p":{"x":1,"y":2},"text":"Ada 104","height":2,"rotation":-30}"#,
            r#"{"kind":"dimension","id":12,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":-2,"height":0.5,"text":"12,5 m","style":"linear","angle":90}"#,
            r#"{"kind":"hatch","id":13,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"cross","angle":30,"spacing":0.5}}"#,
        ];
        for text in objects {
            let e = entity(text);
            let again = with_shape(&e, shape(&e)).expect("the same kind");
            assert_eq!(
                serde_json::to_string(&again).unwrap(),
                serde_json::to_string(&e).unwrap(),
                "{text}"
            );
        }
        let point = entity(objects[0]);
        assert_eq!(
            with_shape(&point, shape(&entity(objects[1]))),
            None,
            "a line is not a point"
        );
        // −0 is carried over as −0.
        let Some(Entity::Point(p)) = with_shape(&point, shape(&point)) else {
            panic!("a point");
        };
        assert!(p.p.y == 0.0 && p.p.y.is_sign_negative());
    }
}
