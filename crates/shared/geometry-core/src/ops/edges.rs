//! An entity as primitive edges (`apps/web/src/model/ops/edges.ts`): a new kind takes
//! part in intersection, trimming, extending and snapping by giving its edges.

use crate::api::Op;
use crate::entity::{CONSTRUCTION_REACH, Entity, Shape, dimension_geom, ellipse_geom};
use crate::geom::arc::sweep;
use crate::geom::bulge::bulge_path_edges;
use crate::geom::dimension::layout_dimension;
use crate::geom::ellipse::{is_full_ellipse, tessellate_ellipse};
use crate::geom::intersect::{Edge, closest_on_edge, full_circle};
use crate::geom::spline::catmull_rom;
use crate::jsmath::js_hypot;
use crate::op;
use crate::vec2::Vec2;

/// Decomposes an entity into primitive edges (points and text have none).
pub fn entity_edges(e: &Shape) -> Vec<Edge> {
    match e {
        Shape::Line { a, b } => vec![Edge::Seg { a: *a, b: *b }],
        Shape::Polyline { pts, bulges, .. } => bulge_path_edges(pts, bulges.as_deref(), false),
        Shape::Polygon { pts, bulges, holes } => {
            let mut out = bulge_path_edges(pts, bulges.as_deref(), true);
            for h in holes.iter().flatten() {
                out.extend(bulge_path_edges(&h.pts, h.bulges.as_deref(), true));
            }
            out
        }
        // The tessellated curve already ends on its first point when closed.
        Shape::Spline { pts, closed } => path_edges(&catmull_rom(pts, *closed, 16.0), false),
        Shape::Hatch { ring, holes, .. } => {
            let mut out = path_edges(ring, true);
            for h in holes.iter().flatten() {
                out.extend(path_edges(h, true));
            }
            out
        }
        Shape::Dimension { .. } => dimension_geom(e)
            .and_then(|d| layout_dimension(&d))
            .map(|l| l.pick)
            .unwrap_or_default(),
        Shape::Circle { c, r } => vec![full_circle(*c, *r)],
        Shape::Arc { c, r, a0, a1 } => vec![Edge::Arc {
            c: *c,
            r: *r,
            a0: *a0,
            sweep: sweep(*a0, *a1),
        }],
        // Fine chords for boundaries and nearest points; crossings are refined onto the curve.
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            path_edges(&tessellate_ellipse(&g, 256.0), is_full_ellipse(&g))
        }
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => {
            let r = CONSTRUCTION_REACH;
            let a = if matches!(e, Shape::Ray { .. }) {
                *p
            } else {
                Vec2::new(p.x - dir.x * r, p.y - dir.y * r)
            };
            vec![Edge::Seg {
                a,
                b: Vec2::new(p.x + dir.x * r, p.y + dir.y * r),
            }]
        }
        Shape::Point { .. } | Shape::Text { .. } => Vec::new(),
    }
}

/// The edge of `e` nearest to `p`: the clicked segment of a polyline, a
/// circle itself (the tangent circle tools' pick, `CircleTool`; docs/adr/0032).
/// The first of equally near edges; None for a point or a text.
pub fn nearest_edge(e: &Shape, p: Vec2) -> Option<Edge> {
    let mut best: Option<(Edge, f64)> = None;
    for edge in entity_edges(e) {
        let d = closest_on_edge(&edge, p).d;
        // `d < best.d`, as in TypeScript: a NaN distance never wins.
        if best.as_ref().is_none_or(|(_, bd)| d < *bd) {
            best = Some((edge, d));
        }
    }
    best.map(|(edge, _)| edge)
}

pub fn edge_length(e: &Edge) -> f64 {
    match *e {
        Edge::Seg { a, b } => js_hypot(b.x - a.x, b.y - a.y),
        Edge::Arc { r, sweep, .. } => r * sweep.abs(),
    }
}

pub fn path_edges(pts: &[Vec2], closed: bool) -> Vec<Edge> {
    let n = pts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    (0..count)
        .map(|i| Edge::Seg {
            a: pts[i],
            b: pts[(i + 1) % n],
        })
        .collect()
}

pub(crate) static OPS: &[Op] = &[
    op!("entityEdges", |e: Entity| entity_edges(&e.shape)),
    op!("edgeLength", |e: Edge| edge_length(&e)),
    op!("nearestEdge", |e: Entity, p: Vec2| nearest_edge(
        &e.shape, p
    )),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_edge_is_the_clicked_segment_or_the_circle() {
        let path = Shape::Polyline {
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 10.0),
            ],
            bulges: None,
            holes: None,
        };
        assert_eq!(
            nearest_edge(&path, Vec2::new(9.0, 6.0)),
            Some(Edge::Seg {
                a: Vec2::new(10.0, 0.0),
                b: Vec2::new(10.0, 10.0)
            })
        );
        // Equally near both: the first.
        assert_eq!(
            nearest_edge(&path, Vec2::new(11.0, -1.0)),
            Some(Edge::Seg {
                a: Vec2::new(0.0, 0.0),
                b: Vec2::new(10.0, 0.0)
            })
        );
        let circle = Shape::Circle {
            c: Vec2::new(5.0, 5.0),
            r: 2.0,
        };
        assert!(matches!(
            nearest_edge(&circle, Vec2::new(0.0, 0.0)),
            Some(Edge::Arc { r, .. }) if r == 2.0
        ));
        assert_eq!(
            nearest_edge(
                &Shape::Point {
                    p: Vec2::new(1.0, 1.0),
                    z: None
                },
                Vec2::new(0.0, 0.0)
            ),
            None
        );
    }
}
