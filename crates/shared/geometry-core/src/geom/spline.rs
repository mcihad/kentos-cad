//! Centripetal Catmull-Rom curve through fit points (`apps/web/src/model/geom/spline.ts`,
//! Barry–Goldman evaluation, α = 0.5).

use crate::api::Op;
use crate::geom::quadrature::integrate;
use crate::jsmath::{js_hypot, js_max, js_min, or};
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

/// Halvings of a step at most: a span's step over 2¹⁸.
const DEPTH: u32 = 18;

/// The distance from `p` to the segment from `a` to `c`.
fn segment_distance(p: Vec2, a: Vec2, c: Vec2) -> f64 {
    let (dx, dy) = (c.x - a.x, c.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        js_min(
            1.0,
            js_max(0.0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2),
        )
    };
    js_hypot(p.x - (a.x + dx * t), p.y - (a.y + dy * t))
}

/// [`catmull_rom`]'s curve with its `per_span` steps a span, and in and near `b` each step halved until its middle
/// lies within `tol` of its chord (docs/adr/9233): never coarser than the outline, round to the deep view's tolerance
/// in it (plan/render.md RM-09). A step is near `b` when the box of its ends and middle, grown by how far the middle
/// strays, meets it.
pub fn catmull_rom_in_box(
    pts: &[Vec2],
    closed: bool,
    per_span: f64,
    b: &crate::geometry::Bounds,
    tol: f64,
) -> Vec<Vec2> {
    let n = pts.len();
    if n < 3 || !(tol > 0.0) || !(per_span >= 1.0) {
        return catmull_rom(pts, closed, per_span);
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
    let near = |a: Vec2, c: Vec2, m: Vec2, far: f64| {
        let lo = Vec2::new(
            js_min(js_min(a.x, c.x), m.x) - far,
            js_min(js_min(a.y, c.y), m.y) - far,
        );
        let hi = Vec2::new(
            js_max(js_max(a.x, c.x), m.x) + far,
            js_max(js_max(a.y, c.y), m.y) + far,
        );
        !(hi.x < b.min_x || lo.x > b.max_x || hi.y < b.min_y || lo.y > b.max_y)
    };
    #[allow(clippy::too_many_arguments)]
    fn halve(
        eval: &dyn Fn(f64) -> Vec2,
        near: &dyn Fn(Vec2, Vec2, Vec2, f64) -> bool,
        tol: f64,
        (s0, a): (f64, Vec2),
        (s1, c): (f64, Vec2),
        depth: u32,
        out: &mut Vec<Vec2>,
    ) {
        let sm = 0.5 * (s0 + s1);
        let m = eval(sm);
        let far = segment_distance(m, a, c);
        if depth < DEPTH && far > tol && near(a, c, m, far) {
            halve(eval, near, tol, (s0, a), (sm, m), depth + 1, out);
            out.push(m);
            halve(eval, near, tol, (sm, m), (s1, c), depth + 1, out);
        }
    }
    let mut out = Vec::new();
    let spans = if closed { ni } else { ni - 1 };
    for i in 0..spans {
        let p = [at(i - 1), at(i), at(i + 1), at(i + 2)];
        let t1 = knot(p[0], p[1]);
        let t2 = t1 + knot(p[1], p[2]);
        let t3 = t2 + knot(p[2], p[3]);
        let k = [0.0, t1, t2, t3];
        let eval = |s: f64| eval_span(p, k, t1 + ((t2 - t1) * s) / per_span);
        let mut s = 0.0;
        while s < per_span {
            let a = eval(s);
            out.push(a);
            let end = if s + 1.0 < per_span {
                eval(s + 1.0)
            } else {
                p[2]
            };
            halve(&eval, &near, tol, (s, a), (s + 1.0, end), 0, &mut out);
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

/// One span of the curve as a cubic Bézier (`ctrl`: the span's start, two
/// inner control points, its end) and the span's share of the centripetal
/// parameter (`dt`, the knot interval a B-spline of these spans uses).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BezierSpan {
    pub ctrl: [Vec2; 4],
    pub dt: f64,
}

/// The curve `catmull_rom` draws, span by span as cubic Béziers, so a file
/// that holds curves as B-splines (DXF SPLINE) carries the same curve, not
/// a fit. Each span of the Barry–Goldman evaluation is a cubic polynomial
/// in the knot parameter; its inner control points sit a third of the
/// span's knot interval along the derivative at either end, which for
/// knots t0 < t1 < t2 < t3 is
///   C'(t1) = (P1 − P0)/(t1 − t0) − (P2 − P0)/(t2 − t0) + (P2 − P1)/(t2 − t1)
/// (and likewise at t2), written with differences of points so TM-sized
/// coordinates lose nothing. Two points are drawn straight, as
/// `catmull_rom` draws them, and come out as one straight span; fewer give
/// nothing.
pub fn catmull_rom_beziers(pts: &[Vec2], closed: bool) -> Vec<BezierSpan> {
    let n = pts.len();
    if n < 2 {
        return Vec::new();
    }
    let third = |a: Vec2, b: Vec2, k: f64| {
        Vec2::new(a.x + (b.x - a.x) * k / 3.0, a.y + (b.y - a.y) * k / 3.0)
    };
    if n == 2 {
        let (a, b) = (pts[0], pts[1]);
        return vec![BezierSpan {
            ctrl: [a, third(a, b, 1.0), third(a, b, 2.0), b],
            dt: knot(a, b),
        }];
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
    let diff = |a: Vec2, b: Vec2| Vec2::new(b.x - a.x, b.y - a.y);
    // The derivative at the middle point q of p → q → r, whose knot intervals are d0 and d1.
    let tangent = |u0: Vec2, u1: Vec2, d0: f64, d1: f64| {
        let (k0, k1) = (d1 / (d0 * (d0 + d1)), d0 / (d1 * (d0 + d1)));
        Vec2::new(u0.x * k0 + u1.x * k1, u0.y * k0 + u1.y * k1)
    };
    let spans = if closed { ni } else { ni - 1 };
    let mut out = Vec::with_capacity(spans as usize);
    for i in 0..spans {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let (d01, d12, d23) = (knot(p0, p1), knot(p1, p2), knot(p2, p3));
        let (u01, u12, u23) = (diff(p0, p1), diff(p1, p2), diff(p2, p3));
        let m1 = tangent(u01, u12, d01, d12);
        let m2 = tangent(u12, u23, d12, d23);
        let h = d12 / 3.0;
        out.push(BezierSpan {
            ctrl: [
                p1,
                Vec2::new(p1.x + m1.x * h, p1.y + m1.y * h),
                Vec2::new(p2.x - m2.x * h, p2.y - m2.y * h),
                p2,
            ],
            dt: d12,
        });
    }
    out
}

/// The curve's length (docs/adr/0149 §3): each span's cubic Bézier, the
/// length of the curve itself, ∫|B′(s)|ds by adaptive Gauss–Legendre to
/// 10⁻¹⁴ of it; not the length of the outline it is drawn with. The
/// derivative is taken from differences of the control points, so
/// TM-sized coordinates lose nothing.
pub fn spline_length(pts: &[Vec2], closed: bool) -> f64 {
    catmull_rom_beziers(pts, closed)
        .iter()
        .map(|span| bezier_length(&span.ctrl))
        .sum()
}

/// A cubic Bézier's length.
fn bezier_length(b: &[Vec2; 4]) -> f64 {
    let d = [
        Vec2::new(b[1].x - b[0].x, b[1].y - b[0].y),
        Vec2::new(b[2].x - b[1].x, b[2].y - b[1].y),
        Vec2::new(b[3].x - b[2].x, b[3].y - b[2].y),
    ];
    // B′(s) = 3[(1 − s)²·d0 + 2s(1 − s)·d1 + s²·d2].
    let speed = |s: f64| {
        let r = 1.0 - s;
        let (k0, k1, k2) = (3.0 * r * r, 6.0 * r * s, 3.0 * s * s);
        js_hypot(
            d[0].x * k0 + d[1].x * k1 + d[2].x * k2,
            d[0].y * k0 + d[1].y * k1 + d[2].y * k2,
        )
    };
    integrate(&speed, 0.0, 1.0, 1e-14)
}

pub(crate) static OPS: &[Op] = &[op!(
    "catmullRom",
    |pts: Vec<Vec2>, closed: bool, per_span: Option<f64>| catmull_rom(
        &pts,
        closed,
        per_span.unwrap_or(16.0)
    )
)];

#[cfg(test)]
mod tests {
    use super::*;

    /// The deep view's spline (docs/adr/9233): over everything with a loose tolerance, the outline's points; in a
    /// small box with a tight one, more points there, each chord's middle within the tolerance of the curve (a
    /// 4 096-step outline stands in for it), and every outline point kept.
    #[test]
    fn a_spline_in_a_box_is_round_to_the_box_s_tolerance() {
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(3.0, 2.0),
            Vec2::new(6.0, -1.0),
            Vec2::new(9.0, 1.0),
        ];
        let all = crate::geometry::Bounds {
            min_x: -1e9,
            min_y: -1e9,
            max_x: 1e9,
            max_y: 1e9,
        };
        assert_eq!(
            catmull_rom_in_box(&pts, false, 16.0, &all, 1.0),
            catmull_rom(&pts, false, 16.0)
        );
        let b = crate::geometry::Bounds {
            min_x: 4.0,
            min_y: -1.0,
            max_x: 4.2,
            max_y: 1.5,
        };
        let round = catmull_rom_in_box(&pts, false, 16.0, &b, 1e-7);
        let outline = catmull_rom(&pts, false, 16.0);
        assert!(round.len() > outline.len());
        assert!(
            outline.iter().all(|p| round.contains(p)),
            "every outline point kept"
        );
        let fine = catmull_rom(&pts, false, 4096.0);
        let off = |p: Vec2| {
            fine.windows(2)
                .map(|s| segment_distance(p, s[0], s[1]))
                .fold(f64::INFINITY, js_min)
        };
        for s in round.windows(2) {
            let m = Vec2::new(0.5 * (s[0].x + s[1].x), 0.5 * (s[0].y + s[1].y));
            if m.x >= b.min_x && m.x <= b.max_x && m.y >= b.min_y && m.y <= b.max_y {
                assert!(off(m) < 1e-6, "a chord's middle {} off the curve", off(m));
            }
        }
    }

    fn bezier(b: &[Vec2; 4], s: f64) -> Vec2 {
        let r = 1.0 - s;
        let (c0, c1, c2, c3) = (r * r * r, 3.0 * r * r * s, 3.0 * r * s * s, s * s * s);
        Vec2::new(
            b[0].x * c0 + b[1].x * c1 + b[2].x * c2 + b[3].x * c3,
            b[0].y * c0 + b[1].y * c1 + b[2].y * c2 + b[3].y * c3,
        )
    }

    /// The curve as `catmull_rom` evaluates it, at parameter s ∈ [0, 1] of span i.
    fn drawn(pts: &[Vec2], closed: bool, i: usize, s: f64) -> Vec2 {
        let n = pts.len() as i64;
        let at = |k: i64| -> Vec2 {
            if closed {
                pts[(((k % n) + n) % n) as usize]
            } else if k < 0 {
                reflect(pts[1], pts[0])
            } else if k >= n {
                reflect(pts[pts.len() - 2], pts[pts.len() - 1])
            } else {
                pts[k as usize]
            }
        };
        let i = i as i64;
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let t1 = knot(p0, p1);
        let t2 = t1 + knot(p1, p2);
        let t3 = t2 + knot(p2, p3);
        eval_span([p0, p1, p2, p3], [0.0, t1, t2, t3], t1 + (t2 - t1) * s)
    }

    fn dist(a: Vec2, b: Vec2) -> f64 {
        js_hypot(a.x - b.x, a.y - b.y)
    }

    fn same_curve(pts: &[Vec2], tol: f64) {
        for closed in [false, true] {
            let spans = catmull_rom_beziers(pts, closed);
            assert_eq!(spans.len(), if closed { pts.len() } else { pts.len() - 1 });
            for (i, b) in spans.iter().enumerate() {
                // Span ends are the points themselves, bit for bit.
                assert_eq!(b.ctrl[0], pts[i]);
                assert_eq!(b.ctrl[3], pts[(i + 1) % pts.len()]);
                assert!(b.dt >= 1e-6, "{b:?}");
                for k in 0..=16 {
                    let s = k as f64 / 16.0;
                    let (want, got) = (drawn(pts, closed, i, s), bezier(&b.ctrl, s));
                    assert!(
                        dist(want, got) < tol,
                        "closed {closed} span {i} s {s}: {want:?} {got:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn bezier_spans_are_the_drawn_curve() {
        // Uneven spacing and a sharp turn at TM-sized coordinates: the control
        // points differ from the exact ones by their rounding at 4.4·10⁶ m (ulp ≈ 10⁻⁹ m).
        let base = Vec2::new(452_000.0, 4_412_000.0);
        let local = [
            (0.0, 0.0),
            (10.0, 2.0),
            (12.0, 15.0),
            (40.0, 16.0),
            (41.0, 0.0),
        ];
        let tm: Vec<Vec2> = local
            .iter()
            .map(|&(x, y)| Vec2::new(base.x + x, base.y + y))
            .collect();
        same_curve(&tm, 1e-8);
        // A repeated point (a knot interval of 10⁻⁶): the evaluation extrapolates over that tiny
        // interval and cancels large terms, so it is compared near the origin, where it is exact enough.
        let repeated: Vec<Vec2> = [
            (0.0, 0.0),
            (10.0, 2.0),
            (12.0, 15.0),
            (12.0, 15.0),
            (40.0, 16.0),
            (41.0, 0.0),
        ]
        .iter()
        .map(|&(x, y)| Vec2::new(x, y))
        .collect();
        same_curve(&repeated, 1e-8);
    }

    #[test]
    fn neighbouring_spans_meet_with_the_same_tangent() {
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(3.0, 1.0),
            Vec2::new(4.0, 5.0),
            Vec2::new(9.0, 4.0),
        ];
        let spans = catmull_rom_beziers(&pts, false);
        for w in spans.windows(2) {
            // dC/dt at the joint from either side: 3·(end − inner) / dt.
            let (a, b) = (&w[0], &w[1]);
            let left = Vec2::new(
                3.0 * (a.ctrl[3].x - a.ctrl[2].x) / a.dt,
                3.0 * (a.ctrl[3].y - a.ctrl[2].y) / a.dt,
            );
            let right = Vec2::new(
                3.0 * (b.ctrl[1].x - b.ctrl[0].x) / b.dt,
                3.0 * (b.ctrl[1].y - b.ctrl[0].y) / b.dt,
            );
            assert!(dist(left, right) < 1e-12, "{left:?} {right:?}");
        }
    }

    #[test]
    fn two_points_are_one_straight_span_and_fewer_give_nothing() {
        let (a, b) = (Vec2::new(1.0, 2.0), Vec2::new(7.0, 5.0));
        for closed in [false, true] {
            let spans = catmull_rom_beziers(&[a, b], closed);
            assert_eq!(spans.len(), 1);
            assert_eq!((spans[0].ctrl[0], spans[0].ctrl[3]), (a, b));
            assert!(dist(bezier(&spans[0].ctrl, 0.5), Vec2::new(4.0, 3.5)) < 1e-12);
        }
        assert!(catmull_rom_beziers(&[a], false).is_empty());
        assert!(catmull_rom_beziers(&[], true).is_empty());
    }
}
