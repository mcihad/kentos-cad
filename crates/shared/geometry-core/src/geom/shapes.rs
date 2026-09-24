//! Constructions behind the drawing tools (`apps/web/src/model/geom/shapes.ts`):
//! rectangles, regular polygons, AutoCAD's arc modes, revision clouds.

use crate::api::Op;
use crate::geom::arc::{ArcGeom, norm_angle};
use crate::geom::bulge::{bulge_arc, tangent_bulge};
use crate::geometry::{dist, signed_area};
use crate::jsmath::{PI, asin, atan2, cos, js_hypot, js_max, js_min, js_round, or, sin, tan};
use crate::op;
use crate::vec2::Vec2;

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}
fn len(v: Vec2) -> f64 {
    js_hypot(v.x, v.y)
}
fn ang(from: Vec2, to: Vec2) -> f64 {
    atan2(to.y - from.y, to.x - from.x)
}
fn at(c: Vec2, r: f64, a: f64) -> Vec2 {
    Vec2::new(c.x + cos(a) * r, c.y + sin(a) * r)
}

/// Rectangle on edge p1→p2, `width` to its left (negative: right).
pub fn rect_from_edge(p1: Vec2, p2: Vec2, width: f64) -> Option<Vec<Vec2>> {
    let d = sub(p2, p1);
    let l = len(d);
    if l < 1e-9 || width.abs() < 1e-9 {
        return None;
    }
    let n = Vec2::new((-d.y / l) * width, (d.x / l) * width);
    Some(vec![
        p1,
        p2,
        Vec2::new(p2.x + n.x, p2.y + n.y),
        Vec2::new(p1.x + n.x, p1.y + n.y),
    ])
}

/// Signed distance of p from the line p1→p2 (positive on the left).
pub fn side_distance(p1: Vec2, p2: Vec2, p: Vec2) -> f64 {
    let d = sub(p2, p1);
    let l = or(len(d), 1.0);
    (d.x * (p.y - p1.y) - d.y * (p.x - p1.x)) / l
}

/// Rectangle with opposite corners a and b whose sides run at `rotation` (radians).
pub fn rect_from_corners(a: Vec2, b: Vec2, rotation: f64) -> Option<Vec<Vec2>> {
    let u = Vec2::new(cos(rotation), sin(rotation));
    let v = Vec2::new(-u.y, u.x);
    let w = sub(b, a);
    let du = w.x * u.x + w.y * u.y;
    let dv = w.x * v.x + w.y * v.y;
    if du.abs() < 1e-9 || dv.abs() < 1e-9 {
        return None;
    }
    Some(vec![
        a,
        Vec2::new(a.x + u.x * du, a.y + u.y * du),
        Vec2::new(a.x + u.x * du + v.x * dv, a.y + u.y * du + v.y * dv),
        Vec2::new(a.x + v.x * dv, a.y + v.y * dv),
    ])
}

/// Rectangle of `length` × `width` from corner a, in the quadrant `towards` lies in.
pub fn rect_from_size(
    a: Vec2,
    length: f64,
    width: f64,
    rotation: f64,
    towards: Vec2,
) -> Option<Vec<Vec2>> {
    let u = Vec2::new(cos(rotation), sin(rotation));
    let w = sub(towards, a);
    let su = if w.x * u.x + w.y * u.y >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let sv = if -w.x * u.y + w.y * u.x >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let b = Vec2::new(
        a.x + u.x * length * su - u.y * width * sv,
        a.y + u.y * length * su + u.x * width * sv,
    );
    rect_from_corners(a, b, rotation)
}

/// A count as JavaScript's `Array.from({ length })` reads it.
fn count(n: f64) -> usize {
    if n.is_nan() || n <= 0.0 {
        0
    } else {
        n as usize
    }
}

/// Regular n-gon about `center`: inscribed (p a vertex) or circumscribed (p an edge middle).
pub fn regular_polygon(center: Vec2, sides: f64, p: Vec2, mode: &str) -> Option<Vec<Vec2>> {
    let n = js_round(sides);
    let r0 = len(sub(p, center));
    if n < 3.0 || r0 < 1e-9 {
        return None;
    }
    let a0 = ang(center, p);
    let inscribed = mode == "inscribed";
    let r = if inscribed { r0 } else { r0 / cos(PI / n) };
    let start = if inscribed { a0 } else { a0 - PI / n };
    Some(
        (0..count(n))
            .map(|k| at(center, r, start + (2.0 * PI * k as f64) / n))
            .collect(),
    )
}

/// Regular n-gon built on edge p1→p2, counter-clockwise (to the left of the edge).
pub fn regular_polygon_on_edge(p1: Vec2, p2: Vec2, sides: f64) -> Option<Vec<Vec2>> {
    let n = js_round(sides);
    let s = len(sub(p2, p1));
    if n < 3.0 || s < 1e-9 {
        return None;
    }
    let a = ang(p1, p2);
    let mut out = vec![p1];
    let mut k = 0.0;
    while k < n - 1.0 {
        let q = out[k as usize];
        let t = a + (2.0 * PI * k) / n;
        out.push(if k == 0.0 {
            p2
        } else {
            Vec2::new(q.x + cos(t) * s, q.y + sin(t) * s)
        });
        k += 1.0;
    }
    Some(out)
}

/// Arc from a signed sweep to the stored counter-clockwise form.
fn ccw(c: Vec2, r: f64, a0: f64, sweep: f64) -> Option<ArcGeom> {
    if !(r > 1e-9) || sweep.abs() < 1e-9 || !r.is_finite() {
        return None;
    }
    let s = a0;
    let e = a0 + sweep;
    Some(if sweep > 0.0 {
        ArcGeom {
            c,
            r,
            a0: norm_angle(s),
            a1: norm_angle(e),
        }
    } else {
        ArcGeom {
            c,
            r,
            a0: norm_angle(e),
            a1: norm_angle(s),
        }
    })
}

/// Start, centre, end direction: counter-clockwise from start to the ray through `end`.
pub fn arc_start_center_end(start: Vec2, center: Vec2, end: Vec2) -> Option<ArcGeom> {
    let r = len(sub(start, center));
    if r < 1e-9 || len(sub(end, center)) < 1e-9 {
        return None;
    }
    let a0 = ang(center, start);
    let mut sw = norm_angle(ang(center, end) - a0);
    if sw < 1e-9 {
        sw = 2.0 * PI;
    }
    ccw(center, r, a0, sw)
}

/// Start, centre, included angle (degrees; negative runs clockwise).
pub fn arc_start_center_angle(start: Vec2, center: Vec2, degrees: f64) -> Option<ArcGeom> {
    let r = len(sub(start, center));
    ccw(center, r, ang(center, start), (degrees * PI) / 180.0)
}

/// Start, centre, chord length (positive: minor arc CCW; negative: major arc).
pub fn arc_start_center_chord(start: Vec2, center: Vec2, chord: f64) -> Option<ArcGeom> {
    let r = len(sub(start, center));
    let c = chord.abs();
    if r < 1e-9 || c < 1e-9 || c > 2.0 * r + 1e-9 {
        return None;
    }
    let minor = 2.0 * asin(js_min(1.0, c / (2.0 * r)));
    ccw(
        center,
        r,
        ang(center, start),
        if chord > 0.0 { minor } else { 2.0 * PI - minor },
    )
}

/// Start, end, included angle (degrees; positive counter-clockwise from start to end).
pub fn arc_start_end_angle(start: Vec2, end: Vec2, degrees: f64) -> Option<ArcGeom> {
    let t = (degrees * PI) / 180.0;
    if t.abs() < 1e-9 || t.abs() >= 2.0 * PI {
        return None;
    }
    let a = bulge_arc(start, end, tan(t / 4.0))?;
    ccw(a.c, a.r, a.a0, a.sweep)
}

/// Start, end, tangent direction at the start.
pub fn arc_start_end_direction(start: Vec2, end: Vec2, dir: Vec2) -> Option<ArcGeom> {
    let b = tangent_bulge(start, dir, end)?;
    let a = bulge_arc(start, end, b)?;
    ccw(a.c, a.r, a.a0, a.sweep)
}

/// Start, end, radius (positive: minor arc CCW from start to end; negative: major arc).
pub fn arc_start_end_radius(start: Vec2, end: Vec2, radius: f64) -> Option<ArcGeom> {
    let d = len(sub(end, start));
    let r = radius.abs();
    if d < 1e-9 || r < d / 2.0 - 1e-9 {
        return None;
    }
    let half = asin(js_min(1.0, d / (2.0 * r)));
    arc_start_end_angle(
        start,
        end,
        ((if radius > 0.0 {
            2.0 * half
        } else {
            2.0 * PI - 2.0 * half
        }) * 180.0)
            / PI,
    )
}

/// Start, end and centre (radius from the start; the end fixes the direction).
pub fn arc_start_end_center(start: Vec2, end: Vec2, center: Vec2) -> Option<ArcGeom> {
    arc_start_center_end(start, center, end)
}

/// Bulge of the cloud's scallops: about 106° arcs, outward on a counter-clockwise ring.
const SCALLOP: f64 = 0.5;

#[derive(Clone, Debug, PartialEq)]
pub struct Cloud {
    pub pts: Vec<Vec2>,
    pub bulges: Vec<f64>,
}

crate::json_struct!(Cloud { pts, bulges });

/// Scalloped outline of a ring: sides divided into chords of about `arc` metres, all bulging out.
pub fn cloud_of(ring: &[Vec2], arc: f64) -> Option<Cloud> {
    if ring.len() < 3 || !(arc > 0.0) {
        return None;
    }
    let ccw: Vec<Vec2> = if signed_area(ring) > 0.0 {
        ring.to_vec()
    } else {
        ring.iter().rev().copied().collect()
    };
    let mut pts = Vec::new();
    for i in 0..ccw.len() {
        let a = ccw[i];
        let b = ccw[(i + 1) % ccw.len()];
        let k = js_max(1.0, js_round(dist(a, b) / arc));
        let mut j = 0.0;
        while j < k {
            pts.push(Vec2::new(
                a.x + ((b.x - a.x) * j) / k,
                a.y + ((b.y - a.y) * j) / k,
            ));
            j += 1.0;
        }
    }
    let bulges = vec![SCALLOP; pts.len()];
    Some(Cloud { pts, bulges })
}

pub(crate) static OPS: &[Op] = &[
    op!("rectFromEdge", |p1: Vec2, p2: Vec2, width: f64| {
        rect_from_edge(p1, p2, width)
    }),
    op!("sideDistance", |p1: Vec2, p2: Vec2, p: Vec2| side_distance(
        p1, p2, p
    )),
    op!(
        "rectFromCorners",
        |a: Vec2, b: Vec2, rotation: Option<f64>| rect_from_corners(a, b, rotation.unwrap_or(0.0))
    ),
    op!("rectFromSize", |a: Vec2,
                         length: f64,
                         width: f64,
                         rotation: f64,
                         towards: Vec2| {
        rect_from_size(a, length, width, rotation, towards)
    }),
    op!("regularPolygon", |center: Vec2,
                           sides: f64,
                           p: Vec2,
                           mode: String| {
        regular_polygon(center, sides, p, &mode)
    }),
    op!("regularPolygonOnEdge", |p1: Vec2, p2: Vec2, sides: f64| {
        regular_polygon_on_edge(p1, p2, sides)
    }),
    op!(
        "arcStartCenterEnd",
        |start: Vec2, center: Vec2, end: Vec2| arc_start_center_end(start, center, end)
    ),
    op!(
        "arcStartCenterAngle",
        |start: Vec2, center: Vec2, degrees: f64| arc_start_center_angle(start, center, degrees)
    ),
    op!(
        "arcStartCenterChord",
        |start: Vec2, center: Vec2, chord: f64| arc_start_center_chord(start, center, chord)
    ),
    op!(
        "arcStartEndAngle",
        |start: Vec2, end: Vec2, degrees: f64| arc_start_end_angle(start, end, degrees)
    ),
    op!(
        "arcStartEndDirection",
        |start: Vec2, end: Vec2, dir: Vec2| arc_start_end_direction(start, end, dir)
    ),
    op!(
        "arcStartEndRadius",
        |start: Vec2, end: Vec2, radius: f64| arc_start_end_radius(start, end, radius)
    ),
    op!(
        "arcStartEndCenter",
        |start: Vec2, end: Vec2, center: Vec2| arc_start_end_center(start, end, center)
    ),
    op!("cloudOf", |ring: Vec<Vec2>, arc: f64| cloud_of(&ring, arc)),
];
