//! Adding and removing path vertices (`apps/web/src/model/ops/vertex.ts`). An arc
//! segment split by a new vertex keeps its circle.

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::bulge::{bulge_arc, bulge_at, bulge_of_sweep, clean_bulge_path, is_arc_bulge};
use crate::geom::intersect::{Edge, closest_on_edge};
use crate::op;
use crate::ops::curve_cuts::Geometry;
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

/// Index of the edge of an entity nearest to p (0 when it has none). Edges
/// within 1e-9 m tie and the first wins: at a shared vertex both edges are
/// nearest, and rounding in an arc's end must not pick between them.
pub fn nearest_segment(e: &Shape, p: Vec2) -> usize {
    let (mut best_d, mut best_i) = (f64::INFINITY, 0);
    for (i, ed) in entity_edges(e).iter().enumerate() {
        let d = closest_on_edge(ed, p).d;
        if d < best_d - 1e-9 {
            best_d = d;
            best_i = i;
        }
    }
    best_i
}

fn path_shape(closed: bool, pts: Vec<Vec2>, bulges: Option<Vec<f64>>) -> Shape {
    if closed {
        Shape::Polygon {
            pts,
            bulges,
            holes: None,
        }
    } else {
        Shape::Polyline {
            pts,
            bulges,
            holes: None,
        }
    }
}

/// `Array.prototype.splice(start, 1, ...items)` on a bulge list.
fn splice(v: &mut Vec<f64>, start: usize, items: &[f64]) {
    let start = start.min(v.len());
    let end = (start + 1).min(v.len());
    v.splice(start..end, items.iter().copied());
}

/// Adds a vertex on segment `seg` at the point nearest to p. A line becomes
/// a two-segment polyline; an arc segment is split into two arcs on its
/// circle. Err where the TypeScript reads a segment that is not there.
pub fn insert_vertex(e: &Shape, seg: usize, p: Vec2) -> Result<Geometry, String> {
    let (pts, bulges, closed) = match e {
        Shape::Line { a, b } => {
            let q = closest_on_edge(&Edge::Seg { a: *a, b: *b }, p);
            if q.t <= 1e-9 || q.t >= 1.0 - 1e-9 {
                return Ok(Geometry::Error(
                    "Köşe çizginin iç kısmına eklenmeli.".into(),
                ));
            }
            return Ok(Geometry::Ok(Entity::new(Shape::Polyline {
                pts: vec![*a, q.p, *b],
                bulges: None,
                holes: None,
            })));
        }
        Shape::Polyline { pts, bulges, .. } => (pts, bulges.as_deref(), false),
        Shape::Polygon { pts, bulges, .. } => (pts, bulges.as_deref(), true),
        _ => {
            return Ok(Geometry::Error(
                "Köşe yalnızca çizgi, çoklu çizgi ve kapalı alana eklenebilir.".into(),
            ));
        }
    };
    let n = pts.len();
    let missing = || format!("{}. kenar yok.", seg + 1);
    let bulge = bulge_at(bulges, seg);
    let arc = if is_arc_bulge(bulge) {
        let (Some(&a), Some(&b)) = (pts.get(seg), pts.get((seg + 1) % n.max(1))) else {
            return Err(missing());
        };
        bulge_arc(a, b, bulge)
    } else {
        None
    };
    let edges = entity_edges(e);
    let edge = edges.get(seg).ok_or_else(missing)?;
    let q = closest_on_edge(edge, p);
    if q.t <= 1e-9 || q.t >= 1.0 - 1e-9 {
        return Ok(Geometry::Error(
            "Köşe bir kenarın iç kısmına eklenmeli; mevcut köşeye çok yakın.".into(),
        ));
    }
    let cut = (seg + 1).min(n);
    let mut out_p = pts[..cut].to_vec();
    out_p.push(q.p);
    out_p.extend_from_slice(&pts[cut..]);
    let mut out_b: Vec<f64> = (0..n).map(|i| bulge_at(bulges, i)).collect();
    // Splitting an arc at parameter t: both halves keep the circle.
    let halves = match arc {
        Some(arc) => [
            bulge_of_sweep(arc.sweep * q.t),
            bulge_of_sweep(arc.sweep * (1.0 - q.t)),
        ],
        None => [0.0, 0.0],
    };
    splice(&mut out_b, seg, &halves);
    let clean = clean_bulge_path(&out_p, Some(&out_b), closed, 1e-9);
    Ok(Geometry::Ok(Entity::new(path_shape(
        closed,
        clean.pts,
        clean.bulges,
    ))))
}

/// Removes vertex `index`; its two segments merge into one straight segment.
pub fn remove_vertex(e: &Shape, index: usize) -> Geometry {
    let (pts, bulges, closed) = match e {
        Shape::Polyline { pts, bulges, .. } => (pts, bulges.as_deref(), false),
        Shape::Polygon { pts, bulges, .. } => (pts, bulges.as_deref(), true),
        _ => {
            return Geometry::Error(
                "Köşe yalnızca çoklu çizgi ve kapalı alandan silinebilir.".into(),
            );
        }
    };
    let n = pts.len();
    if n < if closed { 4 } else { 3 } {
        return Geometry::Error(if closed {
            "Kapalı alanda en az üç köşe kalmalı.".into()
        } else {
            "Çoklu çizgide en az iki köşe kalmalı.".into()
        });
    }
    let out_p: Vec<Vec2> = pts
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != index)
        .map(|(_, &p)| p)
        .collect();
    let mut out_b: Vec<f64> = (0..n).map(|i| bulge_at(bulges, i)).collect();
    if !closed && index == 0 {
        out_b.remove(0);
    } else if !closed && index == n - 1 {
        out_b.remove(n - 1);
        out_b[n - 2] = 0.0;
    } else {
        // Segments (index−1 → index) and (index → index+1) become one straight segment.
        let prev = (index % n + n - 1) % n;
        out_b[prev] = 0.0;
        if index < out_b.len() {
            out_b.remove(index);
        }
    }
    let clean = clean_bulge_path(&out_p, Some(&out_b), closed, 1e-9);
    Geometry::Ok(Entity::new(path_shape(closed, clean.pts, clean.bulges)))
}

pub(crate) static OPS: &[Op] = &[
    op!("nearestSegment", |e: Entity, p: Vec2| nearest_segment(
        &e.shape, p
    )),
    op!("insertVertex", |e: Entity, seg: usize, p: Vec2| {
        insert_vertex(&e.shape, seg, p)
    }),
    op!("removeVertex", |e: Entity, index: usize| remove_vertex(
        &e.shape, index
    )),
];
