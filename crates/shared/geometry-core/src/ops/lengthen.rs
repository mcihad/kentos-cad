//! Uzat-kısalt, AutoCAD LENGTHEN (`apps/web/src/model/ops/lengthen.ts`): a line, an
//! arc or an open polyline gets a new total length, changed at one end.
//! Shortening cuts the path there (arcs exactly); lengthening continues the
//! end segment along its direction or on its own circle.

use crate::api::Op;
use crate::entity::{Entity, Shape, entity_geometry};
use crate::geom::arc::{norm_angle, sweep as ccw_sweep};
use crate::geom::bulge::{bulge_arc, bulge_at, bulge_of_sweep};
use crate::geom::intersect::Edge;
use crate::jsmath::{PI, TAU, atan2, cos, js_hypot, js_max, js_sign, or, sin};
use crate::op;
use crate::ops::curve_cuts::Geometry;
use crate::ops::path::{nearest_s, path_of, sub_path};
use crate::vec2::Vec2;

fn can_lengthen(e: &Shape) -> bool {
    matches!(
        e,
        Shape::Line { .. } | Shape::Arc { .. } | Shape::Polyline { .. }
    )
}

/// Current length of what Uzat-kısalt accepts, or None.
pub fn length_of(e: &Shape) -> Option<f64> {
    if !can_lengthen(e) {
        return None;
    }
    path_of(e).map(|p| p.length)
}

/// Whether a click at p is nearer the end (true) or the start of the entity.
pub fn near_end(e: &Shape, p: Vec2) -> bool {
    path_of(e).is_some_and(|path| nearest_s(&path, p) > path.length / 2.0)
}

pub fn lengthen_entity(e: &Entity, at_end: bool, new_length: f64) -> Geometry {
    let s = &e.shape;
    if !can_lengthen(s) {
        return Geometry::Error("Uzat-kısalt çizgi, yay ve açık çoklu çizgide çalışır.".into());
    }
    let Some(path) = path_of(s) else {
        return Geometry::Error("Nesnenin uzunluğu yok.".into());
    };
    if !(new_length > 1e-9) {
        return Geometry::Error("Yeni uzunluk sıfırdan büyük olmalı.".into());
    }
    let l = path.length;
    if (new_length - l).abs() <= 1e-12 * js_max(1.0, l) {
        return Geometry::Ok(entity_geometry(e));
    }
    if new_length < l {
        let g = if at_end {
            sub_path(&path, 0.0, new_length, s)
        } else {
            sub_path(&path, l - new_length, l, s)
        };
        // A short polyline stays a polyline.
        if let (Shape::Polyline { .. }, Shape::Line { a, b }) = (s, &g.shape) {
            return Geometry::Ok(Entity::new(Shape::Polyline {
                pts: vec![*a, *b],
                bulges: None,
                holes: None,
            }));
        }
        return Geometry::Ok(g);
    }
    let delta = new_length - l;
    match s {
        Shape::Line { a, b } => {
            let (from, to) = if at_end { (*a, *b) } else { (*b, *a) };
            let k = new_length / l;
            let end = Vec2::new(from.x + (to.x - from.x) * k, from.y + (to.y - from.y) * k);
            Geometry::Ok(Entity::new(if at_end {
                Shape::Line { a: *a, b: end }
            } else {
                Shape::Line { a: end, b: *b }
            }))
        }
        Shape::Arc { c, r, a0, a1 } => {
            let sw = ccw_sweep(*a0, *a1) + delta / r;
            if sw >= TAU - 1e-9 {
                return Geometry::Error(
                    "Yay bu uzunlukta kendini kapatır; en çok bir tam daireden kısa olabilir."
                        .into(),
                );
            }
            Geometry::Ok(Entity::new(if at_end {
                Shape::Arc {
                    c: *c,
                    r: *r,
                    a0: *a0,
                    a1: norm_angle(a0 + sw),
                }
            } else {
                Shape::Arc {
                    c: *c,
                    r: *r,
                    a0: norm_angle(a1 - sw),
                    a1: *a1,
                }
            }))
        }
        Shape::Polyline {
            pts: src,
            bulges: src_bulges,
            ..
        } => {
            let n = src.len();
            // pathOf already refused it; the indexing below needs an end segment.
            if n < 2 {
                return Geometry::Error("Uzat-kısalt en az iki köşe ister.".into());
            }
            let mut pts = src.clone();
            let mut bulges: Vec<f64> = (0..n).map(|i| bulge_at(src_bulges.as_deref(), i)).collect();
            // The end segment, walked away from the rest of the path.
            let seg = if at_end { n - 2 } else { 0 };
            let (fixed, moving) = if at_end {
                (pts[n - 2], pts[n - 1])
            } else {
                (pts[1], pts[0])
            };
            match bulge_arc(pts[seg], pts[seg + 1], bulges[seg]) {
                None => {
                    let len = js_hypot(moving.x - fixed.x, moving.y - fixed.y);
                    let k = (len + delta) / len;
                    let end = Vec2::new(
                        fixed.x + (moving.x - fixed.x) * k,
                        fixed.y + (moving.y - fixed.y) * k,
                    );
                    pts[if at_end { n - 1 } else { 0 }] = end;
                }
                Some(arc) => {
                    // Grow the arc on its circle: the sweep gains delta / r in its own direction.
                    let sw = arc.sweep + js_sign(arc.sweep) * (delta / arc.r);
                    if sw.abs() >= TAU - 1e-9 {
                        return Geometry::Error("Yay parçası bu uzunlukta kendini kapatır.".into());
                    }
                    let a0 = if at_end {
                        arc.a0
                    } else {
                        arc.a0 + arc.sweep - sw
                    };
                    let start = Vec2::new(arc.c.x + cos(a0) * arc.r, arc.c.y + sin(a0) * arc.r);
                    let end = Vec2::new(
                        arc.c.x + cos(a0 + sw) * arc.r,
                        arc.c.y + sin(a0 + sw) * arc.r,
                    );
                    if at_end {
                        pts[n - 1] = end;
                    } else {
                        pts[0] = start;
                    }
                    bulges[seg] = bulge_of_sweep(sw);
                }
            }
            let bulges = bulges.iter().any(|&x| x != 0.0).then_some(bulges);
            Geometry::Ok(Entity::new(Shape::Polyline {
                pts,
                bulges,
                holes: None,
            }))
        }
        _ => Geometry::Error("Uzat-kısalt çizgi, yay ve açık çoklu çizgide çalışır.".into()),
    }
}

/// The total length that puts the moving end at the point nearest to p.
pub fn length_toward(e: &Shape, at_end: bool, p: Vec2) -> Option<f64> {
    let path = path_of(e)?;
    if !can_lengthen(e) {
        return None;
    }
    let l = path.length;
    let edges = &path.edges;
    let last = if at_end {
        edges[edges.len() - 1]
    } else {
        edges[0]
    };
    let before = if at_end {
        path.cum[edges.len() - 1]
    } else {
        0.0
    };
    let last_len = if at_end {
        l - path.cum[edges.len() - 1]
    } else if edges.len() > 1 {
        path.cum[1]
    } else {
        l
    };
    let inside = if at_end {
        nearest_s(&path, p)
    } else {
        l - nearest_s(&path, p)
    };
    match last {
        Edge::Seg { a, b } => {
            let (from, to) = if at_end { (a, b) } else { (b, a) };
            let len = js_hypot(to.x - from.x, to.y - from.y);
            let t = ((p.x - from.x) * (to.x - from.x) + (p.y - from.y) * (to.y - from.y)) / len;
            Some(if t > len { before + t } else { inside })
        }
        Edge::Arc { c, r, a0, sweep } => {
            // Arc: angle travelled from the fixed end of the segment, in its direction.
            let dir = or(js_sign(sweep), 1.0);
            let fixed_angle = if at_end { a0 } else { a0 + sweep };
            let walk = if at_end { dir } else { -dir };
            let theta = norm_angle((atan2(p.y - c.y, p.x - c.x) - fixed_angle) * walk);
            let travelled = theta * r;
            // Past the end but not so far round that it would rather be the start again.
            Some(if travelled > last_len && theta < PI + sweep.abs() / 2.0 {
                l - last_len + travelled
            } else {
                inside
            })
        }
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("lengthOf", |e: Entity| length_of(&e.shape)),
    op!("nearEnd", |e: Entity, p: Vec2| near_end(&e.shape, p)),
    op!(
        "lengthenEntity",
        |e: Entity, at_end: bool, new_length: f64| lengthen_entity(&e, at_end, new_length)
    ),
    op!("lengthToward", |e: Entity, at_end: bool, p: Vec2| {
        length_toward(&e.shape, at_end, p)
    }),
];
