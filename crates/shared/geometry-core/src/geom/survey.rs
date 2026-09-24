//! Surveying point constructions, "Koordinat hesap makinası"
//! (`apps/web/src/model/geom/survey.ts`): abscissa along A→B, ordinate square to it
//! and positive to the right; horizontal angles clockwise.

use crate::api::Op;
use crate::geom::intersect::{circle_circle, line_line};
use crate::jsmath::{PI, atan2, cos, js_cmp, js_hypot, sin, stable_sort};
use crate::op;
use crate::vec2::Vec2;

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn unit(a: Vec2, b: Vec2) -> Option<Vec2> {
    let d = sub(b, a);
    let l = js_hypot(d.x, d.y);
    if l < 1e-12 {
        None
    } else {
        Some(Vec2::new(d.x / l, d.y / l))
    }
}

/// Yan nokta: `absis` along A→B from A, `ordinat` square to it (+ right).
pub fn side_point(a: Vec2, b: Vec2, absis: f64, ordinat: f64) -> Option<Vec2> {
    let u = unit(a, b)?;
    // Right of travel is the direction turned −90°: (u.y, −u.x).
    Some(Vec2::new(
        a.x + u.x * absis + u.y * ordinat,
        a.y + u.y * absis - u.x * ordinat,
    ))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Offsets {
    pub absis: f64,
    pub ordinat: f64,
}

crate::json_struct!(Offsets { absis, ordinat });

/// Absis and ordinat (+ right) of p relative to the line A→B.
pub fn side_offsets(a: Vec2, b: Vec2, p: Vec2) -> Option<Offsets> {
    let u = unit(a, b)?;
    let v = sub(p, a);
    Some(Offsets {
        absis: v.x * u.x + v.y * u.y,
        ordinat: v.x * u.y - v.y * u.x,
    })
}

/// Kenar kesişimi: points at d1 from A and d2 from B, the one right of A→B first.
pub fn distance_intersection(a: Vec2, b: Vec2, d1: f64, d2: f64) -> Vec<Vec2> {
    if !(d1 > 0.0) || !(d2 > 0.0) {
        return Vec::new();
    }
    let mut pts = circle_circle(a, d1, b, d2);
    let ord = |p: &Vec2| side_offsets(a, b, *p).map_or(0.0, |o| o.ordinat);
    stable_sort(&mut pts, &mut |p, q| js_cmp(ord(q) - ord(p), 0.0));
    pts
}

/// Doğru kesişimi: where the lines A→B and C→D (unbounded) cross.
pub fn line_intersection(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<Vec2> {
    line_line(a, b, c, d).map(|h| h.p)
}

/// Hat üzerinde nokta: `distance` from A towards B (negative: behind A).
pub fn along_line(a: Vec2, b: Vec2, distance: f64) -> Option<Vec2> {
    unit(a, b).map(|u| Vec2::new(a.x + u.x * distance, a.y + u.y * distance))
}

/// Açı-mesafe: from station S, `angle` (radians) clockwise from S→R, then `distance`.
pub fn polar_point(s: Vec2, r: Vec2, angle: f64, distance: f64) -> Option<Vec2> {
    let u = unit(s, r)?;
    let c = cos(-angle);
    let n = sin(-angle);
    Some(Vec2::new(
        s.x + (u.x * c - u.y * n) * distance,
        s.y + (u.x * n + u.y * c) * distance,
    ))
}

/// Clockwise angle (radians, 0…2π) at S from S→R to S→P.
pub fn clockwise_angle(s: Vec2, r: Vec2, p: Vec2) -> f64 {
    let a = atan2(r.y - s.y, r.x - s.x) - atan2(p.y - s.y, p.x - s.x);
    ((a % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI)
}

pub(crate) static OPS: &[Op] = &[
    op!("sidePoint", |a: Vec2, b: Vec2, absis: f64, ordinat: f64| {
        side_point(a, b, absis, ordinat)
    }),
    op!("sideOffsets", |a: Vec2, b: Vec2, p: Vec2| side_offsets(
        a, b, p
    )),
    op!(
        "distanceIntersection",
        |a: Vec2, b: Vec2, d1: f64, d2: f64| distance_intersection(a, b, d1, d2)
    ),
    op!("lineIntersection", |a: Vec2, b: Vec2, c: Vec2, d: Vec2| {
        line_intersection(a, b, c, d)
    }),
    op!("alongLine", |a: Vec2, b: Vec2, distance: f64| along_line(
        a, b, distance
    )),
    op!("polarPoint", |s: Vec2,
                       r: Vec2,
                       angle: f64,
                       distance: f64| {
        polar_point(s, r, angle, distance)
    }),
    op!("clockwiseAngle", |s: Vec2, r: Vec2, p: Vec2| {
        clockwise_angle(s, r, p)
    }),
];
