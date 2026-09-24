//! The modify, corner and dimension tools' constructions
//! (`apps/web/src/tools/modifyTools.ts`, `arrangeTools.ts`, `cornerTools.ts`,
//! `dimensionTool.ts`): rotation and scale parameters, polar array and align
//! transforms, the corner a fillet or chamfer works on and its pieces, and
//! the arms of angular and radial dimensions — what those tools computed
//! inline before (docs/adr/0008, S5).

use crate::api::Op;
use crate::geom::affine::{Affine, compose, rotation, scaling, translation};
use crate::geom::arc::{ArcGeom, norm_angle};
use crate::geom::dimension::sector_arms;
use crate::geom::intersect::line_line;
use crate::geometry::dist;
use crate::jsmath::{PI, acos, atan2, cos, js_max, js_min, js_round, or, pow, sin, tan};
use crate::op;
use crate::vec2::Vec2;

/// Rotation by the direction from `base` to p, less a reference angle (radians).
pub fn rotation_angle(base: Vec2, p: Vec2, reference: f64) -> f64 {
    atan2(p.y - base.y, p.x - base.x) - reference
}

/// Scale factor: the distance from `base` to p over the reference length.
pub fn scale_factor(base: Vec2, p: Vec2, ref_length: f64) -> f64 {
    dist(base, p) / ref_length
}

/// Polar array around c: `count` items (the original included) over `fill`
/// degrees (minus: clockwise). A full turn shares the circle out; a partial
/// fill puts the last copy on the end angle. Copies that do not turn move by
/// where `reference` (the selection's middle) goes.
pub fn polar_array_transforms(
    c: Vec2,
    count: f64,
    fill: f64,
    rotate: bool,
    reference: Vec2,
) -> Vec<Affine> {
    let step = if (fill.abs() - 360.0).abs() < 1e-9 {
        fill / count
    } else {
        fill / (count - 1.0)
    };
    let mut out = Vec::new();
    let mut k = 1.0;
    while k < count {
        let a = (step * k * PI) / 180.0;
        if rotate {
            out.push(rotation(a, c));
        } else {
            // The middle's offset from the centre, turned: rotating TM-size coordinates and taking
            // them apart again left the move to the last bits of sin and cos (docs/adr/0008, S5).
            let dx = reference.x - c.x;
            let dy = reference.y - c.y;
            let (cs, sn) = (cos(a), sin(a));
            out.push(translation(cs * dx - sn * dy - dx, sn * dx + cs * dy - dy));
        }
        k += 1.0;
    }
    out
}

/// ALIGN: the first source point onto the first destination; with a second
/// pair the source direction turns onto the destination direction, and
/// scales to fit when `scale`. `pts` is s1, d1, s2, d2 as far as given.
pub fn align_transform(pts: &[Vec2], scale: bool) -> Option<Affine> {
    let (Some(&s1), Some(&d1)) = (pts.first(), pts.get(1)) else {
        return None;
    };
    let (Some(&s2), Some(&d2)) = (pts.get(2), pts.get(3)) else {
        return Some(translation(d1.x - s1.x, d1.y - s1.y));
    };
    let ls = dist(s1, s2);
    let ld = dist(d1, d2);
    if ls < 1e-9 || ld < 1e-9 {
        return None;
    }
    let turn = atan2(d2.y - d1.y, d2.x - d1.x) - atan2(s2.y - s1.y, s2.x - s1.x);
    let k = if scale { ld / ls } else { 1.0 };
    // Around s1: scale, turn, then carry s1 onto d1.
    Some(compose(
        &translation(d1.x - s1.x, d1.y - s1.y),
        &compose(&rotation(turn, s1), &scaling(k, s1)),
    ))
}

/// A corner that can be rounded or cut: `u1`/`u2` point along the kept
/// sides, `reach` is how far the shorter side goes, `phi` the angle between
/// the sides (radians).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerGeom {
    pub at: Vec2,
    pub u1: Vec2,
    pub u2: Vec2,
    pub reach: f64,
    pub phi: f64,
}

crate::json_struct!(CornerGeom {
    at,
    u1,
    u2,
    reach,
    phi
});

fn unit(a: Vec2, b: Vec2) -> Vec2 {
    let l = or(dist(a, b), 1.0);
    Vec2::new((b.x - a.x) / l, (b.y - a.y) / l)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn angle_between(u: Vec2, v: Vec2) -> f64 {
    acos(js_max(-1.0, js_min(1.0, dot(u, v))))
}

/// The corner at a path vertex `at` between the segment from `prev` (bulge
/// `bulge_in`) and the one to `next` (`bulge_out`); None beside an arc, on a
/// straight run or where the path turns back.
pub fn vertex_corner(
    prev: Vec2,
    at: Vec2,
    next: Vec2,
    bulge_in: f64,
    bulge_out: f64,
) -> Option<CornerGeom> {
    if bulge_in.abs() > 1e-12 || bulge_out.abs() > 1e-12 {
        return None;
    }
    let u1 = unit(at, prev);
    let u2 = unit(at, next);
    let phi = angle_between(u1, u2);
    if phi < 1e-6 || PI - phi < 1e-6 {
        return None;
    }
    Some(CornerGeom {
        at,
        u1,
        u2,
        reach: js_min(dist(at, prev), dist(at, next)),
        phi,
    })
}

/// Corner of two lines (a1–b1 picked at p1, a2–b2 at p2); each keeps the
/// side its pick point is on.
pub fn lines_corner_at(
    a1: Vec2,
    b1: Vec2,
    p1: Vec2,
    a2: Vec2,
    b2: Vec2,
    p2: Vec2,
) -> Option<CornerGeom> {
    let x = line_line(a1, b1, a2, b2)?.p;
    let side = |a: Vec2, b: Vec2, pick: Vec2| {
        let d = unit(a, b);
        let u = if dot(Vec2::new(pick.x - x.x, pick.y - x.y), d) >= 0.0 {
            d
        } else {
            Vec2::new(-d.x, -d.y)
        };
        let reach = js_max(
            dot(Vec2::new(a.x - x.x, a.y - x.y), u),
            dot(Vec2::new(b.x - x.x, b.y - x.y), u),
        );
        (u, reach)
    };
    let (u1, r1) = side(a1, b1, p1);
    let (u2, r2) = side(a2, b2, p2);
    if r1 <= 1e-9 || r2 <= 1e-9 {
        return None;
    }
    let phi = angle_between(u1, u2);
    if phi < 1e-6 || PI - phi < 1e-6 {
        return None;
    }
    Some(CornerGeom {
        at: x,
        u1,
        u2,
        reach: js_min(r1, r2),
        phi,
    })
}

/// How far the cursor has been pulled along the nearer side, rounded to a
/// step that suits the zoom (`tol`: four pixels in metres); the corner
/// itself without a cursor.
pub fn pulled_distance(c: &CornerGeom, cursor: Option<Vec2>, tol: f64) -> f64 {
    let cur = cursor.unwrap_or(c.at);
    let v = Vec2::new(cur.x - c.at.x, cur.y - c.at.y);
    let t = js_min(js_max(js_max(dot(v, c.u1), dot(v, c.u2)), 0.0), c.reach);
    let step = pow(10.0, libm::log10(js_max(tol, 1e-3)).floor());
    js_min(js_round(t / step) * step, c.reach)
}

/// Fillet radius for a pulled distance: that is where the arc meets the side (tangent length).
pub fn fillet_radius_for(t: f64, phi: f64) -> f64 {
    t * tan(phi / 2.0)
}

/// The fillet arc alone ("Kırp: hayır"): the short arc between the tangent
/// points; None for no radius.
pub fn fillet_arc(c: &CornerGeom, radius: f64) -> Option<ArcGeom> {
    if !(radius > 0.0) {
        return None;
    }
    // Tangent points at r / tan(φ/2) along each side; centre on the bisector at r / sin(φ/2).
    let t = radius / tan(c.phi / 2.0);
    let bis = unit(
        Vec2::new(0.0, 0.0),
        Vec2::new(c.u1.x + c.u2.x, c.u1.y + c.u2.y),
    );
    let k = radius / sin(c.phi / 2.0);
    let centre = Vec2::new(c.at.x + bis.x * k, c.at.y + bis.y * k);
    let angle = |u: Vec2| atan2(c.at.y + u.y * t - centre.y, c.at.x + u.x * t - centre.x);
    let (a0, a1) = (angle(c.u1), angle(c.u2));
    // The fillet is the short arc between the tangent points; arcs run counter-clockwise.
    let ccw = (((a1 - a0) % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI) < PI;
    Some(ArcGeom {
        c: centre,
        r: radius,
        a0: if ccw { a0 } else { a1 },
        a1: if ccw { a1 } else { a0 },
    })
}

/// A straight piece from `a` to `b`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub a: Vec2,
    pub b: Vec2,
}

crate::json_struct!(out Piece { a, b });

/// The chamfer cut alone ("Kırp: hayır"); None unless both distances are positive.
pub fn chamfer_line(c: &CornerGeom, d1: f64, d2: f64) -> Option<Piece> {
    if !(d1 > 0.0) || !(d2 > 0.0) {
        return None;
    }
    Some(Piece {
        a: Vec2::new(c.at.x + c.u1.x * d1, c.at.y + c.u1.y * d1),
        b: Vec2::new(c.at.x + c.u2.x * d2, c.at.y + c.u2.y * d2),
    })
}

/// The vertex and arm points of an angular dimension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arms {
    pub c: Vec2,
    pub a: Vec2,
    pub b: Vec2,
}

crate::json_struct!(out Arms { c, a, b });

/// Angular dimension from its vertex c and a point on each arm: the arc goes
/// where it is placed (`loc`), from p1 to p2 counter-clockwise, or the other
/// way round.
pub fn vertex_arms(c: Vec2, p1: Vec2, p2: Vec2, loc: Vec2) -> Arms {
    let t = norm_angle(atan2(loc.y - c.y, loc.x - c.x) - atan2(p1.y - c.y, p1.x - c.x));
    let sweep = norm_angle(atan2(p2.y - c.y, p2.x - c.x) - atan2(p1.y - c.y, p1.x - c.x));
    if t <= sweep {
        Arms { c, a: p1, b: p2 }
    } else {
        Arms { c, a: p2, b: p1 }
    }
}

/// A straight edge picked for an angular dimension, and where it was clicked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickedEdge {
    pub a: Vec2,
    pub b: Vec2,
    pub at: Vec2,
}

crate::json_struct!(PickedEdge { a, b, at });

/// Angular dimension between two picked edges: the vertex where their lines
/// cross, the sector the arc is placed in (`loc`), each arm as far as its
/// edge was clicked (at least a millimetre); None for parallel edges.
pub fn edge_arms(e1: &PickedEdge, e2: &PickedEdge, loc: Vec2) -> Option<Arms> {
    let c = line_line(e1.a, e1.b, e2.a, e2.b)?.p;
    let dir = |s: &PickedEdge| {
        let l = dist(s.a, s.b);
        Vec2::new((s.b.x - s.a.x) / l, (s.b.y - s.a.y) / l)
    };
    let u1 = dir(e1);
    let u2 = dir(e2);
    let [s, e] = sector_arms(c, u1, u2, loc);
    let reach = |u: Vec2| {
        let edge = if (u.x * u1.y - u.y * u1.x).abs() < 1e-9 {
            e1
        } else {
            e2
        };
        js_max(dist(c, edge.at), 1e-3)
    };
    Some(Arms {
        c,
        a: Vec2::new(c.x + s.x * reach(s), c.y + s.y * reach(s)),
        b: Vec2::new(c.x + e.x * reach(e), c.y + e.y * reach(e)),
    })
}

/// A radius or diameter dimension's point on the circle and how far outside it is placed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radial {
    pub b: Vec2,
    pub offset: f64,
}

crate::json_struct!(out Radial { b, offset });

/// Radius or diameter dimension placed at `loc`: the point on the circle towards it.
pub fn radial_dimension(c: Vec2, r: f64, loc: Vec2) -> Radial {
    let l = dist(c, loc);
    let u = if l > 1e-9 {
        Vec2::new((loc.x - c.x) / l, (loc.y - c.y) / l)
    } else {
        Vec2::new(1.0, 0.0)
    };
    Radial {
        b: Vec2::new(c.x + u.x * r, c.y + u.y * r),
        offset: js_max(0.0, l - r),
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("rotationAngle", |base: Vec2, p: Vec2, reference: f64| {
        rotation_angle(base, p, reference)
    }),
    op!("scaleFactor", |base: Vec2, p: Vec2, ref_length: f64| {
        scale_factor(base, p, ref_length)
    }),
    op!(
        "polarArrayTransforms",
        |c: Vec2, count: f64, fill: f64, rotate: bool, reference: Vec2| polar_array_transforms(
            c, count, fill, rotate, reference
        )
    ),
    op!("alignTransform", |pts: Vec<Vec2>, scale: bool| {
        align_transform(&pts, scale)
    }),
    op!("vertexCorner", |prev: Vec2,
                         at: Vec2,
                         next: Vec2,
                         bulge_in: f64,
                         bulge_out: f64| {
        vertex_corner(prev, at, next, bulge_in, bulge_out)
    }),
    op!("linesCornerAt", |a1: Vec2,
                          b1: Vec2,
                          p1: Vec2,
                          a2: Vec2,
                          b2: Vec2,
                          p2: Vec2| {
        lines_corner_at(a1, b1, p1, a2, b2, p2)
    }),
    op!("pulledDistance", |c: CornerGeom,
                           cursor: Option<Vec2>,
                           tol: f64| {
        pulled_distance(&c, cursor, tol)
    }),
    op!("filletRadiusFor", |t: f64, phi: f64| fillet_radius_for(
        t, phi
    )),
    op!("filletArc", |c: CornerGeom, radius: f64| fillet_arc(
        &c, radius
    )),
    op!("chamferLine", |c: CornerGeom, d1: f64, d2: f64| {
        chamfer_line(&c, d1, d2)
    }),
    op!("vertexArms", |c: Vec2, p1: Vec2, p2: Vec2, loc: Vec2| {
        vertex_arms(c, p1, p2, loc)
    }),
    op!("edgeArms", |e1: PickedEdge, e2: PickedEdge, loc: Vec2| {
        edge_arms(&e1, &e2, loc)
    }),
    op!("radialDimension", |c: Vec2, r: f64, loc: Vec2| {
        radial_dimension(c, r, loc)
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_rotating_polar_copies_move_the_same_in_a_tm_zone() {
        // Rotating TM-size coordinates and taking them apart again left the
        // move to the last bits of sin and cos (docs/adr/0008, S5).
        let near = polar_array_transforms(
            Vec2::new(0.0, 0.0),
            11.0,
            360.0,
            false,
            Vec2::new(10.5, 3.25),
        );
        let tm = polar_array_transforms(
            Vec2::new(486000.0, 4420000.0),
            11.0,
            360.0,
            false,
            Vec2::new(486010.5, 4420003.25),
        );
        assert_eq!(near.len(), 10);
        assert_eq!(near, tm);
    }

    #[test]
    fn a_pulled_size_rounds_to_a_step_that_suits_the_zoom() {
        let c = CornerGeom {
            at: Vec2::new(0.0, 0.0),
            u1: Vec2::new(1.0, 0.0),
            u2: Vec2::new(0.0, 1.0),
            reach: 10.0,
            phi: PI / 2.0,
        };
        let pulled = |x: f64, tol: f64| pulled_distance(&c, Some(Vec2::new(x, 0.5)), tol);
        assert!((pulled(2.34567, 0.037) - 2.35).abs() < 1e-12);
        assert!((pulled(2.34567, 0.37) - 2.3).abs() < 1e-12);
        assert_eq!(pulled(30.0, 0.037), 10.0);
        assert_eq!(pulled_distance(&c, None, 0.037), 0.0);
    }
}
