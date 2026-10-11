//! Nesnelere değenler (docs/adr/0220 §2): whether a grid's cell meets one
//! of the objects as Konuma göre seç's Kesişen takes it (docs/adr/0200 §1,
//! within 1 mm), for a million cells: the objects' edges and points in
//! packed trees, their areas' insides from winding indexes. A cell meets an
//! object when an edge of it meets one of the cell's sides or starts in the
//! cell, a point of it is in the cell or within 1 mm of it, or the cell's
//! first corner is in the object's areas: what `spatial_query::intersects`
//! asks, without going through every edge of a large area for every cell.

use crate::entity::Shape;
use crate::geom::arrangement::{WindingIndex, tight_box};
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::geom::region::area_source;
use crate::geometry::{Bounds, empty_bounds};
use crate::jsmath::{js_max, js_min};
use crate::ops::spatial_query::{TOLERANCE, edges_meet, geometry_of};
use crate::predicates::orientation;
use crate::store::rtree::PackedTree;
use crate::vec2::Vec2;

pub struct Cover {
    edges: Vec<Edge>,
    edge_tree: PackedTree,
    points: Vec<Vec2>,
    point_tree: PackedTree,
    areas: Vec<WindingIndex<'static>>,
    area_tree: PackedTree,
}

fn grow(b: &mut Bounds, o: &Bounds) {
    b.min_x = js_min(b.min_x, o.min_x);
    b.min_y = js_min(b.min_y, o.min_y);
    b.max_x = js_max(b.max_x, o.max_x);
    b.max_y = js_max(b.max_y, o.max_y);
}

fn point_box(p: Vec2) -> Bounds {
    Bounds {
        min_x: p.x,
        min_y: p.y,
        max_x: p.x,
        max_y: p.y,
    }
}

/// In or on a counter-clockwise convex ring (exact sides).
fn in_convex(ring: &[Vec2], p: Vec2) -> bool {
    let n = ring.len();
    (0..n).all(|i| orientation(ring[i], ring[(i + 1) % n], p) >= 0)
}

impl Cover {
    pub fn new(shapes: &[Shape]) -> Cover {
        let mut edges = Vec::new();
        let mut edge_boxes = Vec::new();
        let mut points = Vec::new();
        let mut areas = Vec::new();
        let mut area_boxes = Vec::new();
        for s in shapes {
            let Some(g) = geometry_of(s) else {
                continue;
            };
            for e in g.edges {
                edge_boxes.push((edges.len() as u32, tight_box(&e)));
                edges.push(e);
            }
            points.extend(g.points);
            if !g.areas.is_empty() {
                let ring_edges = area_source(&g.areas).edges;
                let mut b = empty_bounds();
                for e in &ring_edges {
                    grow(&mut b, &tight_box(e));
                }
                area_boxes.push((areas.len() as u32, b));
                areas.push(WindingIndex::owned(ring_edges));
            }
        }
        let point_boxes: Vec<(u32, Bounds)> = points
            .iter()
            .enumerate()
            .map(|(i, &p)| (i as u32, point_box(p)))
            .collect();
        Cover {
            edge_tree: PackedTree::build(&edge_boxes),
            edges,
            point_tree: PackedTree::build(&point_boxes),
            points,
            area_tree: PackedTree::build(&area_boxes),
            areas,
        }
    }

    /// Whether the counter-clockwise convex ring (a cell) meets an object.
    pub fn meets(&self, ring: &[Vec2], near: &mut Vec<u32>) -> bool {
        let n = ring.len();
        let mut b = point_box(ring[0]);
        for &p in &ring[1..] {
            grow(&mut b, &point_box(p));
        }
        let padded = Bounds {
            min_x: b.min_x - TOLERANCE,
            min_y: b.min_y - TOLERANCE,
            max_x: b.max_x + TOLERANCE,
            max_y: b.max_y + TOLERANCE,
        };
        near.clear();
        self.edge_tree.search(&padded, near);
        for &i in near.iter() {
            let e = &self.edges[i as usize];
            for k in 0..n {
                let side = Edge::Seg {
                    a: ring[k],
                    b: ring[(k + 1) % n],
                };
                if edges_meet(&side, e) {
                    return true;
                }
            }
            if in_convex(ring, point_at(e, 0.0)) {
                return true;
            }
        }
        near.clear();
        self.point_tree.search(&padded, near);
        for &i in near.iter() {
            let p = self.points[i as usize];
            if in_convex(ring, p) {
                return true;
            }
            let close = (0..n).any(|k| {
                let side = Edge::Seg {
                    a: ring[k],
                    b: ring[(k + 1) % n],
                };
                closest_on_edge(&side, p).d <= TOLERANCE
            });
            if close {
                return true;
            }
        }
        near.clear();
        self.area_tree.search(&point_box(ring[0]), near);
        near.iter()
            .any(|&i| self.areas[i as usize].winding(ring[0]) != 0.0)
    }
}
