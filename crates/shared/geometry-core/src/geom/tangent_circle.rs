//! Circles tangent to two edges with a given radius ("Teğet, teğet,
//! yarıçap") and to three edges ("Teğet, teğet, teğet"; Apollonius)
//! (`apps/web/src/model/geom/tangentCircle.ts`). Lines count as infinite, arcs as
//! their full circle; the solution whose tangent points are nearest the
//! picked points wins.

use crate::api::Op;
use crate::geom::arc::Circle;
use crate::geom::intersect::{Edge, circle_circle, line_circle_params, line_line};
use crate::jsmath::{js_hypot, js_hypot_n, js_max, js_max_all, or};
use crate::op;
use crate::vec2::Vec2;

enum Support {
    Line { a: Vec2, b: Vec2 },
    Circle { c: Vec2, r: f64 },
}

pub fn tangent_tangent_radius(
    e1: &Edge,
    pick1: Vec2,
    e2: &Edge,
    pick2: Vec2,
    r: f64,
) -> Option<Circle> {
    if !(r > 0.0) {
        return None;
    }
    let mut candidates = Vec::new();
    for o1 in parallels(e1, r) {
        for o2 in parallels(e2, r) {
            candidates.extend(crossings(&o1, &o2));
        }
    }
    let mut best: Option<(Vec2, f64)> = None;
    for c in candidates {
        let (Some(t1), Some(t2)) = (touch_point(e1, c), touch_point(e2, c)) else {
            continue;
        };
        let score =
            js_hypot(t1.x - pick1.x, t1.y - pick1.y) + js_hypot(t2.x - pick2.x, t2.y - pick2.y);
        if best.is_none_or(|(_, s)| score < s) {
            best = Some((c, score));
        }
    }
    best.map(|(c, _)| Circle { c, r })
}

fn parallels(e: &Edge, r: f64) -> Vec<Support> {
    match *e {
        Edge::Seg { a, b } => {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let l = js_hypot(dx, dy);
            if l < 1e-12 {
                return Vec::new();
            }
            let nx = (-dy / l) * r;
            let ny = (dx / l) * r;
            [1.0, -1.0]
                .into_iter()
                .map(|s| Support::Line {
                    a: Vec2::new(a.x + nx * s, a.y + ny * s),
                    b: Vec2::new(b.x + nx * s, b.y + ny * s),
                })
                .collect()
        }
        Edge::Arc { c, r: er, .. } => {
            let mut out = vec![Support::Circle { c, r: er + r }];
            if (er - r).abs() > 1e-9 {
                out.push(Support::Circle {
                    c,
                    r: (er - r).abs(),
                });
            }
            out
        }
    }
}

fn crossings(a: &Support, b: &Support) -> Vec<Vec2> {
    match (a, b) {
        (Support::Line { a: a1, b: b1 }, Support::Line { a: a2, b: b2 }) => {
            line_line(*a1, *b1, *a2, *b2)
                .map(|h| h.p)
                .into_iter()
                .collect()
        }
        (Support::Circle { c: c1, r: r1 }, Support::Circle { c: c2, r: r2 }) => {
            circle_circle(*c1, *r1, *c2, *r2)
        }
        (Support::Line { a: la, b: lb }, Support::Circle { c, r })
        | (Support::Circle { c, r }, Support::Line { a: la, b: lb }) => {
            line_circle_params(*la, *lb, *c, *r)
                .into_iter()
                .map(|t| Vec2::new(la.x + (lb.x - la.x) * t, la.y + (lb.y - la.y) * t))
                .collect()
        }
    }
}

/// Where a circle centred at c touches the edge's supporting line or circle.
fn touch_point(e: &Edge, c: Vec2) -> Option<Vec2> {
    match *e {
        Edge::Seg { a, b } => {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let l2 = dx * dx + dy * dy;
            let t = ((c.x - a.x) * dx + (c.y - a.y) * dy) / l2;
            Some(Vec2::new(a.x + dx * t, a.y + dy * t))
        }
        Edge::Arc { c: ec, r, .. } => {
            let d = js_hypot(c.x - ec.x, c.y - ec.y);
            if d < 1e-12 {
                return None;
            }
            Some(Vec2::new(
                ec.x + ((c.x - ec.x) / d) * r,
                ec.y + ((c.y - ec.y) / d) * r,
            ))
        }
    }
}

/// One tangency: a line keeps n·(c − a) = s·r; a circle keeps |c − C| = R + r
/// (s = 1), R − r (s = −1) or r − R (s = 0, the given circle inside).
#[derive(Clone, Copy)]
enum Constraint {
    Line { a: Vec2, n: Vec2, s: f64 },
    Circle { c: Vec2, big_r: f64, s: f64 },
}

fn shift(e: &Edge, o: Vec2) -> Edge {
    match *e {
        Edge::Seg { a, b } => Edge::Seg {
            a: Vec2::new(a.x - o.x, a.y - o.y),
            b: Vec2::new(b.x - o.x, b.y - o.y),
        },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c: Vec2::new(c.x - o.x, c.y - o.y),
            r,
            a0,
            sweep,
        },
    }
}

/// Residual and gradient (∂/∂cx, ∂/∂cy, ∂/∂r) of a constraint.
fn residual(k: &Constraint, c: Vec2, r: f64) -> [f64; 4] {
    match *k {
        Constraint::Line { a, n, s } => {
            [n.x * (c.x - a.x) + n.y * (c.y - a.y) - s * r, n.x, n.y, -s]
        }
        Constraint::Circle { c: kc, big_r, s } => {
            let dx = c.x - kc.x;
            let dy = c.y - kc.y;
            let d = or(js_hypot(dx, dy), 1e-12);
            let target = if s == 0.0 { r - big_r } else { big_r + s * r };
            [d - target, dx / d, dy / d, if s == 0.0 { -1.0 } else { -s }]
        }
    }
}

fn det3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// Newton's method (exact in one step when all three are lines).
fn solve3(ks: &[Constraint; 3], c0: Vec2, r0: f64, scale: f64) -> Option<Circle> {
    let mut c = c0;
    let mut r = r0;
    for _ in 0..60 {
        let rows = ks.map(|k| residual(&k, c, r));
        let j = rows.map(|row| [row[1], row[2], row[3]]);
        let f = rows.map(|row| row[0]);
        let d = det3(&j);
        if d.abs() < 1e-14 {
            return None;
        }
        let col = |i: usize| {
            let mut m = j;
            for (row, fj) in m.iter_mut().zip(f) {
                row[i] = -fj;
            }
            m
        };
        let step = [det3(&col(0)) / d, det3(&col(1)) / d, det3(&col(2)) / d];
        c = Vec2::new(c.x + step[0], c.y + step[1]);
        r += step[2];
        if js_hypot_n(&step) <= 1e-13 * scale {
            break;
        }
    }
    let worst = js_max_all(ks.iter().map(|k| residual(k, c, r)[0].abs()));
    (worst <= 1e-9 * js_max(scale, r)).then_some(Circle { c, r })
}

pub fn tangent_tangent_tangent(edges: &[Edge; 3], picks: &[Vec2; 3]) -> Option<Circle> {
    // Work near the picks: the equations lose digits at TM magnitudes.
    let o = Vec2::new(
        (picks[0].x + picks[1].x + picks[2].x) / 3.0,
        (picks[0].y + picks[1].y + picks[2].y) / 3.0,
    );
    let local = edges.map(|e| shift(&e, o));
    let lp = picks.map(|p| Vec2::new(p.x - o.x, p.y - o.y));
    let scale = js_max_all(std::iter::once(1.0).chain(lp.iter().map(|p| js_hypot(p.x, p.y))));
    let options: Vec<Vec<Constraint>> = local
        .iter()
        .map(|e| match *e {
            Edge::Seg { a, b } => {
                let l = js_hypot(b.x - a.x, b.y - a.y);
                if l < 1e-12 {
                    return Vec::new();
                }
                let n = Vec2::new(-(b.y - a.y) / l, (b.x - a.x) / l);
                [1.0, -1.0]
                    .into_iter()
                    .map(|s| Constraint::Line { a, n, s })
                    .collect()
            }
            Edge::Arc { c, r, .. } => [1.0, -1.0, 0.0]
                .into_iter()
                .map(|s| Constraint::Circle { c, big_r: r, s })
                .collect(),
        })
        .collect();
    let r0 = or(
        lp.iter().fold(0.0, |m, p| m + js_hypot(p.x, p.y)) / 3.0,
        1.0,
    );
    let mut best: Option<(Circle, f64)> = None;
    for &c1 in &options[0] {
        for &c2 in &options[1] {
            for &c3 in &options[2] {
                let Some(sol) = solve3(&[c1, c2, c3], Vec2::new(0.0, 0.0), r0, scale) else {
                    continue;
                };
                if !(sol.r > 1e-9) {
                    continue;
                }
                let mut score = 0.0;
                for i in 0..3 {
                    score += match touch_point(&local[i], sol.c) {
                        Some(t) => js_hypot(t.x - lp[i].x, t.y - lp[i].y),
                        None => f64::INFINITY,
                    };
                }
                if best.is_none_or(|(_, s)| score < s) {
                    best = Some((sol, score));
                }
            }
        }
    }
    best.map(|(s, _)| Circle {
        c: Vec2::new(s.c.x + o.x, s.c.y + o.y),
        r: s.r,
    })
}

pub(crate) static OPS: &[Op] = &[
    op!(
        "tangentTangentRadius",
        |e1: Edge, pick1: Vec2, e2: Edge, pick2: Vec2, r: f64| tangent_tangent_radius(
            &e1, pick1, &e2, pick2, r
        )
    ),
    op!(
        "tangentTangentTangent",
        |edges: [Edge; 3], picks: [Vec2; 3]| tangent_tangent_tangent(&edges, &picks)
    ),
];
