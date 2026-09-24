//! Entities ↔ areas for the area tools (`apps/web/src/model/ops/areas.ts`). Polygons
//! and circles convert exactly (a circle is two half-circle bulges); a full
//! ellipse or a closed spline becomes a fine polygon within 1 mm of the
//! curve, since an area has only straight and circular edges.

use crate::api::Op;
use crate::entity::{Entity, Shape, ellipse_geom};
use crate::geom::arc::{ArcGeom, arc_end, arc_start};
use crate::geom::arrangement::{Area, Ring, Source};
use crate::geom::bulge::{bulge_at, has_bulges};
use crate::geom::ellipse::{is_full_ellipse, tessellate_ellipse};
use crate::geom::spline::catmull_rom;
use crate::jsmath::{PI, js_hypot, js_max, js_min};
use crate::op;
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

/// Chord deviation allowed when a curve has to become straight edges (m).
const CURVE_TOL: f64 = 1e-3;
/// Polyline ends closer than this count as closed.
const CLOSE_TOL: f64 = 1e-6;

fn ring(pts: Vec<Vec2>, bulges: Option<Vec<f64>>) -> Ring {
    if has_bulges(bulges.as_deref()) {
        Ring { pts, bulges }
    } else {
        Ring { pts, bulges: None }
    }
}

/// The area an entity encloses, or None for open or non-area entities. Hatches are fills, not areas.
pub fn area_of_entity(e: &Shape) -> Option<Area> {
    match e {
        Shape::Polygon { pts, bulges, holes } => (pts.len() >= 2).then(|| Area {
            outer: ring(pts.clone(), bulges.clone()),
            holes: holes
                .iter()
                .flatten()
                .map(|h| ring(h.pts.clone(), h.bulges.clone()))
                .collect(),
        }),
        Shape::Circle { c, r } => Some(Area {
            outer: Ring {
                pts: vec![Vec2::new(c.x + r, c.y), Vec2::new(c.x - r, c.y)],
                bulges: Some(vec![1.0, 1.0]),
            },
            holes: Vec::new(),
        }),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            if !is_full_ellipse(&g) {
                return None;
            }
            let a = js_hypot(major.x, major.y);
            // Sagitta a·(π/n)²/2 ≤ tolerance.
            let n = js_min(
                4096.0,
                js_max(64.0, (PI / ((2.0 * CURVE_TOL) / a).sqrt()).ceil()),
            );
            Some(Area {
                outer: Ring {
                    pts: tessellate_ellipse(&g, n),
                    bulges: None,
                },
                holes: Vec::new(),
            })
        }
        Shape::Spline { pts, closed } => (*closed && pts.len() >= 3).then(|| {
            let mut ring = catmull_rom(pts, true, 32.0);
            ring.pop();
            Area {
                outer: Ring {
                    pts: ring,
                    bulges: None,
                },
                holes: Vec::new(),
            }
        }),
        Shape::Polyline { pts, bulges, .. } => {
            let n = pts.len();
            if n < 3 {
                return None;
            }
            let (f, l) = (pts[0], pts[n - 1]);
            if js_hypot(f.x - l.x, f.y - l.y) > CLOSE_TOL {
                return None;
            }
            // Drop the repeated end; the last segment's bulge becomes the closing one.
            Some(Area {
                outer: ring(
                    pts[..n - 1].to_vec(),
                    bulges.as_ref().map(|b| b[..b.len().min(n - 1)].to_vec()),
                ),
                holes: Vec::new(),
            })
        }
        _ => None,
    }
}

/// Polygon geometry of an area (holes only when there are some).
pub fn polygon_of_area(a: &Area) -> Entity {
    Entity::new(Shape::Polygon {
        pts: a.outer.pts.clone(),
        bulges: a.outer.bulges.clone(),
        holes: (!a.holes.is_empty()).then(|| a.holes.clone()),
    })
}

/// A path's rings as closed polylines (first point repeated at the end): outer first, then holes.
pub fn polylines_of_polygon(e: &Shape) -> Result<Vec<Entity>, String> {
    let (pts, bulges, holes) = match e {
        Shape::Polyline { pts, bulges, holes } | Shape::Polygon { pts, bulges, holes } => {
            (pts, bulges, holes)
        }
        _ => return Err("çoklu çizgi ya da kapalı alan bekleniyordu".into()),
    };
    let rings =
        std::iter::once((pts, bulges)).chain(holes.iter().flatten().map(|h| (&h.pts, &h.bulges)));
    rings
        .map(|(pts, bulges)| {
            let Some(&start) = pts.first() else {
                return Err("Boş bir halka çoklu çizgiye çevrilemez.".to_string());
            };
            let mut out = pts.clone();
            out.push(start);
            let bulges = has_bulges(bulges.as_deref()).then(|| {
                let mut b: Vec<f64> = (0..pts.len())
                    .map(|i| bulge_at(bulges.as_deref(), i))
                    .collect();
                b.push(0.0);
                b
            });
            Ok(Entity::new(Shape::Polyline {
                pts: out,
                bulges,
                holes: None,
            }))
        })
        .collect()
}

/// Line work as an overlay source (cut lines, boundaries of "click inside").
pub fn line_source(entities: &[Entity]) -> Source {
    let edges = entities
        .iter()
        .flat_map(|e| entity_edges(&e.shape))
        .collect();
    let mut points: Vec<Vec2> = Vec::new();
    for e in entities {
        match &e.shape {
            Shape::Line { a, b } => points.extend([*a, *b]),
            Shape::Polyline { pts, .. } => points.extend_from_slice(pts),
            Shape::Polygon { pts, holes, .. } => {
                points.extend_from_slice(pts);
                for h in holes.iter().flatten() {
                    points.extend_from_slice(&h.pts);
                }
            }
            Shape::Arc { c, r, a0, a1 } => {
                let g = ArcGeom {
                    c: *c,
                    r: *r,
                    a0: *a0,
                    a1: *a1,
                };
                points.extend([arc_start(&g), arc_end(&g)]);
            }
            _ => {}
        }
    }
    Source {
        edges,
        points: Some(points),
        cut: Some(true),
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("areaOfEntity", |e: Entity| area_of_entity(&e.shape)),
    op!("polygonOfArea", |a: Area| polygon_of_area(&a)),
    op!("polylinesOfPolygon", |e: Entity| polylines_of_polygon(
        &e.shape
    )),
    op!("lineSource", |entities: Vec<Entity>| line_source(&entities)),
];
