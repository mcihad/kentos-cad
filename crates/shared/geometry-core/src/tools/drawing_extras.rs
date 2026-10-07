//! Çizim ekleri (docs/adr/0197): what three drawing tools make of their clicks.
//!
//! - **İki daireye teğet**: the common tangents of two circles or arcs (an
//!   arc by its circle, a solution only when its tangent point is on the
//!   arc), outer then inner, left then right; and the one two clicks choose,
//!   its tangent points nearest them (AutoCAD's deferred tangent).
//! - **Dördüncü köşe**: a parallelogram's corner opposite the middle one of three.
//! - **Menzil halkaları**: rings at equal spacing round a centre, and rays
//!   from it to the outer ring, from north clockwise.
//!
//! The tools pick and preview. The independent reference is
//! `scripts/fixtures/drawing_extras_cases.py` (`fixtures/drawing-extras/v1/cases.json`).

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::arc::{ON_ARC_EPS, on_arc, sweep};
use crate::jsmath::{PI, atan2, cos, js_hypot, sin};
use crate::op;
use crate::vec2::Vec2;

/// Most rings and rays (§3).
pub const MAX_RINGS: u32 = 100;
pub const MAX_RAYS: u32 = 360;

/// A common tangent of two circles: from its point on the first to its point
/// on the second. `kind` is `outerLeft`, `outerRight`, `innerLeft` or
/// `innerRight`: outer ones leave both circles on one side, inner ones cross
/// between them; left and right as seen from the first centre to the second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tangent {
    pub kind: &'static str,
    pub a: Vec2,
    pub b: Vec2,
}

crate::json_struct!(out Tangent { kind, a, b });

/// A circle, and for an arc where on it the arc runs (its start and
/// counter-clockwise sweep, radians from east).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Round {
    pub c: Vec2,
    pub r: f64,
    pub span: Option<(f64, f64)>,
}

impl Round {
    /// A circle's or an arc's; none for any other shape or a radius not above zero.
    pub fn of(shape: &Shape) -> Option<Round> {
        let round = match *shape {
            Shape::Circle { c, r } => Round { c, r, span: None },
            Shape::Arc { c, r, a0, a1 } => Round {
                c,
                r,
                span: Some((a0, sweep(a0, a1))),
            },
            _ => return None,
        };
        (round.r > 0.0 && round.r.is_finite()).then_some(round)
    }

    /// Whether a point of its circle is on it: always for a circle; on an arc
    /// within its sweep, its ends with 1e-9 rad.
    fn holds(&self, p: Vec2) -> bool {
        self.span.is_none_or(|(a0, sw)| {
            on_arc(atan2(p.y - self.c.y, p.x - self.c.x), a0, sw, ON_ARC_EPS)
        })
    }
}

/// The common tangents of two circles or arcs (§1): outer left, outer right,
/// inner left, inner right, those there are. Outer ones when the centres are
/// farther apart than the radii's difference, inner ones when farther than
/// their sum (touching circles have neither at the touch); a tangent whose
/// point on an arc is off the arc is none.
pub fn common_tangents(first: &Round, second: &Round) -> Vec<Tangent> {
    let (c1, r1, c2, r2) = (first.c, first.r, second.c, second.r);
    let (dx, dy) = (c2.x - c1.x, c2.y - c1.y);
    let d = js_hypot(dx, dy);
    let mut out = Vec::new();
    if !(d > 0.0) {
        return out;
    }
    let u = Vec2::new(dx / d, dy / d);
    let n = Vec2::new(-u.y, u.x);
    for (inner, sides) in [
        (false, ["outerLeft", "outerRight"]),
        (true, ["innerLeft", "innerRight"]),
    ] {
        let there = if inner {
            d > r1 + r2
        } else {
            d > (r1 - r2).abs()
        };
        if !there {
            continue;
        }
        let gap = if inner { r1 + r2 } else { r1 - r2 };
        let c = gap / d;
        let s = (1.0 - c * c).sqrt();
        for (k, kind) in [1.0, -1.0].into_iter().zip(sides) {
            let m = Vec2::new(c * u.x + k * s * n.x, c * u.y + k * s * n.y);
            let a = Vec2::new(c1.x + r1 * m.x, c1.y + r1 * m.y);
            let sign = if inner { -1.0 } else { 1.0 };
            let b = Vec2::new(c2.x + sign * r2 * m.x, c2.y + sign * r2 * m.y);
            if first.holds(a) && second.holds(b) {
                out.push(Tangent { kind, a, b });
            }
        }
    }
    out
}

/// The tangent two clicks choose (§1): `p1` on the first, `p2` on the
/// second; the least |p1 − a| + |p2 − b|, the first in order on a tie.
pub fn chosen_tangent(tangents: &[Tangent], p1: Vec2, p2: Vec2) -> Option<Tangent> {
    let mut best: Option<(f64, Tangent)> = None;
    for t in tangents {
        let cost = js_hypot(p1.x - t.a.x, p1.y - t.a.y) + js_hypot(p2.x - t.b.x, p2.y - t.b.y);
        if best.is_none_or(|(least, _)| cost < least) {
            best = Some((cost, *t));
        }
    }
    best.map(|(_, t)| t)
}

/// A parallelogram's fourth corner (§2): opposite `b`, the middle of the three.
pub fn fourth_corner(a: Vec2, b: Vec2, c: Vec2) -> Vec2 {
    Vec2::new(a.x + c.x - b.x, a.y + c.y - b.y)
}

/// Range rings (§3): the rings' radii and where the rays end.
#[derive(Clone, Debug, PartialEq)]
pub struct Rings {
    pub radii: Vec<f64>,
    pub rays: Vec<Vec2>,
}

crate::json_struct!(out Rings { radii, rays });

/// Rings `count` (1 to 100) at `spacing` metres (above zero) round
/// `center`, and `rays` (0 to 360) from it to the outer ring, the first due
/// north, the rest clockwise at equal angles. None for values out of range.
pub fn range_rings(center: Vec2, spacing: f64, count: u32, rays: u32) -> Option<Rings> {
    let fits = spacing > 0.0 && spacing.is_finite() && (1..=MAX_RINGS).contains(&count);
    if !fits || rays > MAX_RAYS {
        return None;
    }
    let n = f64::from(count);
    let outer = n * spacing;
    Some(Rings {
        radii: (1..=count).map(|k| f64::from(k) * spacing).collect(),
        rays: (0..rays)
            .map(|j| {
                let t = 2.0 * PI * f64::from(j) / f64::from(rays);
                Vec2::new(center.x + outer * sin(t), center.y + outer * cos(t))
            })
            .collect(),
    })
}

fn rounds(first: &Entity, second: &Entity) -> Option<(Round, Round)> {
    Some((Round::of(&first.shape)?, Round::of(&second.shape)?))
}

pub(crate) static OPS: &[Op] = &[
    // The common tangents of two circles or arcs, in order; none when either is neither.
    op!("commonTangents", |first: Entity, second: Entity| {
        rounds(&first, &second).map_or_else(Vec::new, |(a, b)| common_tangents(&a, &b))
    }),
    // The tangent clicks at `p1` on the first and `p2` on the second choose; null when there is none.
    op!("chosenTangent", |first: Entity,
                          second: Entity,
                          p1: Vec2,
                          p2: Vec2| {
        rounds(&first, &second).and_then(|(a, b)| chosen_tangent(&common_tangents(&a, &b), p1, p2))
    }),
    op!("fourthCorner", |a: Vec2, b: Vec2, c: Vec2| fourth_corner(
        a, b, c
    )),
    // Null for a spacing not above zero, a count not 1 to 100, rays above 360.
    op!("rangeRings", |center: Vec2,
                       spacing: f64,
                       count: f64,
                       rays: f64| {
        let whole = |x: f64| (x.fract() == 0.0 && (0.0..=1e6).contains(&x)).then_some(x as u32);
        whole(count)
            .zip(whole(rays))
            .and_then(|(n, m)| range_rings(center, spacing, n, m))
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn circle(x: f64, y: f64, r: f64) -> Round {
        Round {
            c: v(x, y),
            r,
            span: None,
        }
    }

    #[test]
    fn two_apart_circles_have_four_tangents_in_order() {
        let t = common_tangents(&circle(0.0, 0.0, 5.0), &circle(30.0, 0.0, 5.0));
        let kinds: Vec<_> = t.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            ["outerLeft", "outerRight", "innerLeft", "innerRight"]
        );
        assert!((t[0].a.y - 5.0).abs() < 1e-12 && (t[0].b.y - 5.0).abs() < 1e-12);
        assert!((t[1].a.y + 5.0).abs() < 1e-12);
    }

    #[test]
    fn nested_and_crossing_circles_have_fewer() {
        assert!(common_tangents(&circle(0.0, 0.0, 10.0), &circle(2.0, 1.0, 3.0)).is_empty());
        assert_eq!(
            common_tangents(&circle(0.0, 0.0, 6.0), &circle(8.0, 0.0, 5.0)).len(),
            2
        );
        assert!(common_tangents(&circle(0.0, 0.0, 5.0), &circle(0.0, 0.0, 5.0)).is_empty());
    }

    #[test]
    fn the_clicks_choose_the_nearest() {
        let t = common_tangents(&circle(0.0, 0.0, 5.0), &circle(30.0, 0.0, 5.0));
        let low = chosen_tangent(&t, v(0.0, -5.5), v(30.0, -5.5)).expect("one");
        assert_eq!(low.kind, "outerRight");
        assert!(chosen_tangent(&[], v(0.0, 0.0), v(1.0, 1.0)).is_none());
    }

    #[test]
    fn rings_refuse_values_out_of_range() {
        assert!(range_rings(v(0.0, 0.0), 0.0, 5, 0).is_none());
        assert!(range_rings(v(0.0, 0.0), 10.0, 0, 0).is_none());
        assert!(range_rings(v(0.0, 0.0), 10.0, 101, 0).is_none());
        assert!(range_rings(v(0.0, 0.0), 10.0, 5, 361).is_none());
        let r = range_rings(v(0.0, 0.0), 10.0, 2, 4).expect("rings");
        assert_eq!(r.radii, [10.0, 20.0]);
        assert!((r.rays[1].x - 20.0).abs() < 1e-12 && r.rays[1].y.abs() < 1e-12);
    }
}
