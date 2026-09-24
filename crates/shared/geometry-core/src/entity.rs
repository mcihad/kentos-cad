//! Drawing objects and their geometry (`apps/web/src/model/entities.ts`): the 13
//! kinds, and the helpers every tool and view uses (vertices, outlines,
//! bounds, anchors, lengths, areas). An entity keeps the fields the core
//! does not interpret (id, layer, colour, attributes, label, symbol) as they
//! came, so operations that return `{ ...e, … }` in TypeScript give them
//! back unchanged (docs/adr/0008).

use crate::api::json::{FromJson, Json, ToJson, write_str};
use crate::api::{Op, json};
use crate::geom::arc::{
    ArcGeom, DEFAULT_STEP, arc_end, arc_length, arc_mid, arc_start, tessellate_arc,
};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::{bulge_path_length, bulge_path_outline, bulge_ring_area, has_bulges};
use crate::geom::dimension::{DimensionGeom, DimensionLayout, layout_dimension};
use crate::geom::ellipse::{
    EllipseGeom, ellipse_area, ellipse_length, ellipse_point, is_full_ellipse, quadrant_params,
    tessellate_ellipse,
};
use crate::geom::spline::catmull_rom;
use crate::geometry::{
    Bounds, centroid, empty_bounds, extend_bounds, path_length, point_in_polygon, signed_area,
};
use crate::jsmath::{PI, TAU, cos, js_max, sin};
use crate::op;
use crate::vec2::Vec2;

/// Half-length (1000 km) an infinite line gets when it meets finite geometry
/// on the CPU (`CONSTRUCTION_REACH`).
pub const CONSTRUCTION_REACH: f64 = 1e6;

#[derive(Clone, Debug, PartialEq)]
pub struct HatchPattern {
    pub kind: String,
    pub angle: f64,
    pub spacing: f64,
}

crate::json_struct!(HatchPattern { kind => "type", angle, spacing });

/// The geometry of an entity, tagged by `kind` as in TypeScript.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Point {
        p: Vec2,
        z: Option<f64>,
    },
    Line {
        a: Vec2,
        b: Vec2,
    },
    Polyline {
        pts: Vec<Vec2>,
        bulges: Option<Vec<f64>>,
        holes: Option<Vec<Ring>>,
    },
    Polygon {
        pts: Vec<Vec2>,
        bulges: Option<Vec<f64>>,
        holes: Option<Vec<Ring>>,
    },
    Circle {
        c: Vec2,
        r: f64,
    },
    Arc {
        c: Vec2,
        r: f64,
        a0: f64,
        a1: f64,
    },
    Ellipse {
        c: Vec2,
        major: Vec2,
        ratio: f64,
        t0: f64,
        t1: f64,
    },
    Xline {
        p: Vec2,
        dir: Vec2,
    },
    Ray {
        p: Vec2,
        dir: Vec2,
    },
    Spline {
        pts: Vec<Vec2>,
        closed: bool,
    },
    Text {
        p: Vec2,
        text: String,
        height: f64,
        rotation: f64,
    },
    Dimension {
        a: Vec2,
        b: Vec2,
        offset: f64,
        height: f64,
        text: Option<String>,
        style: Option<String>,
        angle: Option<f64>,
        c: Option<Vec2>,
    },
    Hatch {
        ring: Vec<Vec2>,
        holes: Option<Vec<Vec<Vec2>>>,
        pattern: HatchPattern,
    },
}

crate::json_tagged!(Shape, "kind",
    Point => "point" { p, z },
    Line => "line" { a, b },
    Polyline => "polyline" { pts, bulges, holes },
    Polygon => "polygon" { pts, bulges, holes },
    Circle => "circle" { c, r },
    Arc => "arc" { c, r, a0, a1 },
    Ellipse => "ellipse" { c, major, ratio, t0, t1 },
    Xline => "xline" { p, dir },
    Ray => "ray" { p, dir },
    Spline => "spline" { pts, closed },
    Text => "text" { p, text, height, rotation },
    Dimension => "dimension" { a, b, offset, height, text, style, angle, c },
    Hatch => "hatch" { ring, holes, pattern },
);

/// An entity: its geometry and every other field, untouched and in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub shape: Shape,
    pub rest: Vec<(String, Json)>,
}

impl Entity {
    pub fn new(shape: Shape) -> Entity {
        Entity {
            shape,
            rest: Vec::new(),
        }
    }

    /// The same entity with another geometry (`{ ...e, …geometry }`).
    pub fn with(&self, shape: Shape) -> Entity {
        Entity {
            shape,
            rest: self.rest.clone(),
        }
    }
}

impl FromJson for Entity {
    fn from_json(v: &Json) -> Result<Entity, String> {
        let Json::Obj(fields) = v else {
            return Err("nesne bekleniyordu".into());
        };
        let shape = Shape::from_json(v)?;
        let kind = match v.get("kind") {
            Json::Str(k) => k.as_str(),
            _ => "",
        };
        let own = Shape::field_names(kind);
        let rest = fields
            .iter()
            .filter(|(k, _)| k != "kind" && !own.contains(&k.as_str()))
            .cloned()
            .collect();
        Ok(Entity { shape, rest })
    }
}

impl ToJson for Entity {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        for (k, v) in &self.rest {
            if !first {
                out.push(',');
            }
            first = false;
            write_str(out, k);
            out.push(':');
            v.write_json(out);
        }
        self.shape.write_fields(out, &mut first);
        out.push('}');
    }
}

/// A dimension shape as the layout reads it.
pub fn dimension_geom(s: &Shape) -> Option<DimensionGeom> {
    match s {
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            style,
            angle,
            c,
            ..
        } => Some(DimensionGeom {
            a: *a,
            b: *b,
            offset: *offset,
            height: *height,
            style: style.clone(),
            angle: *angle,
            c: *c,
        }),
        _ => None,
    }
}

pub fn ellipse_geom(c: Vec2, major: Vec2, ratio: f64, t0: f64, t1: f64) -> EllipseGeom {
    EllipseGeom {
        c,
        major,
        ratio,
        t0,
        t1,
    }
}

fn layout_of(s: &Shape) -> Option<DimensionLayout> {
    dimension_geom(s).and_then(|d| layout_dimension(&d))
}

pub fn tessellate_circle(c: Vec2, r: f64, segments: f64) -> Vec<Vec2> {
    let mut pts = Vec::new();
    let mut i = 0.0;
    while i < segments {
        let t = (i / segments) * PI * 2.0;
        pts.push(Vec2::new(c.x + cos(t) * r, c.y + sin(t) * r));
        i += 1.0;
    }
    pts
}

/// Characteristic vertices: grips, snapping and coordinate tables.
pub fn entity_vertices(e: &Shape) -> Vec<Vec2> {
    match e {
        Shape::Point { p, .. } | Shape::Text { p, .. } => vec![*p],
        Shape::Line { a, b } => vec![*a, *b],
        Shape::Polyline { pts, .. } => pts.clone(),
        Shape::Polygon { pts, holes, .. } => match holes {
            Some(h) if !h.is_empty() => pts
                .iter()
                .copied()
                .chain(h.iter().flat_map(|r| r.pts.iter().copied()))
                .collect(),
            _ => pts.clone(),
        },
        Shape::Circle { c, r } => {
            vec![
                *c,
                Vec2::new(c.x + r, c.y),
                Vec2::new(c.x, c.y + r),
                Vec2::new(c.x - r, c.y),
                Vec2::new(c.x, c.y - r),
            ]
        }
        Shape::Arc { c, r, a0, a1 } => {
            let g = ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            };
            vec![arc_start(&g), arc_mid(&g), arc_end(&g)]
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            let mut out = vec![*c];
            if !is_full_ellipse(&g) {
                out.push(ellipse_point(&g, *t0));
                out.push(ellipse_point(&g, *t1));
            }
            out.extend(
                quadrant_params(&g)
                    .into_iter()
                    .map(|t| ellipse_point(&g, t)),
            );
            out
        }
        Shape::Xline { p, .. } | Shape::Ray { p, .. } => vec![*p],
        Shape::Spline { pts, .. } => pts.clone(),
        Shape::Dimension { a, b, c, .. } => match c {
            Some(c) => vec![*a, *b, *c],
            None => vec![*a, *b],
        },
        Shape::Hatch { ring, holes, .. } => match holes {
            Some(h) if !h.is_empty() => ring
                .iter()
                .copied()
                .chain(h.iter().flatten().copied())
                .collect(),
            _ => ring.clone(),
        },
    }
}

/// Outline as a point list (curves tessellated): previews, bounds and hit tests.
pub fn entity_outline(e: &Shape, segments: f64) -> Vec<Vec2> {
    match e {
        Shape::Circle { c, r } => tessellate_circle(*c, *r, segments),
        Shape::Arc { c, r, a0, a1 } => tessellate_arc(
            &ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            },
            DEFAULT_STEP,
        ),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => tessellate_ellipse(
            &ellipse_geom(*c, *major, *ratio, *t0, *t1),
            js_max(64.0, segments * 2.0),
        ),
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => {
            // Long enough for any preview; the renderer clips to the view.
            let r = CONSTRUCTION_REACH;
            let start = if matches!(e, Shape::Ray { .. }) {
                *p
            } else {
                Vec2::new(p.x - dir.x * r, p.y - dir.y * r)
            };
            vec![start, Vec2::new(p.x + dir.x * r, p.y + dir.y * r)]
        }
        Shape::Spline { pts, closed } => catmull_rom(pts, *closed, 16.0),
        Shape::Polyline { pts, bulges, .. } => {
            bulge_path_outline(pts, bulges.as_deref(), false, TAU / segments)
        }
        Shape::Polygon { pts, bulges, .. } => {
            bulge_path_outline(pts, bulges.as_deref(), true, TAU / segments)
        }
        Shape::Dimension { a, b, style, .. } => match layout_of(e) {
            None => vec![*a, *b],
            Some(l) => {
                let style = style.as_deref().unwrap_or("aligned");
                if style == "aligned" || style == "linear" {
                    vec![*a, l.d1, l.d2, *b]
                } else {
                    let mut out = vec![*a];
                    out.extend(l.lines.iter().flatten().copied());
                    out.push(*b);
                    out
                }
            }
        },
        _ => entity_vertices(e),
    }
}

/// Closed ring of a polygon, arcs tessellated: fills, hit tests and hatching.
pub fn polygon_ring(pts: &[Vec2], bulges: Option<&[f64]>) -> Vec<Vec2> {
    if has_bulges(bulges) {
        bulge_path_outline(pts, bulges, true, DEFAULT_STEP)
    } else {
        pts.to_vec()
    }
}

/// Hole rings of a polygon (arcs tessellated) or a hatch; empty for anything else.
pub fn polygon_holes(e: &Shape) -> Vec<Vec<Vec2>> {
    match e {
        Shape::Polygon { holes: Some(h), .. } => h
            .iter()
            .map(|r| polygon_ring(&r.pts, r.bulges.as_deref()))
            .collect(),
        Shape::Hatch { holes: Some(h), .. } => h.clone(),
        _ => Vec::new(),
    }
}

/// Whether p is inside a path's outer ring and outside its holes (tessellated test).
pub fn inside_polygon(e: &Shape, p: Vec2) -> bool {
    let (pts, bulges, holes) = match e {
        Shape::Polyline { pts, bulges, holes } | Shape::Polygon { pts, bulges, holes } => {
            (pts, bulges, holes)
        }
        _ => return false,
    };
    point_in_polygon(p, &polygon_ring(pts, bulges.as_deref()))
        && !holes
            .iter()
            .flatten()
            .any(|h| point_in_polygon(p, &polygon_ring(&h.pts, h.bulges.as_deref())))
}

/// Approximate rotated box of a text (~0.55 em per glyph, JavaScript's string length).
pub fn text_box(p: Vec2, text: &str, height: f64, rotation: f64) -> Vec<Vec2> {
    let w = js_max(1.0, text.encode_utf16().count() as f64) * height * 0.55;
    let h = height * 1.15;
    let r = (rotation * PI) / 180.0;
    let ux = cos(r);
    let uy = sin(r);
    let at = |u: f64, v: f64| Vec2::new(p.x + ux * u - uy * v, p.y + uy * u + ux * v);
    vec![at(0.0, -h * 0.2), at(w, -h * 0.2), at(w, h), at(0.0, h)]
}

/// Whether the outline is a closed ring.
pub fn is_closed_outline(e: &Shape) -> bool {
    match e {
        Shape::Polygon { .. } | Shape::Circle { .. } | Shape::Hatch { .. } => true,
        Shape::Spline { closed, .. } => *closed,
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => is_full_ellipse(&ellipse_geom(*c, *major, *ratio, *t0, *t1)),
        _ => false,
    }
}

pub fn entity_bounds(e: &Shape) -> Bounds {
    let mut b = empty_bounds();
    match e {
        Shape::Circle { c, r } => {
            extend_bounds(&mut b, *c, *r);
            return b;
        }
        Shape::Text {
            p,
            text,
            height,
            rotation,
        } => {
            for q in text_box(*p, text, *height, *rotation) {
                extend_bounds(&mut b, q, 0.0);
            }
            return b;
        }
        // Construction lines count by their base point only (zoom extents ignores their reach).
        Shape::Xline { p, .. } | Shape::Ray { p, .. } => {
            extend_bounds(&mut b, *p, 0.0);
            return b;
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            for q in tessellate_ellipse(&ellipse_geom(*c, *major, *ratio, *t0, *t1), 256.0) {
                extend_bounds(&mut b, q, 0.0);
            }
            return b;
        }
        _ => {}
    }
    let curved = match e {
        Shape::Arc { .. } | Shape::Spline { .. } | Shape::Dimension { .. } => true,
        Shape::Polyline { bulges, .. } | Shape::Polygon { bulges, .. } => {
            has_bulges(bulges.as_deref())
        }
        _ => false,
    };
    if curved {
        for q in entity_outline(e, 72.0) {
            extend_bounds(&mut b, q, 0.0);
        }
        if let Shape::Dimension {
            a, b: bb, height, ..
        } = e
        {
            extend_bounds(
                &mut b,
                Vec2::new((a.x + bb.x) / 2.0, (a.y + bb.y) / 2.0),
                height * 2.0,
            );
        }
        return b;
    }
    for q in entity_vertices(e) {
        extend_bounds(&mut b, q, 0.0);
    }
    b
}

/// Where a label sits; None for a path without vertices (the TypeScript's undefined).
pub fn entity_anchor(e: &Shape) -> Option<Vec2> {
    Some(match e {
        Shape::Polygon { pts, bulges, .. } => centroid(&polygon_ring(pts, bulges.as_deref())),
        Shape::Circle { c, .. } | Shape::Ellipse { c, .. } => *c,
        Shape::Arc { c, r, a0, a1 } => arc_mid(&ArcGeom {
            c: *c,
            r: *r,
            a0: *a0,
            a1: *a1,
        }),
        Shape::Line { a, b } => Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0),
        Shape::Polyline { pts, .. } | Shape::Spline { pts, .. } => {
            return pts.get(pts.len() / 2).copied();
        }
        Shape::Dimension { a, .. } => layout_of(e).map_or(*a, |l| l.text_at),
        Shape::Hatch { ring, .. } => centroid(ring),
        Shape::Point { p, .. }
        | Shape::Text { p, .. }
        | Shape::Xline { p, .. }
        | Shape::Ray { p, .. } => *p,
    })
}

pub fn entity_length(e: &Shape) -> Option<f64> {
    match e {
        Shape::Line { a, b } => Some(path_length(&[*a, *b], false)),
        Shape::Polyline { pts, bulges, holes } | Shape::Polygon { pts, bulges, holes } => {
            // A polygon's perimeter includes its holes (as in GIS).
            let closed = matches!(e, Shape::Polygon { .. });
            let holes = holes.iter().flatten().fold(0.0, |s, h| {
                s + bulge_path_length(&h.pts, h.bulges.as_deref(), true)
            });
            Some(bulge_path_length(pts, bulges.as_deref(), closed) + holes)
        }
        Shape::Circle { r, .. } => Some(2.0 * PI * r),
        Shape::Arc { c, r, a0, a1 } => Some(arc_length(&ArcGeom {
            c: *c,
            r: *r,
            a0: *a0,
            a1: *a1,
        })),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => Some(ellipse_length(&ellipse_geom(*c, *major, *ratio, *t0, *t1))),
        Shape::Spline { pts, closed } => Some(path_length(&catmull_rom(pts, *closed, 16.0), false)),
        // The measured value when it is a length (an angle has none).
        Shape::Dimension { .. } => layout_of(e).filter(|l| l.unit == "length").map(|l| l.value),
        _ => None,
    }
}

pub fn entity_area(e: &Shape) -> Option<f64> {
    match e {
        Shape::Polygon { pts, bulges, holes } => {
            let holes = holes.iter().flatten().fold(0.0, |s, h| {
                s + bulge_ring_area(&h.pts, h.bulges.as_deref()).abs()
            });
            Some(bulge_ring_area(pts, bulges.as_deref()).abs() - holes)
        }
        Shape::Circle { r, .. } => Some(PI * r * r),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            is_full_ellipse(&g).then(|| ellipse_area(&g))
        }
        Shape::Hatch { ring, holes, .. } => Some(
            signed_area(ring).abs()
                - holes
                    .iter()
                    .flatten()
                    .fold(0.0, |s, h| s + signed_area(h).abs()),
        ),
        _ => None,
    }
}

/// Geometry-only view of an entity: drops id, layer, colour, attributes and label.
pub fn entity_geometry(e: &Entity) -> Entity {
    const DROP: [&str; 5] = ["id", "layerId", "attrs", "color", "label"];
    Entity {
        shape: e.shape.clone(),
        rest: e
            .rest
            .iter()
            .filter(|(k, _)| !DROP.contains(&k.as_str()))
            .cloned()
            .collect(),
    }
}

pub(crate) static OPS: &[Op] = &[
    op!(
        "tessellateCircle",
        |c: Vec2, r: f64, segments: Option<f64>| tessellate_circle(c, r, segments.unwrap_or(72.0))
    ),
    op!("entityVertices", |e: Entity| entity_vertices(&e.shape)),
    op!("entityOutline", |e: Entity, segments: Option<f64>| {
        entity_outline(&e.shape, segments.unwrap_or(72.0))
    }),
    op!("polygonRing", |r: Ring| polygon_ring(
        &r.pts,
        r.bulges.as_deref()
    )),
    op!("polygonHoles", |e: Entity| polygon_holes(&e.shape)),
    op!("insidePolygon", |e: Entity, p: Vec2| inside_polygon(
        &e.shape, p
    )),
    op!("textBox", |e: Json| text_box_json(&e)),
    op!("isClosedOutline", |e: Entity| is_closed_outline(&e.shape)),
    op!("entityBounds", |e: Entity| entity_bounds(&e.shape)),
    op!("entityAnchor", |e: Entity| entity_anchor(&e.shape)),
    op!("entityLength", |e: Entity| entity_length(&e.shape)),
    op!("entityArea", |e: Entity| entity_area(&e.shape)),
    op!("entityGeometry", |e: Entity| entity_geometry(&e)),
];

/// `textBox` takes any object with p, text, height and rotation (a text entity or a draft).
fn text_box_json(v: &Json) -> Result<Vec<Vec2>, String> {
    let p: Vec2 = json::read_field(v, "p")?;
    let text: String = json::read_field(v, "text")?;
    let height: f64 = json::read_field(v, "height")?;
    let rotation: f64 = json::read_field(v, "rotation")?;
    Ok(text_box(p, &text, height, rotation))
}

impl FromJson for Json {
    fn from_json(v: &Json) -> Result<Json, String> {
        Ok(v.clone())
    }
}
