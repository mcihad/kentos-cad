//! Objects as the shared geometry core takes them, and back
//! (docs/adr/0029, 0037): the contract's [`Entity`] to the core's [`Shape`],
//! and a shape the core computed for an object back into that object. The
//! desktop's geometry store (`kentos_interaction::spatial`) and the
//! transform command read objects through here. Nothing is computed: each
//! field is carried over as it is, float64 bit for bit.

use kentos_contracts::{
    ArcEntity, AreaPart, BlockId, CircleEntity, ConstructionEntity, DimensionEntity,
    DimensionStyle, DrawingFont, EllipseEntity, Entity, EntityBase, EntityGeometry, HatchEntity,
    HatchPattern as ContractPattern, HatchPatternType, InsertEntity, LeaderArrow, LeaderEntity,
    LineEntity, PathEntity, PointEntity, RingGeometry, SplineEntity, TextEntity, Vec2 as Point,
};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::{Attrs, HatchPattern, Part, Shape};
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::text::{Font, TextAlign};

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

/// A part of a multi-part area as the core takes it (docs/adr/0143).
fn part(p: &AreaPart) -> Part {
    Part {
        pts: points(&p.pts),
        bulges: p.bulges.clone(),
        holes: p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
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

/// A dimension style by the name the core carries; None for a name the contract does not know.
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

/// A hatch pattern's type by the name the core carries; None for a name the contract does not know.
fn pattern_of(name: &str) -> Option<HatchPatternType> {
    Some(match name {
        "solid" => HatchPatternType::Solid,
        "lines" => HatchPatternType::Lines,
        "cross" => HatchPatternType::Cross,
        _ => return None,
    })
}

/// The drawing's typeface as the geometry core measures text in it (the
/// project's `drawingFont`; Barlow without one): what text boxes, the
/// geometry store and a polar array's middle are measured in.
pub fn drawing_font(font: Option<DrawingFont>) -> Font {
    Font::from_id(match font {
        None | Some(DrawingFont::Barlow) => "barlow",
        Some(DrawingFont::Arimo) => "arimo",
        Some(DrawingFont::Overpass) => "overpass",
        Some(DrawingFont::Quicksand) => "quicksand",
        Some(DrawingFont::ArchitectsDaughter) => "architects-daughter",
        Some(DrawingFont::CourierPrime) => "courier-prime",
        Some(DrawingFont::PlexMono) => "plex-mono",
    })
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
            parts: p.parts.as_ref().map(|ps| ps.iter().map(part).collect()),
        },
        Entity::Circle(c) => Shape::Circle { c: v(&c.c), r: c.r },
        Entity::Leader(l) => Shape::Leader {
            pts: points(&l.pts),
            text: l.text.clone(),
            height: l.height,
            rotation: l.rotation,
            arrow: l.arrow.map(|a| a.name().to_owned()),
            mask: l.mask.then_some(true),
        },
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
            align: t.align.and_then(|a| TextAlign::from_name(a.name())),
            width_factor: t.width_factor,
            mask: t.mask.then_some(true),
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
        // Its attributes too: what its block's attribute texts show (docs/adr/0144 §7).
        Entity::Insert(i) => Shape::Insert {
            block: i.block.to_text(),
            p: v(&i.p),
            scale: i.scale,
            rotation: i.rotation,
            mirror: i.mirror.then_some(true),
            attrs: Some(Attrs(
                i.base
                    .attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), Json::Str(v.clone())))
                    .collect(),
            )),
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
        zs: None,
    }
}

/// The core's holes back, each keeping the elevations of the hole it was
/// when its vertex count is the same (a transform moves each vertex and
/// keeps it; docs/adr/0142).
fn holes_back(
    holes: Option<Vec<Ring>>,
    before: Option<&[RingGeometry]>,
) -> Option<Vec<RingGeometry>> {
    holes.map(|hs| {
        hs.into_iter()
            .enumerate()
            .map(|(i, h)| {
                let mut ring = ring_back(h);
                ring.zs = before
                    .and_then(|b| b.get(i))
                    .filter(|b| b.pts.len() == ring.pts.len())
                    .and_then(|b| b.zs.clone());
                ring
            })
            .collect()
    })
}

/// The core's parts back, each keeping its own elevations and its holes'
/// as `holes_back` does (docs/adr/0143).
fn parts_back(parts: Option<Vec<Part>>, before: Option<&[AreaPart]>) -> Option<Vec<AreaPart>> {
    parts.map(|ps| {
        ps.into_iter()
            .enumerate()
            .map(|(i, p)| {
                let was = before.and_then(|b| b.get(i));
                let pts = back(p.pts);
                let zs = was
                    .filter(|w| w.pts.len() == pts.len())
                    .and_then(|w| w.zs.clone());
                AreaPart {
                    holes: holes_back(p.holes, was.and_then(|w| w.holes.as_deref())),
                    pts,
                    bulges: p.bulges,
                    zs,
                }
            })
            .collect()
    })
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
        | (
            Entity::Polygon(e),
            Shape::Polygon {
                pts,
                bulges,
                holes,
                parts: None,
            },
        ) => {
            e.pts = back(pts);
            e.bulges = bulges;
            // A transform moves each vertex and keeps it: the elevations stay
            // with their vertices, the holes' too (docs/adr/0142).
            let before = e.holes.take();
            e.holes = holes_back(holes, before.as_deref());
            e.parts = None;
            if e.zs.as_ref().is_some_and(|zs| zs.len() != e.pts.len()) {
                e.zs = None;
            }
        }
        (
            Entity::Polygon(e),
            Shape::Polygon {
                pts,
                bulges,
                holes,
                parts: Some(parts),
            },
        ) => {
            e.pts = back(pts);
            e.bulges = bulges;
            let before = e.holes.take();
            e.holes = holes_back(holes, before.as_deref());
            // Every part too, with its elevations and its holes' (docs/adr/0143).
            let was = e.parts.take();
            e.parts = parts_back(Some(parts), was.as_deref());
            if e.zs.as_ref().is_some_and(|zs| zs.len() != e.pts.len()) {
                e.zs = None;
            }
        }
        (Entity::Circle(e), Shape::Circle { c, r }) => {
            e.c = p(c);
            e.r = r;
        }
        // A transform keeps the block and composes the rest (docs/adr/0144 §3).
        (
            Entity::Insert(e),
            Shape::Insert {
                p: at,
                scale,
                rotation,
                mirror,
                ..
            },
        ) => {
            e.p = p(at);
            e.scale = scale;
            e.rotation = rotation;
            e.mirror = mirror.unwrap_or(false);
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
                align,
                width_factor,
                mask,
            },
        ) => {
            e.p = p(at);
            e.text = text;
            e.height = height;
            e.rotation = rotation;
            e.align = align.and_then(|a| kentos_contracts::TextAlign::from_name(a.name()));
            e.width_factor = width_factor;
            e.mask = mask == Some(true);
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
        (
            Entity::Leader(e),
            Shape::Leader {
                pts,
                text,
                height,
                rotation,
                arrow: _,
                mask,
            },
        ) => {
            // The arrowhead is the object's own: the core carries its name through unchanged.
            e.pts = back(pts);
            e.text = text;
            e.height = height;
            e.rotation = rotation;
            e.mask = mask == Some(true);
        }
        _ => return None,
    }
    Some(out)
}

/// Written elevations as an object holds them (docs/adr/0142): a list with
/// none in it is no list.
fn held(zs: Option<Vec<Option<f64>>>) -> Option<Vec<Option<f64>>> {
    zs.filter(|z| z.iter().any(Option::is_some))
}

/// An object of `geometry` with the fields every object has from `base`:
/// what `cad.entities.edit` writes (docs/adr/0047). A polyline has no holes.
pub fn entity_of(geometry: &EntityGeometry, base: EntityBase) -> Entity {
    match geometry.clone() {
        EntityGeometry::Point { p, z } => Entity::Point(PointEntity { base, p, z }),
        EntityGeometry::Line { a, b, zs } => {
            let z = |k: usize| zs.as_ref().and_then(|zs| zs.get(k).copied().flatten());
            Entity::Line(LineEntity {
                base,
                a,
                b,
                za: z(0),
                zb: z(1),
            })
        }
        EntityGeometry::Polyline { pts, bulges, zs } => Entity::Polyline(PathEntity {
            base,
            pts,
            bulges,
            holes: None,
            zs: held(zs),
            parts: None,
        }),
        EntityGeometry::Polygon {
            pts,
            bulges,
            holes,
            zs,
            parts,
        } => {
            let holes_held = |hs: Option<Vec<RingGeometry>>| {
                hs.map(|hs| {
                    hs.into_iter()
                        .map(|mut h| {
                            h.zs = held(h.zs);
                            h
                        })
                        .collect::<Vec<_>>()
                })
            };
            Entity::Polygon(PathEntity {
                base,
                pts,
                bulges,
                holes: holes_held(holes),
                zs: held(zs),
                // Every part as written, its elevations and its holes' held as the area's (docs/adr/0143).
                parts: parts.map(|ps| {
                    ps.into_iter()
                        .map(|mut p| {
                            p.zs = held(p.zs);
                            p.holes = holes_held(p.holes);
                            p
                        })
                        .collect()
                }),
            })
        }
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
        // A width factor of 1 is no width factor (docs/adr/0145): one spelling.
        EntityGeometry::Text {
            p,
            text,
            height,
            rotation,
            align,
            width_factor,
            mask,
        } => Entity::Text(TextEntity {
            base,
            p,
            text,
            height,
            rotation,
            align,
            width_factor: width_factor.filter(|w| *w != 1.0),
            mask,
        }),
        EntityGeometry::Dimension {
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
            a,
            b,
            offset,
            height,
            text,
            style,
            angle,
            c,
        }),
        EntityGeometry::Hatch {
            ring,
            holes,
            pattern,
        } => Entity::Hatch(HatchEntity {
            base,
            ring,
            holes,
            pattern,
        }),
        EntityGeometry::Insert {
            block,
            p,
            scale,
            rotation,
            mirror,
        } => Entity::Insert(InsertEntity {
            base,
            block,
            p,
            scale,
            rotation,
            mirror,
        }),
        EntityGeometry::Leader {
            pts,
            text,
            height,
            rotation,
            arrow,
            mask,
        } => Entity::Leader(LeaderEntity {
            base,
            pts,
            text,
            height,
            rotation,
            arrow,
            mask,
        }),
    }
}

/// A shape the core computed as `cad.entities.edit` takes it: its kind and
/// geometry fields, float64 bit for bit; a polyline's holes are left out.
/// None for a dimension style, a hatch pattern or a block id the contract
/// does not know (the core carries them as names).
pub fn edit_geometry(shape: Shape) -> Option<EntityGeometry> {
    Some(match shape {
        Shape::Point { p: at, z } => EntityGeometry::Point { p: p(at), z },
        Shape::Line { a, b } => EntityGeometry::Line {
            a: p(a),
            b: p(b),
            zs: None,
        },
        Shape::Polyline { pts, bulges, .. } => EntityGeometry::Polyline {
            pts: back(pts),
            bulges,
            zs: None,
        },
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => EntityGeometry::Polygon {
            pts: back(pts),
            bulges,
            holes: holes.map(|hs| hs.into_iter().map(ring_back).collect()),
            zs: None,
            parts: parts_back(parts, None),
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
            align,
            width_factor,
            mask,
        } => EntityGeometry::Text {
            p: p(at),
            text,
            height,
            rotation,
            align: align.and_then(|a| kentos_contracts::TextAlign::from_name(a.name())),
            width_factor,
            mask: mask == Some(true),
        },
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            text,
            style,
            angle,
            c,
        } => EntityGeometry::Dimension {
            a: p(a),
            b: p(b),
            offset,
            height,
            text,
            style: match style {
                Some(name) => Some(style_of(&name)?),
                None => None,
            },
            angle,
            c: c.map(p),
        },
        Shape::Hatch {
            ring,
            holes,
            pattern,
        } => EntityGeometry::Hatch {
            ring: back(ring),
            holes: holes.map(|hs| hs.into_iter().map(back).collect()),
            pattern: ContractPattern {
                kind: pattern_of(&pattern.kind)?,
                angle: pattern.angle,
                spacing: pattern.spacing,
            },
        },
        // An arrowhead the contract does not name is none of a leader's (docs/adr/0146 §1).
        Shape::Leader {
            pts,
            text,
            height,
            rotation,
            arrow,
            mask,
        } => EntityGeometry::Leader {
            pts: back(pts),
            text,
            height,
            rotation,
            arrow: match arrow {
                Some(name) => Some(LeaderArrow::from_name(&name)?),
                None => None,
            },
            mask: mask == Some(true),
        },
        // A block's id the contract does not read is none of the drawing's.
        Shape::Insert {
            block,
            p: at,
            scale,
            rotation,
            mirror,
            ..
        } => EntityGeometry::Insert {
            block: BlockId::parse(&block)?,
            p: p(at),
            scale,
            rotation,
            mirror: mirror == Some(true),
        },
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

    /// Every kind's geometry as the edit command takes it, written back
    /// onto its object, is that object (Esnet writes dimensions and
    /// hatches too); a name the contract does not know is no geometry.
    #[test]
    fn every_kind_is_written_as_the_core_gives_it() {
        let objects = [
            r#"{"kind":"point","id":1,"layerId":"a","attrs":{},"p":{"x":486512.34,"y":-0.0},"z":850.5}"#,
            r#"{"kind":"polyline","id":3,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":0}],"bulges":[0.5]}"#,
            r#"{"kind":"polygon","id":4,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[{"pts":[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}],"bulges":[0,0,-0.1]}]}"#,
            r#"{"kind":"arc","id":6,"layerId":"a","attrs":{},"c":{"x":1,"y":2},"r":3,"a0":0.5,"a1":2}"#,
            r#"{"kind":"text","id":11,"layerId":"a","attrs":{},"p":{"x":1,"y":2},"text":"Ada 104","height":2,"rotation":-30}"#,
            r#"{"kind":"dimension","id":12,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":-2,"height":0.5,"text":"12,5 m","style":"linear","angle":90}"#,
            r#"{"kind":"dimension","id":14,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":2,"height":0.5,"style":"angular","c":{"x":1,"y":1}}"#,
            r#"{"kind":"hatch","id":13,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"cross","angle":30,"spacing":0.5}}"#,
            r#"{"kind":"leader","id":15,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":3},{"x":9,"y":3}],"text":"Ø150 PVC","height":2,"rotation":-15,"arrow":"open","mask":true}"#,
            r#"{"kind":"leader","id":16,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":3}],"height":2.5,"rotation":0}"#,
        ];
        for text in objects {
            let e = entity(text);
            let g = edit_geometry(shape(&e)).expect("a geometry");
            assert_eq!(
                serde_json::to_string(&entity_of(&g, e.base().clone())).unwrap(),
                serde_json::to_string(&e).unwrap(),
                "{text}"
            );
        }
        let odd = Shape::Hatch {
            ring: vec![Vec2::new(0.0, 0.0); 3],
            holes: None,
            pattern: HatchPattern {
                kind: "dots".into(),
                angle: 0.0,
                spacing: 1.0,
            },
        };
        assert_eq!(edit_geometry(odd), None);
    }

    /// A multi-part area (docs/adr/0143) to the core and back, and through
    /// the edit command's geometry, is itself: every part, its arcs and
    /// holes; a move keeps each vertex's elevation, a part's and its holes'.
    #[test]
    fn a_multi_part_area_goes_to_the_core_and_back_with_its_elevations() {
        let text = r#"{"kind":"polygon","id":4,"layerId":"a","attrs":{"Ada":"104"},"pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"zs":[1,null,3],"parts":[{"pts":[{"x":10,"y":0},{"x":14,"y":0},{"x":14,"y":4},{"x":10,"y":4}],"bulges":[0,0.5,0,0],"holes":[{"pts":[{"x":11,"y":1},{"x":12,"y":1},{"x":12,"y":2}],"zs":[7,8,null]}],"zs":[4,5,6,-0.0]}]}"#;
        let e = entity(text);
        let again = with_shape(&e, shape(&e)).expect("the same kind");
        assert_eq!(
            serde_json::to_string(&again).unwrap(),
            serde_json::to_string(&e).unwrap()
        );
        // Moved 100 m east: every elevation stays with its vertex.
        let moved = kentos_geometry_core::ops::transform::transform_shape(
            &shape(&e),
            &kentos_geometry_core::geom::affine::translation(100.0, 0.0),
        );
        let Some(Entity::Polygon(p)) = with_shape(&e, moved) else {
            panic!("an area");
        };
        let part = &p.parts.as_ref().expect("parts")[0];
        assert_eq!(part.pts[0].x, 110.0);
        assert_eq!(
            part.zs,
            Some(vec![Some(4.0), Some(5.0), Some(6.0), Some(-0.0)])
        );
        assert_eq!(
            part.holes.as_ref().expect("a hole")[0].zs,
            Some(vec![Some(7.0), Some(8.0), None])
        );
        assert_eq!(p.zs, Some(vec![Some(1.0), None, Some(3.0)]));
        // Through the edit command's geometry: the parts come along, their elevations as the geometry says.
        let g = edit_geometry(shape(&e)).expect("a geometry");
        let EntityGeometry::Polygon {
            parts: Some(parts), ..
        } = &g
        else {
            panic!("{g:?}");
        };
        assert_eq!(parts.len(), 1);
        let written = entity_of(&g, e.base().clone());
        let Entity::Polygon(w) = written else {
            panic!()
        };
        assert_eq!(w.parts.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            w.parts.as_ref().unwrap()[0].pts,
            p.parts.as_ref().unwrap()[0]
                .pts
                .iter()
                .map(|q| Point {
                    x: q.x - 100.0,
                    y: q.y
                })
                .collect::<Vec<_>>()
        );
    }
}
