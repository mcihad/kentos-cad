//! Quick trim and extend, AutoCAD-style: every other visible edge is a
//! boundary (`apps/web/src/model/ops/trim.ts`). The target is a path parameterised by
//! arc length; cuts are the s values where boundaries cross it.

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::arc::norm_angle;
use crate::geom::bulge::{bulge_arc, bulge_at, bulge_of_sweep};
use crate::geom::intersect::{
    Edge, circle_circle, line_circle_params, on_edge_arc, point_at, ray_edge,
};
use crate::jsmath::{TAU, atan2, cos, js_hypot, js_max, js_max_all, js_min, js_min_all, sin};
use crate::op;
use crate::ops::curve_cuts::{
    Cut, Geometry, construction_of, ellipse_of, extend_ellipse, trim_construction, trim_ellipse,
};
use crate::ops::path::{cuts_on, nearest_s, path_of, sub_path};
use crate::vec2::Vec2;

/// Removes the part of `target` between the two cuts that bracket `pick`.
pub fn trim_entity(target: &Entity, pick: Vec2, boundaries: &[Edge]) -> Cut {
    let s = &target.shape;
    match s {
        Shape::Spline { .. } => {
            return Cut::Error(
                "Eğri budanamaz; önce Patlat (X) ile çoklu çizgiye dönüştürün.".into(),
            );
        }
        Shape::Dimension { .. } | Shape::Hatch { .. } => {
            return Cut::Error("Ölçü ve tarama budanamaz.".into());
        }
        _ => {}
    }
    if let Some(g) = ellipse_of(s) {
        return trim_ellipse(&g, pick, boundaries);
    }
    if let Some(c) = construction_of(s) {
        return trim_construction(&c, pick, boundaries);
    }
    let Some(path) = path_of(s) else {
        return Cut::Error("Bu nesne budanamaz.".into());
    };
    let cuts = cuts_on(&path, boundaries);
    let sp = nearest_s(&path, pick);
    let eps = 1e-7 * js_max(1.0, path.length);
    if !path.closed {
        let lo = js_max_all(std::iter::once(0.0).chain(cuts.iter().copied().filter(|&c| c < sp)));
        let hi = js_min_all(
            std::iter::once(path.length).chain(cuts.iter().copied().filter(|&c| c > sp)),
        );
        if lo <= eps && hi >= path.length - eps {
            return Cut::Error("Tıklanan kısmı kesen bir kenar yok.".into());
        }
        let mut pieces = Vec::new();
        if lo > eps {
            pieces.push(sub_path(&path, 0.0, lo, s));
        }
        if hi < path.length - eps {
            pieces.push(sub_path(&path, hi, path.length, s));
        }
        return Cut::Pieces(pieces);
    }
    if cuts.len() < 2 {
        return Cut::Error("Kapalı nesneyi budamak için en az iki kesişim gerekir.".into());
    }
    let lo = cuts
        .iter()
        .copied()
        .rfind(|&c| c < sp)
        .unwrap_or(cuts[cuts.len() - 1] - path.length);
    let hi = cuts
        .iter()
        .copied()
        .find(|&c| c > sp)
        .unwrap_or(cuts[0] + path.length);
    // Keep the complement: from hi around to lo.
    Cut::Pieces(vec![sub_path(&path, hi, lo + path.length, s)])
}

/// Extends the end of `target` nearest to `pick` to the first boundary it meets.
pub fn extend_entity(target: &Entity, pick: Vec2, boundaries: &[Edge]) -> Geometry {
    let s = &target.shape;
    if let Some(g) = ellipse_of(s) {
        return extend_ellipse(&g, pick, boundaries);
    }
    match s {
        Shape::Line { .. } | Shape::Polyline { .. } => {
            let (mut pts, mut bulges) = match s {
                Shape::Line { a, b } => (vec![*a, *b], None),
                Shape::Polyline { pts, bulges, .. } => (pts.clone(), bulges.clone()),
                _ => (Vec::new(), None),
            };
            let n = pts.len();
            // A path needs an end segment to grow.
            if n < 2 {
                return Geometry::Error("Uzatmak için en az iki köşe gerekir.".into());
            }
            let at_end = js_hypot(pick.x - pts[n - 1].x, pick.y - pts[n - 1].y)
                <= js_hypot(pick.x - pts[0].x, pick.y - pts[0].y);
            let seg = if at_end { n - 2 } else { 0 };
            if let Some(arc) = bulge_arc(pts[seg], pts[seg + 1], bulge_at(bulges.as_deref(), seg)) {
                // An arc end segment grows along its own circle.
                let ccw = arc.sweep > 0.0;
                let end_angle = if at_end { arc.a0 + arc.sweep } else { arc.a0 };
                let forward = at_end == ccw; // the tip moves counter-clockwise
                let Some(delta) = arc_reach(
                    arc.c,
                    arc.r,
                    end_angle,
                    forward,
                    TAU - arc.sweep.abs(),
                    boundaries,
                ) else {
                    return Geometry::Error("Bu yönde ulaşılacak bir sınır yok.".into());
                };
                let tip_angle = end_angle + if forward { delta } else { -delta };
                let tip = Vec2::new(
                    arc.c.x + cos(tip_angle) * arc.r,
                    arc.c.y + sin(tip_angle) * arc.r,
                );
                let sw = arc.sweep + if ccw { delta } else { -delta };
                if at_end {
                    pts[n - 1] = tip;
                } else {
                    pts[0] = tip;
                }
                let mut b = bulges.take().unwrap_or_default();
                if seg < b.len() {
                    b[seg] = bulge_of_sweep(sw);
                }
                return Geometry::Ok(Entity::new(Shape::Polyline {
                    pts,
                    bulges: Some(b),
                    holes: None,
                }));
            }
            let tip = if at_end { pts[n - 1] } else { pts[0] };
            let prev = if at_end { pts[n - 2] } else { pts[1] };
            let len = js_hypot(tip.x - prev.x, tip.y - prev.y);
            if len < 1e-12 {
                return Geometry::Error("Sıfır uzunluklu kenar uzatılamaz.".into());
            }
            let dir = Vec2::new((tip.x - prev.x) / len, (tip.y - prev.y) / len);
            let mut best = f64::INFINITY;
            for b in boundaries {
                for t in ray_edge(tip, dir, b, 1e-7) {
                    best = js_min(best, t);
                }
            }
            if !best.is_finite() {
                return Geometry::Error("Bu doğrultuda ulaşılacak bir sınır yok.".into());
            }
            let np = Vec2::new(tip.x + dir.x * best, tip.y + dir.y * best);
            if at_end {
                pts[n - 1] = np;
            } else {
                pts[0] = np;
            }
            if matches!(s, Shape::Line { .. }) {
                return Geometry::Ok(Entity::new(Shape::Line {
                    a: pts[0],
                    b: pts[1],
                }));
            }
            Geometry::Ok(Entity::new(Shape::Polyline {
                pts,
                bulges,
                holes: None,
            }))
        }
        Shape::Arc { c, r, a0, a1 } => {
            let to_start = js_hypot(pick.x - (c.x + cos(*a0) * r), pick.y - (c.y + sin(*a0) * r));
            let to_end = js_hypot(pick.x - (c.x + cos(*a1) * r), pick.y - (c.y + sin(*a1) * r));
            let at_end = to_end <= to_start;
            let gap = TAU - norm_angle(a1 - a0);
            let Some(best) = arc_reach(
                *c,
                *r,
                if at_end { *a1 } else { *a0 },
                at_end,
                gap,
                boundaries,
            ) else {
                return Geometry::Error("Bu yönde ulaşılacak bir sınır yok.".into());
            };
            Geometry::Ok(Entity::new(Shape::Arc {
                c: *c,
                r: *r,
                a0: if at_end { *a0 } else { norm_angle(a0 - best) },
                a1: if at_end { norm_angle(a1 + best) } else { *a1 },
            }))
        }
        _ => Geometry::Error(
            "Yalnızca çizgi, açık çoklu çizgi, yay ve eliptik yay uzatılabilir.".into(),
        ),
    }
}

/// Smallest angle (< `gap`) by which an arc end at `from` can grow (CCW when `ccw`) before it meets a boundary.
fn arc_reach(c: Vec2, r: f64, from: f64, ccw: bool, gap: f64, boundaries: &[Edge]) -> Option<f64> {
    let mut best = f64::INFINITY;
    for b in boundaries {
        for p in circle_hits(c, r, b) {
            let th = atan2(p.y - c.y, p.x - c.x);
            let delta = if ccw {
                norm_angle(th - from)
            } else {
                norm_angle(from - th)
            };
            if delta > 1e-9 && delta < gap - 1e-9 {
                best = js_min(best, delta);
            }
        }
    }
    best.is_finite().then_some(best)
}

/// Intersections of a full circle with an edge (boundary side bounded).
fn circle_hits(c: Vec2, r: f64, b: &Edge) -> Vec<Vec2> {
    match *b {
        Edge::Seg { a, b: bb } => line_circle_params(a, bb, c, r)
            .into_iter()
            .filter(|&t| t >= -1e-9 && t <= 1.0 + 1e-9)
            .map(|t| point_at(&Edge::Seg { a, b: bb }, t))
            .collect(),
        Edge::Arc {
            c: bc,
            r: br,
            a0,
            sweep,
        } => circle_circle(c, r, bc, br)
            .into_iter()
            .filter(|p| on_edge_arc(a0, sweep, atan2(p.y - bc.y, p.x - bc.x)))
            .collect(),
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("trimEntity", |target: Entity,
                       pick: Vec2,
                       boundaries: Vec<Edge>| {
        trim_entity(&target, pick, &boundaries)
    }),
    op!(
        "extendEntity",
        |target: Entity, pick: Vec2, boundaries: Vec<Edge>| extend_entity(
            &target,
            pick,
            &boundaries
        )
    ),
];
