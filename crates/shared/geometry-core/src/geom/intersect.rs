//! Primitive edges and their intersections (`apps/web/src/model/geom/intersect.ts`).
//! Every entity decomposes into these; an arc edge runs from `a0` through
//! the signed `sweep` (negative = clockwise), so path parameters stay
//! monotonic along polyline arcs.

use crate::api::Op;
use crate::geom::arc::{ON_ARC_EPS, norm_angle, on_arc};
use crate::jsmath::{TAU, acos, atan2, cos, js_hypot, js_max, js_min, sin};
use crate::op;
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

fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ax * by - ay * bx
}

/// Infinite-line intersection; t, u are parameters along a→b and c→d.
pub fn line_line(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<Hit> {
    let rx = b.x - a.x;
    let ry = b.y - a.y;
    let sx = d.x - c.x;
    let sy = d.y - c.y;
    let den = cross(rx, ry, sx, sy);
    if den.abs() < EPS * js_max(1.0, js_hypot(rx, ry) * js_hypot(sx, sy)) {
        return None; // parallel
    }
    let qx = c.x - a.x;
    let qy = c.y - a.y;
    let t = cross(qx, qy, sx, sy) / den;
    let u = cross(qx, qy, rx, ry) / den;
    Some(Hit {
        p: Vec2::new(a.x + t * rx, a.y + t * ry),
        t,
        u,
    })
}

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
