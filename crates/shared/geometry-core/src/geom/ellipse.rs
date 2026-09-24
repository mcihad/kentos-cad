//! Ellipses and elliptical arcs in the DXF ELLIPSE form (`apps/web/src/model/geom/ellipse.ts`):
//! P(t) = c + major·cos t + minor·sin t, minor = major turned +90° × ratio.

use crate::api::Op;
use crate::geom::arc::{norm_angle, sweep};
use crate::geom::intersect::{line_circle_params, tangent_points};
use crate::jsmath::{PI, TAU, atan2, cos, js_hypot, js_max, js_min, sin};
use crate::op;
use crate::vec2::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EllipseGeom {
    pub c: Vec2,
    pub major: Vec2,
    pub ratio: f64,
    pub t0: f64,
    pub t1: f64,
}

crate::json_struct!(EllipseGeom {
    c,
    major,
    ratio,
    t0,
    t1
});

const ORIGIN: Vec2 = Vec2::new(0.0, 0.0);

pub fn minor_axis(e: &EllipseGeom) -> Vec2 {
    Vec2::new(-e.major.y * e.ratio, e.major.x * e.ratio)
}
pub fn major_length(e: &EllipseGeom) -> f64 {
    js_hypot(e.major.x, e.major.y)
}
pub fn ellipse_sweep(e: &EllipseGeom) -> f64 {
    sweep(e.t0, e.t1)
}
pub fn is_full_ellipse(e: &EllipseGeom) -> bool {
    ellipse_sweep(e) >= TAU - 1e-12
}

pub fn ellipse_point(e: &EllipseGeom, t: f64) -> Vec2 {
    let m = minor_axis(e);
    let c = cos(t);
    let s = sin(t);
    Vec2::new(
        e.c.x + e.major.x * c + m.x * s,
        e.c.y + e.major.y * c + m.y * s,
    )
}

/// dP/dt (not normalised).
pub fn ellipse_derivative(e: &EllipseGeom, t: f64) -> Vec2 {
    let m = minor_axis(e);
    let c = cos(t);
    let s = sin(t);
    Vec2::new(-e.major.x * s + m.x * c, -e.major.y * s + m.y * c)
}

/// World point → the ellipse's unit-circle space.
fn to_unit(e: &EllipseGeom, p: Vec2) -> Vec2 {
    let m = minor_axis(e);
    let det = e.major.x * m.y - e.major.y * m.x;
    let dx = p.x - e.c.x;
    let dy = p.y - e.c.y;
    Vec2::new(
        (m.y * dx - m.x * dy) / det,
        (-e.major.y * dx + e.major.x * dy) / det,
    )
}

pub fn param_of_point(e: &EllipseGeom, p: Vec2) -> f64 {
    let q = to_unit(e, p);
    norm_angle(atan2(q.y, q.x))
}

/// Parameter of the point seen from the centre at `angle` from the major axis.
pub fn param_at_polar(e: &EllipseGeom, angle: f64) -> f64 {
    norm_angle(atan2(sin(angle) / e.ratio, cos(angle)))
}

pub fn on_ellipse(e: &EllipseGeom, t: f64, eps: f64) -> bool {
    let sw = ellipse_sweep(e);
    if sw >= TAU - eps {
        return true;
    }
    let d = norm_angle(t - e.t0);
    d <= sw + eps || d >= TAU - eps
}

/// Points along the curve; a whole ellipse is a ring (first point not repeated).
pub fn tessellate_ellipse(e: &EllipseGeom, per_turn: f64) -> Vec<Vec2> {
    let sw = ellipse_sweep(e);
    let full = sw >= TAU - 1e-12;
    let n = js_max(
        if full { 16.0 } else { 4.0 },
        ((sw / TAU) * per_turn).ceil(),
    );
    let end = if full { n } else { n + 1.0 };
    let mut out = Vec::new();
    let mut i = 0.0;
    while i < end {
        out.push(ellipse_point(e, e.t0 + (sw * i) / n));
        i += 1.0;
    }
    out
}

/// Arc length by composite Simpson (2048 intervals).
pub fn ellipse_length(e: &EllipseGeom) -> f64 {
    let sw = ellipse_sweep(e);
    let n = 2048;
    let h = sw / n as f64;
    let f = |t: f64| {
        let d = ellipse_derivative(e, t);
        js_hypot(d.x, d.y)
    };
    let mut s = f(e.t0) + f(e.t0 + sw);
    for i in 1..n {
        s += f(e.t0 + i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
    }
    (s * h) / 3.0
}

/// Area of a whole ellipse (π·a·b).
pub fn ellipse_area(e: &EllipseGeom) -> f64 {
    PI * major_length(e) * major_length(e) * e.ratio
}

/// Parameter of the curve point nearest to p (within the arc range). The
/// nearest of 65 samples tells which way the distance falls; the foot between
/// it and the next sample that way is found by Newton steps on
/// f(t) = (P(t) − p)·P′(t), bisecting whenever a step would leave that
/// bracket. When the distance only grows past an arc end, the end is the
/// answer. Offsets are taken from p first, so TM coordinates do not cancel.
pub fn closest_param(e: &EllipseGeom, p: Vec2) -> f64 {
    let sw = ellipse_sweep(e);
    let full = sw >= TAU - 1e-12;
    let m = minor_axis(e);
    let (ox, oy) = (e.c.x - p.x, e.c.y - p.y);
    let samples = 64.0;
    let at = |i: f64| e.t0 + (sw * i) / samples;
    // Offset from p to P(t), and f at t.
    let rel = |t: f64| {
        let (c, s) = (cos(t), sin(t));
        (ox + e.major.x * c + m.x * s, oy + e.major.y * c + m.y * s)
    };
    let slope = |t: f64| {
        let (c, s) = (cos(t), sin(t));
        let x = ox + e.major.x * c + m.x * s;
        let y = oy + e.major.y * c + m.y * s;
        x * (-e.major.x * s + m.x * c) + y * (-e.major.y * s + m.y * c)
    };
    let (mut bi, mut bd) = (0.0, f64::INFINITY);
    let mut i = 0.0;
    while i <= samples {
        let (x, y) = rel(at(i));
        let d = x * x + y * y;
        if d < bd {
            bi = i;
            bd = d;
        }
        i += 1.0;
    }
    let tb = at(bi);
    let fb = slope(tb);
    // Falling ahead (fb < 0): the foot lies before the next sample; rising: before the previous one.
    let j = if fb < 0.0 {
        bi + 1.0
    } else if fb > 0.0 {
        bi - 1.0
    } else {
        bi
    };
    if j == bi || (!full && (j < 0.0 || j > samples)) {
        return norm_angle(tb);
    }
    let tj = at(j);
    let (mut lo, mut hi) = (js_min(tb, tj), js_max(tb, tj));
    // The sign has to change across the bracket; otherwise the sample stays.
    if !(slope(lo) < 0.0 && slope(hi) > 0.0) {
        return norm_angle(tb);
    }
    let mut t = tb;
    for _ in 0..100 {
        let (c, s) = (cos(t), sin(t));
        let ux = e.major.x * c + m.x * s;
        let uy = e.major.y * c + m.y * s;
        let x = ox + ux;
        let y = oy + uy;
        let dx = -e.major.x * s + m.x * c;
        let dy = -e.major.y * s + m.y * c;
        let f = x * dx + y * dy;
        if f == 0.0 {
            break;
        }
        if f < 0.0 {
            lo = t;
        } else {
            hi = t;
        }
        // f′ = |P′|² + (P − p)·P″, with P″ = −(P − c).
        let fp = dx * dx + dy * dy - (x * ux + y * uy);
        let newton = t - f / fp;
        let next = if fp > 0.0 && newton > lo && newton < hi {
            newton
        } else {
            (lo + hi) / 2.0
        };
        let scale = js_max(1.0, t.abs());
        let done = (next - t).abs() <= 4e-16 * scale || hi - lo <= 4e-16 * scale;
        t = next;
        if done {
            break;
        }
    }
    norm_angle(t)
}

/// A crossing of a line with the curve: line parameter and ellipse parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHit {
    pub u: f64,
    pub t: f64,
}

crate::json_struct!(LineHit { u, t });

pub fn line_ellipse(e: &EllipseGeom, a: Vec2, b: Vec2) -> Vec<LineHit> {
    let qa = to_unit(e, a);
    let qb = to_unit(e, b);
    line_circle_params(qa, qb, ORIGIN, 1.0)
        .into_iter()
        .map(|u| LineHit {
            u,
            t: norm_angle(atan2(qa.y + (qb.y - qa.y) * u, qa.x + (qb.x - qa.x) * u)),
        })
        .filter(|h| on_ellipse(e, h.t, 1e-9))
        .collect()
}

/// Points where lines from p touch the curve (empty when p is inside).
pub fn ellipse_tangent_points(e: &EllipseGeom, p: Vec2) -> Vec<Vec2> {
    tangent_points(to_unit(e, p), ORIGIN, 1.0)
        .into_iter()
        .map(|q| norm_angle(atan2(q.y, q.x)))
        .filter(|&t| on_ellipse(e, t, 1e-9))
        .map(|t| ellipse_point(e, t))
        .collect()
}

/// Parameters of the four axis ends that lie on the curve.
pub fn quadrant_params(e: &EllipseGeom) -> Vec<f64> {
    [0.0, PI / 2.0, PI, (3.0 * PI) / 2.0]
        .into_iter()
        .filter(|&t| on_ellipse(e, t, 1e-9))
        .collect()
}

pub fn inside_ellipse(e: &EllipseGeom, p: Vec2) -> bool {
    let q = to_unit(e, p);
    q.x * q.x + q.y * q.y < 1.0
}

/// Ellipse through an axis (two ends) and the other half-axis (AutoCAD's default).
pub fn ellipse_from_axis(p1: Vec2, p2: Vec2, other_half: f64) -> Option<EllipseGeom> {
    let c = Vec2::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
    ellipse_from_center(c, p2, other_half)
}

/// Ellipse from its centre, one axis end and the other half-axis length.
pub fn ellipse_from_center(c: Vec2, axis_end: Vec2, other_half: f64) -> Option<EllipseGeom> {
    let v = Vec2::new(axis_end.x - c.x, axis_end.y - c.y);
    let a = js_hypot(v.x, v.y);
    let b = other_half.abs();
    if a < 1e-9 || b < 1e-9 {
        return None;
    }
    if b <= a {
        return Some(EllipseGeom {
            c,
            major: v,
            ratio: b / a,
            t0: 0.0,
            t1: 0.0,
        });
    }
    // The given axis is the minor one: the major runs at +90°.
    Some(EllipseGeom {
        c,
        major: Vec2::new((-v.y / a) * b, (v.x / a) * b),
        ratio: a / b,
        t0: 0.0,
        t1: 0.0,
    })
}

pub(crate) static OPS: &[Op] = &[
    op!("minorAxis", |e: EllipseGeom| minor_axis(&e)),
    op!("majorLength", |e: EllipseGeom| major_length(&e)),
    op!("ellipseSweep", |e: EllipseGeom| ellipse_sweep(&e)),
    op!("isFullEllipse", |e: EllipseGeom| is_full_ellipse(&e)),
    op!("ellipsePoint", |e: EllipseGeom, t: f64| ellipse_point(
        &e, t
    )),
    op!("ellipseDerivative", |e: EllipseGeom, t: f64| {
        ellipse_derivative(&e, t)
    }),
    op!("paramOfPoint", |e: EllipseGeom, p: Vec2| param_of_point(
        &e, p
    )),
    op!("paramAtPolar", |e: EllipseGeom, angle: f64| param_at_polar(
        &e, angle
    )),
    op!("onEllipse", |e: EllipseGeom, t: f64, eps: Option<f64>| {
        on_ellipse(&e, t, eps.unwrap_or(1e-9))
    }),
    op!(
        "tessellateEllipse",
        |e: EllipseGeom, per_turn: Option<f64>| tessellate_ellipse(&e, per_turn.unwrap_or(128.0))
    ),
    op!("ellipseLength", |e: EllipseGeom| ellipse_length(&e)),
    op!("ellipseArea", |e: EllipseGeom| ellipse_area(&e)),
    op!("closestParam", |e: EllipseGeom, p: Vec2| closest_param(
        &e, p
    )),
    op!("lineEllipse", |e: EllipseGeom, a: Vec2, b: Vec2| {
        line_ellipse(&e, a, b)
    }),
    op!("ellipseTangentPoints", |e: EllipseGeom, p: Vec2| {
        ellipse_tangent_points(&e, p)
    }),
    op!("quadrantParams", |e: EllipseGeom| quadrant_params(&e)),
    op!("insideEllipse", |e: EllipseGeom, p: Vec2| inside_ellipse(
        &e, p
    )),
    op!("ellipseFromAxis", |p1: Vec2, p2: Vec2, other_half: f64| {
        ellipse_from_axis(p1, p2, other_half)
    }),
    op!(
        "ellipseFromCenter",
        |c: Vec2, axis_end: Vec2, other_half: f64| ellipse_from_center(c, axis_end, other_half)
    ),
];
