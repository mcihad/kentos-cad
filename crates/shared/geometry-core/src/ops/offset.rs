//! Parallel copy of an entity (`apps/web/src/model/ops/offset.ts`): circles and arcs
//! grow or shrink about their centre, paths are offset with mitred corners,
//! ellipses and construction lines by their own rules.

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::bulge::has_bulges;
use crate::geom::intersect::{Edge, closest_on_edge};
use crate::geom::offset::{OffsetResult, offset_bulge_path, offset_path, side_of};
use crate::jsmath::{js_hypot, js_min};
use crate::op;
use crate::ops::curve_cuts::{
    Geometry, construction_of, ellipse_of, offset_construction, offset_ellipse,
};
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

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

/// Parallel copy at `distance`, on the side of `through`.
pub fn offset_entity(e: &Shape, distance: f64, through: Vec2) -> Geometry {
    if !(distance > 0.0) {
        return Geometry::Error("Öteleme mesafesi sıfırdan büyük olmalı.".into());
    }
    match e {
        Shape::Line { a, b } => {
            let side = side_of(&[*a, *b], false, through);
            let pts = offset_path(&[*a, *b], f64::from(side) * distance, false);
            match pts[..] {
                [a, b] => Geometry::Ok(Entity::new(Shape::Line { a, b })),
                _ => Geometry::Error("Öteleme sonucu geçerli bir şekil oluşmadı.".into()),
            }
        }
        Shape::Polyline { pts, bulges, .. } | Shape::Polygon { pts, bulges, .. } => {
            let closed = matches!(e, Shape::Polygon { .. });
            if let Some(bs) = bulges.as_deref()
                && has_bulges(Some(bs))
            {
                let d = f64::from(bulged_side(e, through)) * distance;
                return match offset_bulge_path(pts, bs, d, closed) {
                    OffsetResult::Path { pts, bulges } => {
                        Geometry::Ok(Entity::new(path_shape(closed, pts, Some(bulges))))
                    }
                    OffsetResult::Error { error } => Geometry::Error(error),
                };
            }
            let side = side_of(pts, closed, through);
            let out = offset_path(pts, f64::from(side) * distance, closed);
            if out.len() >= 2 {
                Geometry::Ok(Entity::new(path_shape(closed, out, None)))
            } else {
                Geometry::Error("Öteleme sonucu geçerli bir şekil oluşmadı.".into())
            }
        }
        Shape::Circle { c, r } | Shape::Arc { c, r, .. } => {
            let outside = js_hypot(through.x - c.x, through.y - c.y) > *r;
            let r = r + if outside { distance } else { -distance };
            if r <= 1e-9 {
                return Geometry::Error(
                    "Yarıçap sıfırın altına düşüyor; daha küçük bir mesafe girin.".into(),
                );
            }
            Geometry::Ok(Entity::new(match e {
                Shape::Arc { a0, a1, .. } => Shape::Arc {
                    c: *c,
                    r,
                    a0: *a0,
                    a1: *a1,
                },
                _ => Shape::Circle { c: *c, r },
            }))
        }
        _ => {
            if let Some(g) = ellipse_of(e) {
                return offset_ellipse(&g, distance, through);
            }
            if let Some(c) = construction_of(e) {
                return offset_construction(&c, distance, through);
            }
            Geometry::Error(
                "Yalnızca çizgi, çoklu çizgi, kapalı alan, daire, yay, elips ve yardımcı çizgiler ötelenebilir."
                    .into(),
            )
        }
    }
}

/// The distance “Noktadan geç” (through) offsets by: from `p` to the
/// object's nearest edge, so the copy passes through `p` (the offset tool's
/// `distanceFor`, docs/adr/0047); +∞ for an object without edges.
pub fn through_distance(e: &Shape, p: Vec2) -> f64 {
    let mut d = f64::INFINITY;
    for ed in entity_edges(e) {
        d = js_min(d, closest_on_edge(&ed, p).d);
    }
    d
}

/// Side of a bulged path a point lies on (+1 left of travel), judged at the nearest edge.
fn bulged_side(e: &Shape, p: Vec2) -> i32 {
    let (mut best_d, mut best_side) = (f64::INFINITY, 1);
    for ed in entity_edges(e) {
        let c = closest_on_edge(&ed, p);
        if c.d >= best_d {
            continue;
        }
        let side = match ed {
            Edge::Seg { a, b } => {
                if (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x) >= 0.0 {
                    1
                } else {
                    -1
                }
            }
            Edge::Arc { c, r, sweep, .. } => {
                // Inside the circle is left of a counter-clockwise arc.
                let inside = js_hypot(p.x - c.x, p.y - c.y) < r;
                if inside == (sweep > 0.0) { 1 } else { -1 }
            }
        };
        best_d = c.d;
        best_side = side;
    }
    best_side
}

pub(crate) static OPS: &[Op] = &[
    op!("offsetEntity", |e: Entity, distance: f64, through: Vec2| {
        offset_entity(&e.shape, distance, through)
    }),
    op!("offsetThroughDistance", |e: Entity, p: Vec2| {
        through_distance(&e.shape, p)
    }),
];
