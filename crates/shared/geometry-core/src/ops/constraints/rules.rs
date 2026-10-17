//! Each constraint's equations (docs/adr/0334 §2): residuals that are zero when it holds, written on dual numbers. A
//! length residual is in metres; a direction residual is a sine (or an angle) times the scale `s` of the drawing being
//! solved, so every row weighs about as a length does.

use std::ops::{Neg, Sub};

use super::dual::{D, P};
use super::model::Built;
use super::{Axis, Rule};

/// The residuals of `rule` from its parts built in order (`parts`), its value (`value`, metres or radians) and, for
/// a fix, the parts' place to hold (`at`, in the same order).
pub(super) fn rows(rule: &Rule, parts: &[Built], value: f64, at: &[Built], s: f64) -> Vec<D> {
    let point = |i: usize| match &parts[i] {
        Built::Point(p) => *p,
        Built::Segment(a, _) => *a,
        Built::Circle(c, _) => *c,
    };
    let segment = |i: usize| match &parts[i] {
        Built::Segment(a, b) => (*a, *b),
        Built::Point(p) => (*p, *p),
        Built::Circle(c, _) => (*c, *c),
    };
    let circle = |i: usize| match &parts[i] {
        Built::Circle(c, r) => (*c, *r),
        Built::Point(p) => (*p, D::c(0.0)),
        Built::Segment(a, _) => (*a, D::c(0.0)),
    };
    // The signed distance of `p` from the line through `a` and `b`: positive on its left.
    let side = |a: P, b: P, p: P| b.sub(a).cross(p.sub(a)) / b.sub(a).len();
    match rule {
        Rule::Coincident { b, .. } => {
            let a = point(0);
            match b.element {
                super::Element::Point(_) => {
                    let q = point(1);
                    vec![a.x - q.x, a.y - q.y]
                }
                super::Element::Segment { .. } => {
                    let (s0, s1) = segment(1);
                    vec![side(s0, s1, a)]
                }
                super::Element::Circle => {
                    let (c, r) = circle(1);
                    vec![a.sub(c).len() - r]
                }
            }
        }
        Rule::Horizontal { b, .. } => match b {
            None => {
                let (a, b) = segment(0);
                vec![b.y - a.y]
            }
            Some(_) => vec![point(1).y - point(0).y],
        },
        Rule::Vertical { b, .. } => match b {
            None => {
                let (a, b) = segment(0);
                vec![b.x - a.x]
            }
            Some(_) => vec![point(1).x - point(0).x],
        },
        Rule::Parallel { .. } | Rule::Perpendicular { .. } => {
            let (a0, a1) = segment(0);
            let (b0, b1) = segment(1);
            let (da, db) = (a1.sub(a0), b1.sub(b0));
            let norm = da.len() * db.len();
            let r = if matches!(rule, Rule::Parallel { .. }) {
                da.cross(db)
            } else {
                da.dot(db)
            };
            vec![r / norm * s]
        }
        Rule::Tangent {
            inside, left, a, ..
        } => match a.element {
            super::Element::Segment { .. } => {
                let (s0, s1) = segment(0);
                let (c, r) = circle(1);
                let d = side(s0, s1, c);
                vec![if *left { d } else { -d } - r]
            }
            _ => {
                let (c0, r0) = circle(0);
                let (c1, r1) = circle(1);
                let d = c1.sub(c0).len();
                vec![if *inside {
                    d - (r0 - r1).abs()
                } else {
                    d - (r0 + r1)
                }]
            }
        },
        Rule::Concentric { .. } => {
            let (c0, _) = circle(0);
            let (c1, _) = circle(1);
            vec![c1.x - c0.x, c1.y - c0.y]
        }
        Rule::Collinear { .. } => {
            let (a0, a1) = segment(0);
            let (b0, b1) = segment(1);
            vec![side(a0, a1, b0), side(a0, a1, b1)]
        }
        Rule::Equal { a, .. } => match a.element {
            super::Element::Circle => vec![circle(1).1 - circle(0).1],
            _ => {
                let (a0, a1) = segment(0);
                let (b0, b1) = segment(1);
                vec![b1.sub(b0).len() - a1.sub(a0).len()]
            }
        },
        Rule::Symmetric { .. } => {
            let (p, q) = (point(0), point(1));
            let (s0, s1) = segment(2);
            let m = P {
                x: (p.x + q.x) * 0.5,
                y: (p.y + q.y) * 0.5,
            };
            let d = s1.sub(s0);
            vec![side(s0, s1, m), d.dot(q.sub(p)) / d.len()]
        }
        Rule::Fix { .. } => {
            let mut out = Vec::new();
            for (now, then) in parts.iter().zip(at) {
                match (now, then) {
                    (Built::Point(p), Built::Point(q)) => out.extend([p.x - q.x.v, p.y - q.y.v]),
                    (Built::Segment(a, b), Built::Segment(c, d)) => {
                        out.extend([a.x - c.x.v, a.y - c.y.v, b.x - d.x.v, b.y - d.y.v])
                    }
                    (Built::Circle(c, r), Built::Circle(e, q)) => {
                        out.extend([c.x - e.x.v, c.y - e.y.v, *r - q.v])
                    }
                    _ => {}
                }
            }
            out
        }
        Rule::Distance { axis, left, b, .. } => {
            let a = point(0);
            match b.element {
                super::Element::Segment { .. } => {
                    let (s0, s1) = segment(1);
                    let d = side(s0, s1, a);
                    vec![if *left { d } else { -d } - value]
                }
                _ => {
                    let q = point(1);
                    vec![match axis {
                        Axis::Aligned => q.sub(a).len() - value,
                        Axis::Horizontal => q.x - a.x - value,
                        Axis::Vertical => q.y - a.y - value,
                    }]
                }
            }
        }
        Rule::Angle {
            reverse_a,
            reverse_b,
            ..
        } => {
            let (a0, a1) = segment(0);
            let (b0, b1) = segment(1);
            let da = if *reverse_a {
                a1.sub(a0).neg()
            } else {
                a1.sub(a0)
            };
            let db = if *reverse_b {
                b1.sub(b0).neg()
            } else {
                b1.sub(b0)
            };
            // `db` turned back by the value: the residual is the angle left between `da` and it, in (−π, π].
            let (c, sn) = (libm::cos(value), libm::sin(value));
            let e = P {
                x: db.x * c + db.y * sn,
                y: db.y * c - db.x * sn,
            };
            vec![D::atan2(da.cross(e), da.dot(e)) * s]
        }
        Rule::Radius { .. } => vec![circle(0).1 - value],
        Rule::Diameter { .. } => vec![circle(0).1 * 2.0 - value],
    }
}

/// What a dimensional rule measures now: a distance (signed along an axis or from a segment), an angle in
/// [0, 2π), a radius or a diameter, in metres or radians.
pub(super) fn measured(rule: &Rule, parts: &[Built]) -> Option<f64> {
    let probe = rows(rule, parts, 0.0, &[], 1.0);
    let first = probe.first()?.v;
    let v = match rule {
        // The residual at value zero is the measure itself.
        Rule::Distance { .. } | Rule::Radius { .. } | Rule::Diameter { .. } => first,
        Rule::Angle { .. } => {
            let a = first;
            if a < 0.0 {
                a + 2.0 * std::f64::consts::PI
            } else {
                a
            }
        }
        _ => return None,
    };
    v.is_finite().then_some(v)
}
