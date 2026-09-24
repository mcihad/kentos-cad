//! 2D affine transforms `[a, b, c, d, e, f]` (`apps/web/src/model/geom/affine.ts`):
//! x' = a·x + c·y + e, y' = b·x + d·y + f.

use crate::api::Op;
use crate::jsmath::{cos, or, sin};
use crate::op;
use crate::vec2::Vec2;

pub type Affine = [f64; 6];

pub const IDENTITY: Affine = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
const ORIGIN: Vec2 = Vec2::new(0.0, 0.0);

pub fn translation(dx: f64, dy: f64) -> Affine {
    [1.0, 0.0, 0.0, 1.0, dx, dy]
}

/// Rotation by `angle` radians (CCW) around `o`.
pub fn rotation(angle: f64, o: Vec2) -> Affine {
    let c = cos(angle);
    let s = sin(angle);
    [
        c,
        s,
        -s,
        c,
        o.x - c * o.x + s * o.y,
        o.y - s * o.x - c * o.y,
    ]
}

/// Uniform scale around `o`.
pub fn scaling(s: f64, o: Vec2) -> Affine {
    [s, 0.0, 0.0, s, o.x * (1.0 - s), o.y * (1.0 - s)]
}

/// Reflection across the line through `p` and `q`.
pub fn mirror(p: Vec2, q: Vec2) -> Affine {
    let dx = q.x - p.x;
    let dy = q.y - p.y;
    let l2 = or(dx * dx + dy * dy, 1.0);
    let a = (dx * dx - dy * dy) / l2;
    let b = (2.0 * dx * dy) / l2;
    [
        a,
        b,
        b,
        -a,
        p.x - a * p.x - b * p.y,
        p.y - b * p.x + a * p.y,
    ]
}

/// m2 ∘ m1: first m1, then m2.
pub fn compose(m2: &Affine, m1: &Affine) -> Affine {
    let [a1, b1, c1, d1, e1, f1] = *m1;
    let [a2, b2, c2, d2, e2, f2] = *m2;
    [
        a2 * a1 + c2 * b1,
        b2 * a1 + d2 * b1,
        a2 * c1 + c2 * d1,
        b2 * c1 + d2 * d1,
        a2 * e1 + c2 * f1 + e2,
        b2 * e1 + d2 * f1 + f2,
    ]
}

pub fn apply(m: &Affine, p: Vec2) -> Vec2 {
    Vec2::new(
        m[0] * p.x + m[2] * p.y + m[4],
        m[1] * p.x + m[3] * p.y + m[5],
    )
}

/// Only the linear part (direction vectors).
pub fn apply_linear(m: &Affine, v: Vec2) -> Vec2 {
    Vec2::new(m[0] * v.x + m[2] * v.y, m[1] * v.x + m[3] * v.y)
}

pub fn determinant(m: &Affine) -> f64 {
    m[0] * m[3] - m[1] * m[2]
}

/// Length scale factor of a similarity transform.
pub fn length_scale(m: &Affine) -> f64 {
    determinant(m).abs().sqrt()
}

/// Whether the transform flips orientation (a mirror).
pub fn is_reflection(m: &Affine) -> bool {
    determinant(m) < 0.0
}

pub(crate) static OPS: &[Op] = &[
    op!("translation", |dx: f64, dy: f64| translation(dx, dy)),
    op!("rotation", |angle: f64, o: Option<Vec2>| rotation(
        angle,
        o.unwrap_or(ORIGIN)
    )),
    op!("scaling", |s: f64, o: Option<Vec2>| scaling(
        s,
        o.unwrap_or(ORIGIN)
    )),
    op!("mirror", |p: Vec2, q: Vec2| mirror(p, q)),
    op!("compose", |m2: Affine, m1: Affine| compose(&m2, &m1)),
    op!("apply", |m: Affine, p: Vec2| apply(&m, p)),
    op!("applyLinear", |m: Affine, v: Vec2| apply_linear(&m, v)),
    op!("determinant", |m: Affine| determinant(&m)),
    op!("lengthScale", |m: Affine| length_scale(&m)),
    op!("isReflection", |m: Affine| is_reflection(&m)),
];
