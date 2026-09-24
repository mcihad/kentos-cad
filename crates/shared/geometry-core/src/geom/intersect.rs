//! Primitive edges and their intersections (`apps/web/src/model/geom/intersect.ts`).
//! Every entity decomposes into these; an arc edge runs from `a0` through
//! the signed `sweep` (negative = clockwise), so path parameters stay
//! monotonic along polyline arcs.

use crate::api::Op;
use crate::geom::arc::{ON_ARC_EPS, norm_angle, on_arc};
use crate::jsmath::{TAU, acos, atan2, cos, js_hypot, js_max, js_min, sin};
use crate::op;
use crate::predicates::cross_accurate;
use crate::vec2::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edge {
    Seg {
        a: Vec2,
        b: Vec2,
    },
    Arc {
        c: Vec2,
        r: f64,
        a0: f64,
        sweep: f64,
    },
}

crate::json_tagged!(Edge, "kind", Seg => "seg" { a, b }, Arc => "arc" { c, r, a0, sweep });

/// Whether angle θ lies on an arc edge (either direction).
pub fn on_edge_arc(a0: f64, sweep: f64, theta: f64) -> bool {
    if sweep >= 0.0 {
        on_arc(theta, a0, sweep, ON_ARC_EPS)
    } else {
        on_arc(theta, a0 + sweep, -sweep, ON_ARC_EPS)
    }
}

/// A hit on edge 1 at parameter t (0..1 along it) and on edge 2 at u.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub p: Vec2,
    pub t: f64,
    pub u: f64,
}

crate::json_struct!(Hit { p, t, u });

const EPS: f64 = 1e-12;

/// Infinite-line intersection; t, u are parameters along a→b and c→d.
/// The three cross products are taken accurately (`cross_accurate`: below
/// 2⁻⁴¹ relative error however nearly parallel the lines are; the plain
/// products, bit for bit, where they are good), so t and u are too: a
/// crossing inside both segments always gives t and u in [0, 1] up to
/// that error, never outside `seg_seg`'s band (CLAUDE.md §23.3). Lines
/// within 1e-12 of parallel are parallel (a CAD tolerance, kept).
pub fn line_line(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<Hit> {
    let rx = b.x - a.x;
    let ry = b.y - a.y;
    let sx = d.x - c.x;
    let sy = d.y - c.y;
    let den = cross_accurate(a, b, c, d);
    if den.abs() < EPS * js_max(1.0, js_hypot(rx, ry) * js_hypot(sx, sy)) {
        return None; // parallel
    }
    let t = cross_accurate(a, c, c, d) / den;
    let u = cross_accurate(a, c, a, b) / den;
    Some(Hit {
        p: Vec2::new(a.x + t * rx, a.y + t * ry),
        t,
        u,
    })
}

/// Two segments' crossing, touching counted: t and u within `eps` of
/// [0, 1] (a CAD tolerance). With `line_line`'s accurate parameters the
/// decision is exact but for crossings within 2⁻⁴¹·|t| of the band's edge.
pub fn seg_seg(a: Vec2, b: Vec2, c: Vec2, d: Vec2, eps: f64) -> Option<Hit> {
    line_line(a, b, c, d)
        .filter(|h| h.t >= -eps && h.t <= 1.0 + eps && h.u >= -eps && h.u <= 1.0 + eps)
}

/// Parameters t along a→b (unbounded) where the line meets the circle,
/// solved from the foot of the perpendicular from the centre (construction
/// lines are 2·10⁷ m long; the discriminant would cancel its digits away).
pub fn line_circle_params(a: Vec2, b: Vec2, c: Vec2, r: f64) -> Vec<f64> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let aa = dx * dx + dy * dy;
    if aa < EPS {
        return Vec::new();
    }
    let s = ((c.x - a.x) * dx + (c.y - a.y) * dy) / aa;
    let fx = a.x + dx * s - c.x;
    let fy = a.y + dy * s - c.y;
    let h2 = r * r - (fx * fx + fy * fy);
    let tol = 1e-12 * js_max(1.0, r * r);
    if h2 < -tol {
        return Vec::new();
    }
    if h2 <= tol {
        return vec![s];
    }
    let h = (h2 / aa).sqrt();
    vec![s - h, s + h]
}

pub fn circle_circle(c1: Vec2, r1: f64, c2: Vec2, r2: f64) -> Vec<Vec2> {
    let dx = c2.x - c1.x;
    let dy = c2.y - c1.y;
    let d = js_hypot(dx, dy);
    if d < EPS || d > r1 + r2 + 1e-9 || d < (r1 - r2).abs() - 1e-9 {
        return Vec::new();
    }
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h2 = r1 * r1 - a * a;
    let h = if h2 > 0.0 { h2.sqrt() } else { 0.0 };
    let mx = c1.x + (a * dx) / d;
    let my = c1.y + (a * dy) / d;
    if h < 1e-12 {
        return vec![Vec2::new(mx, my)];
    }
    vec![
        Vec2::new(mx + (h * dy) / d, my - (h * dx) / d),
        Vec2::new(mx - (h * dy) / d, my + (h * dx) / d),
    ]
}

fn angle_of(c: Vec2, p: Vec2) -> f64 {
    atan2(p.y - c.y, p.x - c.x)
}

/// Parameter (0..1) of a point known to lie on the edge.
pub fn param_on(e: &Edge, p: Vec2) -> f64 {
    match *e {
        Edge::Seg { a, b } => {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let l2 = dx * dx + dy * dy;
            if l2 < EPS {
                0.0
            } else {
                ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2
            }
        }
        Edge::Arc { c, a0, sweep, .. } => {
            if sweep >= 0.0 {
                norm_angle(angle_of(c, p) - a0) / sweep
            } else {
                norm_angle(a0 - angle_of(c, p)) / -sweep
            }
        }
    }
}

pub fn point_at(e: &Edge, t: f64) -> Vec2 {
    match *e {
        Edge::Seg { a, b } => Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t),
        Edge::Arc { c, r, a0, sweep } => {
            let a = a0 + sweep * t;
            Vec2::new(c.x + cos(a) * r, c.y + sin(a) * r)
        }
    }
}

/// All intersections between two bounded edges.
pub fn intersect_edges(e1: &Edge, e2: &Edge) -> Vec<Hit> {
    match (*e1, *e2) {
        (Edge::Seg { a, b }, Edge::Seg { a: c, b: d }) => {
            seg_seg(a, b, c, d, 1e-9).into_iter().collect()
        }
        (Edge::Seg { .. }, Edge::Arc { .. }) => seg_arc(e1, e2),
        (Edge::Arc { .. }, Edge::Seg { .. }) => seg_arc(e2, e1)
            .into_iter()
            .map(|h| Hit {
                p: h.p,
                t: h.u,
                u: h.t,
            })
            .collect(),
        (
            Edge::Arc {
                c: c1,
                r: r1,
                a0: a01,
                sweep: s1,
            },
            Edge::Arc {
                c: c2,
                r: r2,
                a0: a02,
                sweep: s2,
            },
        ) => circle_circle(c1, r1, c2, r2)
            .into_iter()
            .filter(|&p| {
                on_edge_arc(a01, s1, angle_of(c1, p)) && on_edge_arc(a02, s2, angle_of(c2, p))
            })
            .map(|p| Hit {
                p,
                t: param_on(e1, p),
                u: param_on(e2, p),
            })
            .collect(),
    }
}

fn seg_arc(s: &Edge, arc: &Edge) -> Vec<Hit> {
    let (Edge::Seg { a, b }, Edge::Arc { c, r, a0, sweep }) = (*s, *arc) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for t in line_circle_params(a, b, c, r) {
        if t < -1e-9 || t > 1.0 + 1e-9 {
            continue;
        }
        let p = point_at(s, t);
        if !on_edge_arc(a0, sweep, angle_of(c, p)) {
            continue;
        }
        out.push(Hit {
            p,
            t,
            u: param_on(arc, p),
        });
    }
    out
}

/// Where a ray from `o` along `dir` meets an edge beyond `min_t` (in units of |dir|).
pub fn ray_edge(o: Vec2, dir: Vec2, e: &Edge, min_t: f64) -> Vec<f64> {
    let far = Vec2::new(o.x + dir.x, o.y + dir.y);
    match *e {
        Edge::Seg { a, b } => match line_line(o, far, a, b) {
            Some(h) if h.t > min_t && h.u >= -1e-9 && h.u <= 1.0 + 1e-9 => vec![h.t],
            _ => Vec::new(),
        },
        Edge::Arc { c, r, a0, sweep } => {
            let ray = Edge::Seg { a: o, b: far };
            line_circle_params(o, far, c, r)
                .into_iter()
                .filter(|&t| t > min_t && on_edge_arc(a0, sweep, angle_of(c, point_at(&ray, t))))
                .collect()
        }
    }
}

/// The closest point on an edge, its parameter and distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Closest {
    pub p: Vec2,
    pub t: f64,
    pub d: f64,
}

crate::json_struct!(Closest { p, t, d });

pub fn closest_on_edge(e: &Edge, p: Vec2) -> Closest {
    match *e {
        Edge::Seg { .. } => {
            let t = js_max(0.0, js_min(1.0, param_on(e, p)));
            let q = point_at(e, t);
            Closest {
                p: q,
                t,
                d: js_hypot(p.x - q.x, p.y - q.y),
            }
        }
        Edge::Arc { c, r, a0, sweep } => {
            let ang = angle_of(c, p);
            if on_edge_arc(a0, sweep, ang) {
                let q = Vec2::new(c.x + cos(ang) * r, c.y + sin(ang) * r);
                return Closest {
                    p: q,
                    t: param_on(e, q),
                    d: (js_hypot(p.x - c.x, p.y - c.y) - r).abs(),
                };
            }
            let s = point_at(e, 0.0);
            let f = point_at(e, 1.0);
            let ds = js_hypot(p.x - s.x, p.y - s.y);
            let df = js_hypot(p.x - f.x, p.y - f.y);
            if ds <= df {
                Closest {
                    p: s,
                    t: 0.0,
                    d: ds,
                }
            } else {
                Closest {
                    p: f,
                    t: 1.0,
                    d: df,
                }
            }
        }
    }
}

/// Foot of the perpendicular from p onto the edge's line or circle, when it lies on the edge.
pub fn perpendicular_foot(e: &Edge, p: Vec2) -> Option<Vec2> {
    match *e {
        Edge::Seg { .. } => {
            let t = param_on(e, p);
            (t >= -1e-9 && t <= 1.0 + 1e-9).then(|| point_at(e, t))
        }
        Edge::Arc { c, r, a0, sweep } => {
            let ang = angle_of(c, p);
            if js_hypot(p.x - c.x, p.y - c.y) < EPS {
                return None;
            }
            on_edge_arc(a0, sweep, ang).then(|| Vec2::new(c.x + cos(ang) * r, c.y + sin(ang) * r))
        }
    }
}

pub fn full_circle(c: Vec2, r: f64) -> Edge {
    Edge::Arc {
        c,
        r,
        a0: 0.0,
        sweep: TAU,
    }
}

/// Points where lines from `p` touch the circle (c, r); empty when p is inside.
pub fn tangent_points(p: Vec2, c: Vec2, r: f64) -> Vec<Vec2> {
    let dx = p.x - c.x;
    let dy = p.y - c.y;
    let d2 = dx * dx + dy * dy;
    if d2 <= r * r + 1e-12 {
        return Vec::new();
    }
    let d = d2.sqrt();
    let base = atan2(dy, dx);
    let half = acos(r / d);
    [base + half, base - half]
        .into_iter()
        .map(|a| Vec2::new(c.x + cos(a) * r, c.y + sin(a) * r))
        .collect()
}

fn arc_edge(e: Edge) -> Result<(f64, f64), String> {
    match e {
        Edge::Arc { a0, sweep, .. } => Ok((a0, sweep)),
        Edge::Seg { .. } => Err("onEdgeArc: yay kenarı bekleniyordu".into()),
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("onEdgeArc", |e: Edge, theta: f64| arc_edge(e)
        .map(|(a0, s)| on_edge_arc(a0, s, theta))
        .ok()),
    op!("lineLine", |a: Vec2, b: Vec2, c: Vec2, d: Vec2| {
        line_line(a, b, c, d)
    }),
    op!("segSeg", |a: Vec2,
                   b: Vec2,
                   c: Vec2,
                   d: Vec2,
                   eps: Option<f64>| seg_seg(
        a,
        b,
        c,
        d,
        eps.unwrap_or(1e-9)
    )),
    op!("lineCircleParams", |a: Vec2, b: Vec2, c: Vec2, r: f64| {
        line_circle_params(a, b, c, r)
    }),
    op!("circleCircle", |c1: Vec2, r1: f64, c2: Vec2, r2: f64| {
        circle_circle(c1, r1, c2, r2)
    }),
    op!("paramOn", |e: Edge, p: Vec2| param_on(&e, p)),
    op!("pointAt", |e: Edge, t: f64| point_at(&e, t)),
    op!("intersectEdges", |e1: Edge, e2: Edge| intersect_edges(
        &e1, &e2
    )),
    op!("rayEdge", |o: Vec2,
                    dir: Vec2,
                    e: Edge,
                    min_t: Option<f64>| {
        ray_edge(o, dir, &e, min_t.unwrap_or(1e-9))
    }),
    op!("closestOnEdge", |e: Edge, p: Vec2| closest_on_edge(&e, p)),
    op!("perpendicularFoot", |e: Edge, p: Vec2| {
        perpendicular_foot(&e, p)
    }),
    op!("fullCircle", |c: Vec2, r: f64| full_circle(c, r)),
    op!("tangentPoints", |p: Vec2, c: Vec2, r: f64| {
        tangent_points(p, c, r)
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// What `line_line` computed before: the plain rounded cross products.
    fn plain_line_line(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<Hit> {
        let (rx, ry, sx, sy) = (b.x - a.x, b.y - a.y, d.x - c.x, d.y - c.y);
        let den = rx * sy - ry * sx;
        if den.abs() < EPS * js_max(1.0, js_hypot(rx, ry) * js_hypot(sx, sy)) {
            return None;
        }
        let (qx, qy) = (c.x - a.x, c.y - a.y);
        let t = (qx * sy - qy * sx) / den;
        let u = (qx * ry - qy * rx) / den;
        Some(Hit {
            p: Vec2::new(a.x + t * rx, a.y + t * ry),
            t,
            u,
        })
    }

    fn band(h: &Hit) -> bool {
        let eps = 1e-9;
        h.t >= -eps && h.t <= 1.0 + eps && h.u >= -eps && h.u <= 1.0 + eps
    }

    /// Deterministic random numbers (xorshift64*), the same on every target.
    struct Rnd(u64);
    impl Rnd {
        fn next(&mut self) -> f64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    /// Nearly parallel TM segments crossing near an end, where the rounded
    /// products lose about ε/sin δ of t, and every fourth pair meeting at a
    /// shared end (a sliver between two edges ending at one vertex: t = u = 1
    /// exactly). Against exact integer arithmetic (every float64 there lies
    /// on the 2⁻³⁵ grid): a crossing inside both segments is always found,
    /// with t and u in [0, 1] up to 1e-12 and t within 1e-12 of the exact
    /// value; one clearly outside the 1e-9 band (by another 1e-9) never is.
    /// The plain products put t off by more than the band, and missed shared
    /// ends.
    #[test]
    fn nearly_parallel_segments_decide_like_exact_arithmetic() {
        let unit = (1u64 << 35) as f64;
        let k = |x: f64| {
            let s = x * unit;
            assert!(s == s.trunc(), "{x} is off the grid");
            s as i128
        };
        let cross = |p1: Vec2, p2: Vec2, q1: Vec2, q2: Vec2| {
            (k(p2.x) - k(p1.x)) * (k(q2.y) - k(q1.y)) - (k(p2.y) - k(p1.y)) * (k(q2.x) - k(q1.x))
        };
        let positive = |n: i128, d: i128| if d < 0 { (-n, -d) } else { (n, d) };
        // 0 ≤ n/d ≤ 1 exactly; n/d below −2e-9 or above 1 + 2e-9 exactly.
        let inside = |n: i128, d: i128| {
            let (n, d) = positive(n, d);
            n >= 0 && n <= d
        };
        let clearly_out = |n: i128, d: i128| {
            let (n, d) = positive(n, d);
            1_000_000_000 * n < -2 * d || 1_000_000_000 * (n - d) > 2 * d
        };
        let base = Vec2::new(500_000.37, 4_400_000.11);
        let mut g = Rnd(2026);
        let (mut found, mut refused, mut plain_missed, mut plain_far) = (0, 0, 0, 0);
        for n in 0..20_000 {
            let a = Vec2::new(
                base.x + (g.next() - 0.5) * 100.0,
                base.y + (g.next() - 0.5) * 100.0,
            );
            let th = g.next() * TAU;
            let l1 = 20.0 + g.next() * 180.0;
            let b = Vec2::new(a.x + l1 * cos(th), a.y + l1 * sin(th));
            // A turn of 1e-10 … 1e-6 and a crossing within 1e-7 of an end.
            let sign = if g.next() < 0.5 { -1.0 } else { 1.0 };
            let delta = sign * libm::pow(10.0, -6.0 - 4.0 * g.next());
            let near = if n % 2 == 0 { 0.0 } else { 1.0 };
            let t0 = near + (g.next() - 0.5) * 2.0 * libm::pow(10.0, -7.0 - 6.0 * g.next());
            let x = Vec2::new(a.x + t0 * (b.x - a.x), a.y + t0 * (b.y - a.y));
            let (l2, s0, phi) = (20.0 + g.next() * 180.0, 0.2 + 0.6 * g.next(), th + delta);
            let c = Vec2::new(x.x - s0 * l2 * cos(phi), x.y - s0 * l2 * sin(phi));
            let d = if n % 4 == 3 {
                b
            } else {
                Vec2::new(
                    x.x + (1.0 - s0) * l2 * cos(phi),
                    x.y + (1.0 - s0) * l2 * sin(phi),
                )
            };
            let den = cross(a, b, c, d);
            // Within 2e-12 of parallel the parallel tolerance decides: skipped.
            let sin = (den as f64 / unit / unit).abs() / (l1 * l2);
            if sin < 2e-12 {
                continue;
            }
            let (tn, un) = (cross(a, c, c, d), cross(a, c, a, b));
            let got = seg_seg(a, b, c, d, 1e-9);
            let plain = plain_line_line(a, b, c, d).filter(band);
            if inside(tn, den) && inside(un, den) {
                found += 1;
                let h = got.unwrap_or_else(|| panic!("missed {a:?} {b:?} {c:?} {d:?}"));
                assert!(h.t >= -1e-12 && h.t <= 1.0 + 1e-12 && h.u >= -1e-12 && h.u <= 1.0 + 1e-12);
                let t = tn as f64 / den as f64;
                assert!((h.t - t).abs() <= 1e-12, "t {} ≠ {t}", h.t);
                plain_missed += usize::from(plain.is_none());
                if let Some(p) = plain_line_line(a, b, c, d) {
                    plain_far += usize::from((p.t - t).abs() > 1e-9);
                }
            } else if clearly_out(tn, den) || clearly_out(un, den) {
                refused += 1;
                assert!(got.is_none(), "took {a:?} {b:?} {c:?} {d:?}");
            }
        }
        assert!(found > 3000 && refused > 3000, "{found} {refused}");
        assert!(
            plain_missed > 100 && plain_far > 1000,
            "the plain products no longer fail here: {plain_missed} {plain_far}"
        );
    }
}
