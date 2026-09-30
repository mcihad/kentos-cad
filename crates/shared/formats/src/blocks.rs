//! Blocks for the formats that have none (docs/adr/0144 §5): an insert as
//! its block's objects placed by the shared core (`kentos_geometry_core::block`:
//! nested blocks opened, quarter turns exact), each back as the contract's
//! object of its kind. Objects cross field for field, as the native
//! application's conversion does (no JSON in between: the formats module is
//! downloaded with every drawing a page opens, CLAUDE.md §20). The core's
//! pieces are flat: a placed piece has no elevation but a point's.

use kentos_contracts::{
    ArcEntity, AreaPart, BlockDefinition, CircleEntity, ConstructionEntity, DimensionEntity,
    DimensionStyle, EllipseEntity, Entity, EntityBase, HatchEntity, HatchPattern, HatchPatternType,
    InsertEntity, LineEntity, PathEntity, PointEntity, RingGeometry, SplineEntity, TextEntity,
    Vec2,
};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::block::{Blocks, Definition};
use kentos_geometry_core::entity::{
    Entity as CoreEntity, HatchPattern as CorePattern, Part, Shape,
};
use kentos_geometry_core::geom::arrangement::Ring;

/// A drawing's blocks, flattened once, ready to place inserts.
pub struct Placing(Blocks);

impl Placing {
    pub fn new(blocks: &[BlockDefinition]) -> Placing {
        Placing(Blocks::new(
            blocks
                .iter()
                .map(|b| Definition {
                    id: b.id.to_text(),
                    base: core(b.base),
                    entities: b
                        .entities
                        .iter()
                        .map(|e| CoreEntity::new(shape(e)))
                        .collect(),
                })
                .collect(),
        ))
    }

    /// An insert's objects placed, in its block's order; none for a block
    /// the drawing does not have. Each has no id, layer or attribute of its
    /// own (the insert's object carries them).
    pub fn placed(&self, i: &InsertEntity) -> Vec<Entity> {
        self.0
            .expand(&shape(&Entity::Insert(i.clone())))
            .iter()
            .filter_map(|piece| entity(&piece.shape))
            .collect()
    }
}

fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

fn back(p: CoreVec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

fn points(pts: &[Vec2]) -> Vec<CoreVec2> {
    pts.iter().map(|p| core(*p)).collect()
}

fn points_back(pts: &[CoreVec2]) -> Vec<Vec2> {
    pts.iter().map(|p| back(*p)).collect()
}

fn ring(r: &RingGeometry) -> Ring {
    Ring {
        pts: points(&r.pts),
        bulges: r.bulges.clone(),
    }
}

fn ring_back(r: &Ring) -> RingGeometry {
    RingGeometry {
        pts: points_back(&r.pts),
        bulges: r.bulges.clone(),
        zs: None,
    }
}

fn style_name(s: DimensionStyle) -> &'static str {
    match s {
        DimensionStyle::Aligned => "aligned",
        DimensionStyle::Linear => "linear",
        DimensionStyle::Angular => "angular",
        DimensionStyle::Radius => "radius",
        DimensionStyle::Diameter => "diameter",
    }
}

fn style_of(name: &str) -> Option<DimensionStyle> {
    Some(match name {
        "aligned" => DimensionStyle::Aligned,
        "linear" => DimensionStyle::Linear,
        "angular" => DimensionStyle::Angular,
        "radius" => DimensionStyle::Radius,
        "diameter" => DimensionStyle::Diameter,
        _ => return None,
    })
}

fn pattern_name(k: HatchPatternType) -> &'static str {
    match k {
        HatchPatternType::Solid => "solid",
        HatchPatternType::Lines => "lines",
        HatchPatternType::Cross => "cross",
    }
}

fn pattern_of(name: &str) -> HatchPatternType {
    match name {
        "solid" => HatchPatternType::Solid,
        "cross" => HatchPatternType::Cross,
        _ => HatchPatternType::Lines,
    }
}

/// An object's geometry as the core takes it.
fn shape(e: &Entity) -> Shape {
    let path = |p: &PathEntity| {
        (
            points(&p.pts),
            p.bulges.clone(),
            p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
        )
    };
    match e {
        Entity::Point(p) => Shape::Point {
            p: core(p.p),
            z: p.z,
        },
        Entity::Line(l) => Shape::Line {
            a: core(l.a),
            b: core(l.b),
        },
        Entity::Polyline(p) => {
            let (pts, bulges, holes) = path(p);
            Shape::Polyline { pts, bulges, holes }
        }
        Entity::Polygon(p) => {
            let (pts, bulges, holes) = path(p);
            Shape::Polygon {
                pts,
                bulges,
                holes,
                parts: p.parts.as_ref().map(|ps| {
                    ps.iter()
                        .map(|part| Part {
                            pts: points(&part.pts),
                            bulges: part.bulges.clone(),
                            holes: part.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
                        })
                        .collect()
                }),
            }
        }
        Entity::Circle(c) => Shape::Circle {
            c: core(c.c),
            r: c.r,
        },
        Entity::Arc(a) => Shape::Arc {
            c: core(a.c),
            r: a.r,
            a0: a.a0,
            a1: a.a1,
        },
        Entity::Ellipse(e) => Shape::Ellipse {
            c: core(e.c),
            major: core(e.major),
            ratio: e.ratio,
            t0: e.t0,
            t1: e.t1,
        },
        Entity::Spline(s) => Shape::Spline {
            pts: points(&s.pts),
            closed: s.closed,
        },
        Entity::Xline(x) => Shape::Xline {
            p: core(x.p),
            dir: core(x.dir),
        },
        Entity::Ray(x) => Shape::Ray {
            p: core(x.p),
            dir: core(x.dir),
        },
        Entity::Text(t) => Shape::Text {
            p: core(t.p),
            text: t.text.clone(),
            height: t.height,
            rotation: t.rotation,
        },
        Entity::Dimension(d) => Shape::Dimension {
            a: core(d.a),
            b: core(d.b),
            offset: d.offset,
            height: d.height,
            text: d.text.clone(),
            style: d.style.map(|s| style_name(s).to_owned()),
            angle: d.angle,
            c: d.c.map(core),
        },
        Entity::Hatch(h) => Shape::Hatch {
            ring: points(&h.ring),
            holes: h
                .holes
                .as_ref()
                .map(|hs| hs.iter().map(|r| points(r)).collect()),
            pattern: CorePattern {
                kind: pattern_name(h.pattern.kind).to_owned(),
                angle: h.pattern.angle,
                spacing: h.pattern.spacing,
            },
        },
        Entity::Insert(i) => Shape::Insert {
            block: i.block.to_text(),
            p: core(i.p),
            scale: i.scale,
            rotation: i.rotation,
            mirror: i.mirror.then_some(true),
        },
    }
}

/// A placed piece as the contract's object (never an insert: the core opens them).
fn entity(s: &Shape) -> Option<Entity> {
    let base = EntityBase {
        id: 0,
        layer_id: String::new(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
    };
    let path =
        |base, pts: &[CoreVec2], bulges: &Option<Vec<f64>>, holes: &Option<Vec<Ring>>| PathEntity {
            base,
            pts: points_back(pts),
            bulges: bulges.clone(),
            holes: holes.as_ref().map(|hs| hs.iter().map(ring_back).collect()),
            zs: None,
            parts: None,
        };
    Some(match s {
        Shape::Point { p, z } => Entity::Point(PointEntity {
            base,
            p: back(*p),
            z: *z,
        }),
        Shape::Line { a, b } => Entity::Line(LineEntity {
            base,
            a: back(*a),
            b: back(*b),
            za: None,
            zb: None,
        }),
        Shape::Polyline { pts, bulges, holes } => Entity::Polyline(path(base, pts, bulges, holes)),
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => Entity::Polygon(PathEntity {
            parts: parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|part| AreaPart {
                        pts: points_back(&part.pts),
                        bulges: part.bulges.clone(),
                        holes: part
                            .holes
                            .as_ref()
                            .map(|hs| hs.iter().map(ring_back).collect()),
                        zs: None,
                    })
                    .collect()
            }),
            ..path(base, pts, bulges, holes)
        }),
        Shape::Circle { c, r } => Entity::Circle(CircleEntity {
            base,
            c: back(*c),
            r: *r,
        }),
        Shape::Arc { c, r, a0, a1 } => Entity::Arc(ArcEntity {
            base,
            c: back(*c),
            r: *r,
            a0: *a0,
            a1: *a1,
        }),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => Entity::Ellipse(EllipseEntity {
            base,
            c: back(*c),
            major: back(*major),
            ratio: *ratio,
            t0: *t0,
            t1: *t1,
        }),
        Shape::Xline { p, dir } => Entity::Xline(ConstructionEntity {
            base,
            p: back(*p),
            dir: back(*dir),
        }),
        Shape::Ray { p, dir } => Entity::Ray(ConstructionEntity {
            base,
            p: back(*p),
            dir: back(*dir),
        }),
        Shape::Spline { pts, closed } => Entity::Spline(SplineEntity {
            base,
            pts: points_back(pts),
            closed: *closed,
        }),
        Shape::Text {
            p,
            text,
            height,
            rotation,
        } => Entity::Text(TextEntity {
            base,
            p: back(*p),
            text: text.clone(),
            height: *height,
            rotation: *rotation,
        }),
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            text,
            style,
            angle,
            c,
        } => Entity::Dimension(DimensionEntity {
            base,
            a: back(*a),
            b: back(*b),
            offset: *offset,
            height: *height,
            text: text.clone(),
            style: style.as_deref().and_then(style_of),
            angle: *angle,
            c: c.map(back),
        }),
        Shape::Hatch {
            ring,
            holes,
            pattern,
        } => Entity::Hatch(HatchEntity {
            base,
            ring: points_back(ring),
            holes: holes
                .as_ref()
                .map(|hs| hs.iter().map(|r| points_back(r)).collect()),
            pattern: HatchPattern {
                kind: pattern_of(&pattern.kind),
                angle: pattern.angle,
                spacing: pattern.spacing,
            },
        }),
        Shape::Insert { .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::BlockId;

    fn object(json: &str) -> Entity {
        serde_json::from_str(json).expect("an object")
    }

    /// Every kind crosses to the core and back unchanged (a placed piece is flat: no elevations but a point's).
    #[test]
    fn every_kind_crosses_and_comes_back() {
        for json in [
            r#"{"kind":"point","id":0,"layerId":"","attrs":{},"p":{"x":1,"y":2},"z":3.5}"#,
            r#"{"kind":"line","id":0,"layerId":"","attrs":{},"a":{"x":1,"y":2},"b":{"x":3,"y":4}}"#,
            r#"{"kind":"polyline","id":0,"layerId":"","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":0}],"bulges":[0.5,0]}"#,
            r#"{"kind":"polygon","id":0,"layerId":"","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[{"pts":[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]}],"parts":[{"pts":[{"x":9,"y":0},{"x":10,"y":0},{"x":10,"y":1}],"bulges":[0,0.25,0]}]}"#,
            r#"{"kind":"circle","id":0,"layerId":"","attrs":{},"c":{"x":1,"y":2},"r":3}"#,
            r#"{"kind":"arc","id":0,"layerId":"","attrs":{},"c":{"x":1,"y":2},"r":3,"a0":0.5,"a1":2}"#,
            r#"{"kind":"ellipse","id":0,"layerId":"","attrs":{},"c":{"x":1,"y":2},"major":{"x":3,"y":0},"ratio":0.5,"t0":0,"t1":1}"#,
            r#"{"kind":"spline","id":0,"layerId":"","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":1},{"x":2,"y":0}],"closed":false}"#,
            r#"{"kind":"xline","id":0,"layerId":"","attrs":{},"p":{"x":1,"y":2},"dir":{"x":0,"y":1}}"#,
            r#"{"kind":"ray","id":0,"layerId":"","attrs":{},"p":{"x":1,"y":2},"dir":{"x":1,"y":0}}"#,
            r#"{"kind":"text","id":0,"layerId":"","attrs":{},"p":{"x":1,"y":2},"text":"Ağaç","height":2.5,"rotation":30}"#,
            r#"{"kind":"dimension","id":0,"layerId":"","attrs":{},"a":{"x":0,"y":0},"b":{"x":5,"y":0},"offset":1,"height":0.5,"text":"5.00","style":"linear","angle":0}"#,
            r#"{"kind":"hatch","id":0,"layerId":"","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"cross","angle":45,"spacing":0.75}}"#,
        ] {
            let e = object(json);
            assert_eq!(entity(&shape(&e)), Some(e), "{json}");
        }
    }

    /// An insert of a block holding a point and a nested block: placed by the
    /// core, mirrored and turned a quarter (exact), the nested one opened.
    #[test]
    fn an_insert_is_its_blocks_objects_placed() {
        let id = |n: u8| BlockId([n; 16]);
        let block = |n: u8, base: (f64, f64), entities: Vec<Entity>| BlockDefinition {
            id: id(n),
            name: format!("B{n}"),
            base: Vec2 {
                x: base.0,
                y: base.1,
            },
            entities,
            attributes: Vec::new(),
            description: None,
        };
        let inner = block(
            2,
            (0.0, 0.0),
            vec![object(
                r#"{"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":0,"y":0},"b":{"x":1,"y":0}}"#,
            )],
        );
        let outer = block(
            1,
            (1.0, 2.0),
            vec![
                object(r#"{"kind":"point","id":1,"layerId":"","attrs":{},"p":{"x":1,"y":3}}"#),
                Entity::Insert(InsertEntity {
                    base: object(
                        r#"{"kind":"point","id":2,"layerId":"","attrs":{},"p":{"x":0,"y":0}}"#,
                    )
                    .base()
                    .clone(),
                    block: id(2),
                    p: Vec2 { x: 1.0, y: 2.0 },
                    scale: 2.0,
                    rotation: 0.0,
                    mirror: false,
                }),
            ],
        );
        let placing = Placing::new(&[outer, inner]);
        let i = InsertEntity {
            base: object(r#"{"kind":"point","id":7,"layerId":"a","attrs":{},"p":{"x":0,"y":0}}"#)
                .base()
                .clone(),
            block: id(1),
            p: Vec2 { x: 100.0, y: 200.0 },
            scale: 3.0,
            rotation: std::f64::consts::FRAC_PI_2,
            mirror: true,
        };
        // (1, 3) is (0, 1) from the base: mirrored (0, −1), scaled (0, −3), a quarter turn (3, 0).
        // The nested line (0, 0)–(2, 0) from the base: mirrored and scaled (0, 0)–(6, 0), turned (0, 0)–(0, 6).
        let got = placing.placed(&i);
        assert_eq!(
            got,
            [
                object(r#"{"kind":"point","id":0,"layerId":"","attrs":{},"p":{"x":103,"y":200}}"#),
                object(
                    r#"{"kind":"line","id":0,"layerId":"","attrs":{},"a":{"x":100,"y":200},"b":{"x":100,"y":206}}"#
                ),
            ]
        );
        let unknown = InsertEntity { block: id(9), ..i };
        assert!(placing.placed(&unknown).is_empty());
    }
}
