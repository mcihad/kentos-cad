//! Angles, circles and arcs (`apps/web/src/model/geom/arc.ts`).

use crate::api::Op;
use crate::jsmath::{TAU, atan2, cos, js_hypot, js_max, sin, tan};
use crate::op;
use crate::vec2::Vec2;

/// Normalizes an angle to [0, 2π).
pub fn norm_angle(a: f64) -> f64 {
    let r = a % TAU;
    if r < 0.0 { r + TAU } else { r }
}

/// CCW sweep from a0 to a1 in (0, 2π]; equal angles mean a full turn.
pub fn sweep(a0: f64, a1: f64) -> f64 {
    let s = norm_angle(a1 - a0);
    if s < 1e-12 { TAU } else { s }
}

/// Whether θ lies on the CCW arc from a0 spanning `sw` (inclusive, with tolerance).
pub fn on_arc(theta: f64, a0: f64, sw: f64, eps: f64) -> bool {
    if sw >= TAU - eps {
        return true;
    }
    let d = norm_angle(theta - a0);
    d <= sw + eps || d >= TAU - eps
}

pub const ON_ARC_EPS: f64 = 1e-9;

/// Position of θ along the arc as 0..1.
pub fn arc_param(theta: f64, a0: f64, sw: f64) -> f64 {
    norm_angle(theta - a0) / sw
}

pub fn point_on_circle(c: Vec2, r: f64, a: f64) -> Vec2 {
    Vec2::new(c.x + cos(a) * r, c.y + sin(a) * r)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub c: Vec2,
    pub r: f64,
}

crate::json_struct!(Circle { c, r });

pub fn circle_through(p1: Vec2, p2: Vec2, p3: Vec2) -> Option<Circle> {
    let ax = p2.x - p1.x;
    let ay = p2.y - p1.y;
    let bx = p3.x - p1.x;
    let by = p3.y - p1.y;
    let d = 2.0 * (ax * by - ay * bx);
    if d.abs() < 1e-12 {
        return None; // collinear
    }
    let a2 = ax * ax + ay * ay;
    let b2 = bx * bx + by * by;
    let cx = (by * a2 - ay * b2) / d;
    let cy = (ax * b2 - bx * a2) / d;
    Some(Circle {
        c: Vec2::new(p1.x + cx, p1.y + cy),
        r: js_hypot(cx, cy),
    })
}

/// An arc running counter-clockwise from `a0` to `a1` (radians from east).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcGeom {
    pub c: Vec2,
    pub r: f64,
    pub a0: f64,
    pub a1: f64,
}

crate::json_struct!(ArcGeom { c, r, a0, a1 });

/// Arc from start through mid to end, stored CCW (a clockwise pick order swaps the ends).
pub fn arc_through(start: Vec2, mid: Vec2, end: Vec2) -> Option<ArcGeom> {
    let Circle { c, r } = circle_through(start, mid, end)?;
    let a_s = atan2(start.y - c.y, start.x - c.x);
    let a_m = atan2(mid.y - c.y, mid.x - c.x);
    let a_e = atan2(end.y - c.y, end.x - c.x);
    let ccw = norm_angle(a_m - a_s) < norm_angle(a_e - a_s);
    Some(if ccw {
        ArcGeom {
            c,
            r,
            a0: norm_angle(a_s),
            a1: norm_angle(a_e),
        }
    } else {
        ArcGeom {
            c,
            r,
            a0: norm_angle(a_e),
            a1: norm_angle(a_s),
        }
    })
}

pub const DEFAULT_STEP: f64 = TAU / 72.0;

pub fn tessellate_arc(a: &ArcGeom, max_step: f64) -> Vec<Vec2> {
    let sw = sweep(a.a0, a.a1);
    let n = js_max(2.0, (sw / max_step).ceil());
    let mut pts = Vec::new();
    let mut i = 0.0;
    while i <= n {
        pts.push(point_on_circle(a.c, a.r, a.a0 + (sw * i) / n));
        i += 1.0;
    }
    pts
}

pub fn arc_start(a: &ArcGeom) -> Vec2 {
    point_on_circle(a.c, a.r, a.a0)
}
pub fn arc_end(a: &ArcGeom) -> Vec2 {
    point_on_circle(a.c, a.r, a.a1)
}
pub fn arc_mid(a: &ArcGeom) -> Vec2 {
    point_on_circle(a.c, a.r, a.a0 + sweep(a.a0, a.a1) / 2.0)
}
pub fn arc_length(a: &ArcGeom) -> f64 {
    a.r * sweep(a.a0, a.a1)
}
/// DXF bulge (tan(θ/4)) of a counter-clockwise arc.
pub fn bulge_from_arc(a: &ArcGeom) -> f64 {
    tan(sweep(a.a0, a.a1) / 4.0)
}

pub(crate) static OPS: &[Op] = &[
    op!("normAngle", |a: f64| norm_angle(a)),
    op!("sweep", |a0: f64, a1: f64| sweep(a0, a1)),
    op!("onArc", |theta: f64, a0: f64, sw: f64, eps: Option<f64>| {
        on_arc(theta, a0, sw, eps.unwrap_or(ON_ARC_EPS))
    }),
    op!("arcParam", |theta: f64, a0: f64, sw: f64| arc_param(
        theta, a0, sw
    )),
    op!("pointOnCircle", |c: Vec2, r: f64, a: f64| {
        point_on_circle(c, r, a)
    }),
    op!("circleThrough", |p1: Vec2, p2: Vec2, p3: Vec2| {
        circle_through(p1, p2, p3)
    }),
    op!("arcThrough", |s: Vec2, m: Vec2, e: Vec2| arc_through(
        s, m, e
    )),
    op!("tessellateArc", |a: ArcGeom, step: Option<f64>| {
        tessellate_arc(&a, step.unwrap_or(DEFAULT_STEP))
    }),
    op!("arcStart", |a: ArcGeom| arc_start(&a)),
    op!("arcEnd", |a: ArcGeom| arc_end(&a)),
    op!("arcMid", |a: ArcGeom| arc_mid(&a)),
    op!("arcLength", |a: ArcGeom| arc_length(&a)),
    op!("bulgeFromArc", |a: ArcGeom| bulge_from_arc(&a)),
];
