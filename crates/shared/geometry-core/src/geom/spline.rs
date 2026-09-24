//! Centripetal Catmull-Rom curve through fit points (`apps/web/src/model/geom/spline.ts`,
//! Barry–Goldman evaluation, α = 0.5).

use crate::api::Op;
use crate::jsmath::{js_hypot, or};
use crate::op;
use crate::vec2::Vec2;

pub fn catmull_rom(pts: &[Vec2], closed: bool, per_span: f64) -> Vec<Vec2> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }
    let ni = n as i64;
    let at = |i: i64| -> Vec2 {
        if closed {
            return pts[(((i % ni) + ni) % ni) as usize];
        }
        if i < 0 {
            return reflect(pts[1], pts[0]);
        }
        if i >= ni {
            return reflect(pts[n - 2], pts[n - 1]);
        }
        pts[i as usize]
    };
    let mut out = Vec::new();
    let spans = if closed { ni } else { ni - 1 };
    for i in 0..spans {
        let p0 = at(i - 1);
        let p1 = at(i);
        let p2 = at(i + 1);
        let p3 = at(i + 2);
        let t0 = 0.0;
        let t1 = t0 + knot(p0, p1);
        let t2 = t1 + knot(p1, p2);
        let t3 = t2 + knot(p2, p3);
        let mut s = 0.0;
        while s < per_span {
            let t = t1 + ((t2 - t1) * s) / per_span;
            out.push(eval_span([p0, p1, p2, p3], [t0, t1, t2, t3], t));
            s += 1.0;
        }
    }
    out.push(if closed { pts[0] } else { pts[n - 1] });
    out
}

/// Mirror `a` through `b` (phantom end points for open curves).
fn reflect(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(2.0 * b.x - a.x, 2.0 * b.y - a.y)
}

/// Centripetal knot spacing; a tiny epsilon keeps coincident points finite.
fn knot(a: Vec2, b: Vec2) -> f64 {
    or(js_hypot(b.x - a.x, b.y - a.y).sqrt(), 1e-6)
}

fn lerp(a: Vec2, b: Vec2, ta: f64, tb: f64, t: f64) -> Vec2 {
    let d = or(tb - ta, 1e-12);
    let u = (tb - t) / d;
    let v = (t - ta) / d;
    Vec2::new(a.x * u + b.x * v, a.y * u + b.y * v)
}

fn eval_span(p: [Vec2; 4], k: [f64; 4], t: f64) -> Vec2 {
    let a1 = lerp(p[0], p[1], k[0], k[1], t);
    let a2 = lerp(p[1], p[2], k[1], k[2], t);
    let a3 = lerp(p[2], p[3], k[2], k[3], t);
    let b1 = lerp(a1, a2, k[0], k[2], t);
    let b2 = lerp(a2, a3, k[1], k[3], t);
    lerp(b1, b2, k[1], k[2], t)
}

pub(crate) static OPS: &[Op] = &[op!(
    "catmullRom",
    |pts: Vec<Vec2>, closed: bool, per_span: Option<f64>| catmull_rom(
        &pts,
        closed,
        per_span.unwrap_or(16.0)
    )
)];
