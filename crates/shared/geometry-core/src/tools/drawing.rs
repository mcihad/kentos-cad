//! The drawing tools' constructions (`apps/web/src/tools/shapeTools.ts`,
//! `curveTools.ts`, `ellipseTool.ts`, `constructionTools.ts`, `pathTool.ts`,
//! `annotateTools.ts`, `markupTools.ts`): directions and angles, the
//! typed-radius polygon, the arc continuation, circle and ellipse helpers,
//! construction line directions, polyline arc bulges, text angles and donut
//! rings — what those tools computed inline before (docs/adr/0008, S5).

use crate::api::Op;
use crate::entity::Shape;
use crate::geom::arc::{ArcGeom, Circle, arc_end, norm_angle, point_on_circle};
use crate::geom::bulge::{bulge_at, bulge_of_sweep, segment_tangent};
use crate::geom::ellipse::{EllipseGeom, param_at_polar};
use crate::geom::shapes::regular_polygon;
use crate::geometry::dist;
use crate::jsmath::{PI, asin, atan2, cos, js_hypot, js_sign, or, sin, tan};
use crate::op;
use crate::vec2::Vec2;

const DEG: f64 = PI / 180.0;

/// Direction angle from a to b (radians, CCW from east): rectangle rotation, rotation reference.
pub fn direction_angle(a: Vec2, b: Vec2) -> f64 {
    atan2(b.y - a.y, b.x - a.x)
}

/// Regular polygon for a typed radius: the bottom edge horizontal, so the
/// edge middle sits straight below the centre.
pub fn regular_polygon_radius(c: Vec2, sides: f64, r: f64, inscribed: bool) -> Option<Vec<Vec2>> {
    let mode = if inscribed {
        "inscribed"
    } else {
        "circumscribed"
    };
    regular_polygon(c, sides, regular_radius_point(c, sides, r, inscribed), mode)
}

/// The point a typed radius gives a regular polygon about `c`, its bottom edge horizontal: the first corner
/// (`inscribed`) or the first edge's middle (docs/adr/0290: the point `cad.polygon.regular` is given).
pub fn regular_radius_point(c: Vec2, sides: f64, r: f64, inscribed: bool) -> Vec2 {
    let a = if inscribed {
        -PI / 2.0 + PI / sides
    } else {
        -PI / 2.0
    };
    Vec2::new(c.x + cos(a) * r, c.y + sin(a) * r)
}

/// End point and travel direction of a line, arc or polyline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndTangent {
    pub p: Vec2,
    pub dir: Vec2,
}

crate::json_struct!(EndTangent { p, dir });

/// Where a line, arc or polyline ends and which way it runs there (the arc
/// tool's "Devam"); None for other kinds and zero-length lines. A closed
/// polyline ends where its closing edge does, at its first vertex
/// (docs/adr/0289); a last edge of no length has no direction.
pub fn end_tangent(e: &Shape) -> Option<EndTangent> {
    match e {
        Shape::Polyline {
            pts,
            bulges,
            closed: true,
            ..
        } if pts.len() >= 2 => {
            let k = pts.len();
            (dist(pts[k - 1], pts[0]) > 1e-9).then(|| EndTangent {
                p: pts[0],
                dir: segment_tangent(pts[k - 1], pts[0], bulge_at(bulges.as_deref(), k - 1), true),
            })
        }
        Shape::Polyline { pts, .. } if pts.len() >= 2 && dist(pts[pts.len() - 2], pts[pts.len() - 1]) <= 1e-9 => {
            None
        }
        Shape::Line { a, b } if dist(*a, *b) > 1e-9 => {
            let l = dist(*a, *b);
            Some(EndTangent {
                p: *b,
                dir: Vec2::new((b.x - a.x) / l, (b.y - a.y) / l),
            })
        }
        Shape::Arc { c, r, a0, a1 } => Some(EndTangent {
            p: arc_end(&ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            }),
            dir: Vec2::new(-sin(*a1), cos(*a1)),
        }),
        Shape::Polyline { pts, bulges, .. } if pts.len() >= 2 => {
            let k = pts.len();
            Some(EndTangent {
                p: pts[k - 1],
                dir: segment_tangent(
                    pts[k - 2],
                    pts[k - 1],
                    bulge_at(bulges.as_deref(), k - 2),
                    true,
                ),
            })
        }
        _ => None,
    }
}

/// Where a line, arc or open polyline starts and the way out of it backwards
/// (docs/adr/0289): continuing from an object's start runs on against its
/// direction. A closed polyline has no start apart from its end; none.
pub fn start_tangent(e: &Shape) -> Option<EndTangent> {
    match e {
        Shape::Line { a, b } if dist(*a, *b) > 1e-9 => {
            let l = dist(*a, *b);
            Some(EndTangent {
                p: *a,
                dir: Vec2::new((a.x - b.x) / l, (a.y - b.y) / l),
            })
        }
        Shape::Arc { c, r, a0, .. } => Some(EndTangent {
            p: point_on_circle(*c, *r, *a0),
            dir: Vec2::new(sin(*a0), -cos(*a0)),
        }),
        Shape::Polyline {
            pts,
            bulges,
            closed: false,
            ..
        } if pts.len() >= 2 && dist(pts[0], pts[1]) > 1e-9 => {
            let t = segment_tangent(pts[0], pts[1], bulge_at(bulges.as_deref(), 0), false);
            Some(EndTangent {
                p: pts[0],
                dir: Vec2::new(-t.x, -t.y),
            })
        }
        _ => None,
    }
}

/// Unit direction of a typed angle in degrees (arc start direction).
pub fn deg_direction(deg: f64) -> Vec2 {
    Vec2::new(cos((deg * PI) / 180.0), sin((deg * PI) / 180.0))
}

/// Circle on a diameter (2N).
pub fn circle_on_diameter(a: Vec2, b: Vec2) -> Circle {
    Circle {
        c: Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0),
        r: dist(a, b) / 2.0,
    }
}

/// Ellipse parameter at the polar angle of p seen from the centre, relative to the major axis.
pub fn ellipse_param_toward(g: &EllipseGeom, p: Vec2) -> f64 {
    param_at_polar(
        g,
        atan2(p.y - g.c.y, p.x - g.c.x) - atan2(g.major.y, g.major.x),
    )
}

/// The other half-axis for a rotation angle (degrees): the first axis seen
/// tilted, ratio = cos(angle).
pub fn ellipse_rotation_half(p0: Vec2, p1: Vec2, from_center: bool, angle: f64) -> f64 {
    let a = dist(p0, p1) / (if from_center { 1.0 } else { 2.0 });
    a * cos(angle * DEG)
}

/// Unit vector from a towards b; None when they (nearly) coincide.
pub fn unit_toward(a: Vec2, b: Vec2) -> Option<Vec2> {
    let l = dist(a, b);
    if l < 1e-9 {
        None
    } else {
        Some(Vec2::new((b.x - a.x) / l, (b.y - a.y) / l))
    }
}

/// Direction of a construction line through p in `mode` (`point`,
/// `horizontal`, `vertical`, `angle`, `bisect`); None: not enough input yet.
pub fn xline_direction(mode: &str, pts: &[Vec2], p: Vec2, angle: f64) -> Option<Vec2> {
    match mode {
        "horizontal" => Some(Vec2::new(1.0, 0.0)),
        "vertical" => Some(Vec2::new(0.0, 1.0)),
        "angle" => Some(Vec2::new(cos(angle * DEG), sin(angle * DEG))),
        "bisect" => {
            if pts.len() < 2 {
                return None;
            }
            let u = unit_toward(pts[0], pts[1]);
            let v = unit_toward(pts[0], p);
            let (u, v) = (u?, v?);
            let s = Vec2::new(u.x + v.x, u.y + v.y);
            let l = js_hypot(s.x, s.y);
            // Opposite arms: the bisector is square to them.
            Some(if l < 1e-12 {
                Vec2::new(-u.y, u.x)
            } else {
                Vec2::new(s.x / l, s.y / l)
            })
        }
        _ => pts.first().and_then(|&a| unit_toward(a, p)),
    }
}

/// AutoCAD XLINE's Offset (Paralel, docs/adr/0287): the construction line
/// parallel to the straight edge a→b, `distance` from it on the side `side`
/// lies on, or through `side` when no distance is given. Its point (the foot
/// of `a` on it, or `side`) and unit direction (a→b's); none for an edge of no
/// length, a distance below zero, or a side on the edge's line with a distance.
pub fn xline_offset(a: Vec2, b: Vec2, distance: Option<f64>, side: Vec2) -> Option<(Vec2, Vec2)> {
    let u = unit_toward(a, b)?;
    let Some(d) = distance else {
        return Some((side, u));
    };
    if !d.is_finite() || d < 0.0 {
        return None;
    }
    let cross = u.x * (side.y - a.y) - u.y * (side.x - a.x);
    if cross.abs() < 1e-12 {
        return None;
    }
    let k = if cross > 0.0 { d } else { -d };
    Some((Vec2::new(a.x - u.y * k, a.y + u.x * k), u))
}

/// The point on the circle (c, r) in the direction of p; None when p is the centre.
pub fn radial_point(c: Vec2, r: f64, p: Vec2) -> Option<Vec2> {
    let l = dist(c, p);
    if l < 1e-9 {
        None
    } else {
        Some(Vec2::new(
            c.x + ((p.x - c.x) / l) * r,
            c.y + ((p.y - c.y) / l) * r,
        ))
    }
}

/// Bulge of a polyline arc of radius r from `last` to p, bending the way the
/// path turns towards p (counter-clockwise without a tangent); None when the
/// chord is longer than the diameter.
pub fn radius_bulge(last: Vec2, p: Vec2, r: f64, tangent: Option<Vec2>) -> Option<f64> {
    let c = dist(last, p);
    if c > 2.0 * r {
        return None;
    }
    let side = match tangent {
        Some(t) => or(js_sign(t.x * (p.y - last.y) - t.y * (p.x - last.x)), 1.0),
        None => 1.0,
    };
    Some(side * tan(asin(c / (2.0 * r)) / 2.0))
}

/// Bulge of a polyline arc around centre c from `last` to `end`, counter-clockwise; None for no sweep.
pub fn centre_bulge(c: Vec2, last: Vec2, end: Vec2) -> Option<f64> {
    let sweep = norm_angle(atan2(end.y - c.y, end.x - c.x) - atan2(last.y - c.y, last.x - c.x));
    if sweep > 1e-9 {
        Some(bulge_of_sweep(sweep))
    } else {
        None
    }
}

/// PLINE's arc given its centre and its included angle (docs/adr/0288): the
/// end, `start` turned about `c` by `sweep` radians (positive counter-clockwise).
pub fn arc_end_about(c: Vec2, start: Vec2, sweep: f64) -> Vec2 {
    let (s, k) = (sin(sweep), cos(sweep));
    let (dx, dy) = (start.x - c.x, start.y - c.y);
    Vec2::new(c.x + dx * k - dy * s, c.y + dx * s + dy * k)
}

/// PLINE's arc given its centre and its chord's length (docs/adr/0288): the
/// included angle, 2 asin(|chord| / 2r) with `chord`'s sign (negative
/// clockwise), r the centre's distance from `start`; none for a chord longer
/// than the diameter, of no length, or a centre at the start.
pub fn chord_sweep(c: Vec2, start: Vec2, chord: f64) -> Option<f64> {
    let r = dist(c, start);
    if !(r > 0.0) || chord == 0.0 || !chord.is_finite() || chord.abs() > 2.0 * r {
        return None;
    }
    let half = asin(chord.abs() / (2.0 * r));
    Some(if chord < 0.0 { -2.0 * half } else { 2.0 * half })
}

/// PLINE's arc given its included angle and its radius (docs/adr/0288): the
/// end, the chord 2r sin(|sweep| / 2) laid from `start` toward `toward`; none
/// when `toward` is the start, or for a radius of zero or less.
pub fn chord_end(start: Vec2, toward: Vec2, r: f64, sweep: f64) -> Option<Vec2> {
    let u = unit_toward(start, toward)?;
    if !(r > 0.0) {
        return None;
    }
    let chord = 2.0 * r * sin(sweep.abs() / 2.0);
    Some(Vec2::new(start.x + u.x * chord, start.y + u.y * chord))
}

/// p moved `distance` along `dir`.
pub fn offset_along(p: Vec2, dir: Vec2, distance: f64) -> Vec2 {
    Vec2::new(p.x + dir.x * distance, p.y + dir.y * distance)
}

/// Text angle (degrees) from two points, kept readable: a direction pointing left is turned around.
pub fn text_angle(from: Vec2, p: Vec2) -> f64 {
    let mut a = (atan2(p.y - from.y, p.x - from.x) * 180.0) / PI;
    if a > 90.0 {
        a -= 180.0;
    } else if a <= -90.0 {
        a += 180.0;
    }
    a
}

/// A donut (halka): the outer ring, and the hole when the inner diameter is not zero.
#[derive(Clone, Debug, PartialEq)]
pub struct Donut {
    pub ring: Vec<Vec2>,
    pub holes: Option<Vec<Vec<Vec2>>>,
}

crate::json_struct!(out Donut { ring, holes });

fn circle(c: Vec2, r: f64, n: usize) -> Vec<Vec2> {
    let nf = n as f64;
    (0..n)
        .map(|i| {
            let t = (i as f64 / nf) * 2.0 * PI;
            Vec2::new(c.x + cos(t) * r, c.y + sin(t) * r)
        })
        .collect()
}

/// Donut rings for inner and outer diameters, 96 points each.
pub fn donut_rings(c: Vec2, inner: f64, outer: f64) -> Donut {
    let ring = circle(c, outer / 2.0, 96);
    let holes = if inner > 0.0 {
        Some(vec![circle(c, inner / 2.0, 96)])
    } else {
        None
    };
    Donut { ring, holes }
}

pub(crate) static OPS: &[Op] = &[
    op!("directionAngle", |a: Vec2, b: Vec2| direction_angle(a, b)),
    op!(
        "regularPolygonRadius",
        |c: Vec2, sides: f64, r: f64, inscribed: bool| regular_polygon_radius(
            c, sides, r, inscribed
        )
    ),
    op!("endTangent", |e: Shape| end_tangent(&e)),
    op!("startTangent", |e: Shape| start_tangent(&e)),
    op!("degDirection", |deg: f64| deg_direction(deg)),
    op!("circleOnDiameter", |a: Vec2, b: Vec2| circle_on_diameter(
        a, b
    )),
    op!("ellipseParamToward", |g: EllipseGeom, p: Vec2| {
        ellipse_param_toward(&g, p)
    }),
    op!(
        "ellipseRotationHalf",
        |p0: Vec2, p1: Vec2, from_center: bool, angle: f64| ellipse_rotation_half(
            p0,
            p1,
            from_center,
            angle
        )
    ),
    op!("unitToward", |a: Vec2, b: Vec2| unit_toward(a, b)),
    op!("xlineDirection", |mode: String,
                           pts: Vec<Vec2>,
                           p: Vec2,
                           angle: f64| {
        xline_direction(&mode, &pts, p, angle)
    }),
    op!("radialPoint", |c: Vec2, r: f64, p: Vec2| radial_point(
        c, r, p
    )),
    op!(
        "radiusBulge",
        |last: Vec2, p: Vec2, r: f64, tangent: Option<Vec2>| radius_bulge(last, p, r, tangent)
    ),
    op!(
        "centreBulge",
        |c: Vec2, last: Vec2, end: Vec2| centre_bulge(c, last, end)
    ),
    op!("offsetAlong", |p: Vec2, dir: Vec2, distance: f64| {
        offset_along(p, dir, distance)
    }),
    op!("textAngle", |from: Vec2, p: Vec2| text_angle(from, p)),
    op!("donutRings", |c: Vec2, inner: f64, outer: f64| donut_rings(
        c, inner, outer
    )),
];

#[cfg(test)]
mod method_tests {
    use super::*;

    /// PLINE's arc pairs (docs/adr/0288): the end about a centre, the angle of a chord, the chord's end.
    #[test]
    fn the_arc_pairs() {
        let near = |p: Vec2, x: f64, y: f64| (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9;
        let (c, s) = (Vec2::new(10.0, 10.0), Vec2::new(10.0, 0.0));
        assert!(near(arc_end_about(c, s, PI / 2.0), 20.0, 10.0));
        assert!(near(arc_end_about(c, s, -PI / 2.0), 0.0, 10.0));
        let sweep = chord_sweep(c, s, 10.0).expect("60°");
        assert!((sweep - PI / 3.0).abs() < 1e-12);
        assert!(chord_sweep(c, s, -10.0).is_some_and(|w| (w + PI / 3.0).abs() < 1e-12));
        assert_eq!(chord_sweep(c, s, 30.0), None, "longer than the diameter");
        assert_eq!(chord_sweep(s, s, 1.0), None);
        let end =
            chord_end(Vec2::new(10.0, 0.0), Vec2::new(10.0, 30.0), 5.0, PI).expect("a chord of 10");
        assert!(near(end, 10.0, 10.0));
        assert_eq!(chord_end(s, s, 5.0, PI), None);
    }

    /// XLINE's Offset (docs/adr/0287): on the side shown, the distance away; through the point without one.
    #[test]
    fn the_offset_construction_line() {
        let (a, b) = (Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let left = xline_offset(a, b, Some(2.0), Vec2::new(5.0, 3.0));
        assert_eq!(left, Some((Vec2::new(0.0, 2.0), Vec2::new(1.0, 0.0))));
        let right = xline_offset(a, b, Some(2.0), Vec2::new(5.0, -1.0));
        assert_eq!(right, Some((Vec2::new(0.0, -2.0), Vec2::new(1.0, 0.0))));
        let through = xline_offset(a, b, None, Vec2::new(3.0, 7.0));
        assert_eq!(through, Some((Vec2::new(3.0, 7.0), Vec2::new(1.0, 0.0))));
        assert_eq!(
            xline_offset(a, b, Some(2.0), Vec2::new(4.0, 0.0)),
            None,
            "a side on the line"
        );
        assert_eq!(
            xline_offset(a, a, Some(2.0), Vec2::new(4.0, 1.0)),
            None,
            "no edge"
        );
        assert_eq!(xline_offset(a, b, Some(-1.0), Vec2::new(4.0, 1.0)), None);
        // A slanted edge at TM coordinates: 3-4-5, 5 m to its left.
        let (a, b) = (
            Vec2::new(487_000.0, 4_420_000.0),
            Vec2::new(487_003.0, 4_420_004.0),
        );
        let (p, u) =
            xline_offset(a, b, Some(5.0), Vec2::new(486_990.0, 4_420_010.0)).expect("a line");
        assert_eq!((u.x, u.y), (0.6, 0.8));
        assert_eq!((p.x, p.y), (487_000.0 - 4.0, 4_420_000.0 + 3.0));
    }
}
