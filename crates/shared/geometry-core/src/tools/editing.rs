//! The modify, corner and dimension tools' constructions
//! (`apps/web/src/tools/modifyTools.ts`, `arrangeTools.ts`, `cornerTools.ts`,
//! `dimensionTool.ts`): rotation and scale parameters, rectangular and polar
//! array and align transforms, the corner a fillet or chamfer works on and
//! its pieces, and the arms of angular and radial dimensions — what those
//! tools computed inline before (docs/adr/0008, S5, 0047).

use crate::api::Op;
use crate::entity::{Entity, Shape, entity_bounds_in};
use crate::geom::affine::{Affine, align, rotation, translation};
use crate::geom::arc::{ArcGeom, norm_angle};
use crate::geom::bulge::bulge_at;
use crate::geom::dimension::sector_arms;
use crate::geom::intersect::line_line;
use crate::geometry::{dist, empty_bounds, is_empty_bounds};
use crate::jsmath::{PI, acos, atan2, cos, js_max, js_min, js_round, or, pow, sin, tan};
use crate::op;
use crate::text::Font;
use crate::tools::point_input::midpoint;
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

/// Most places a rectangular array fills, the originals' among them, and the
/// most items of a polar array (docs/adr/0047): what the web's tools took.
pub const GRID_PLACES_MAX: f64 = 10_000.0;
pub const POLAR_COUNT_MAX: f64 = 1_000.0;

/// Rectangular array (Dizi): `rows` × `cols` places, row after row and each
/// row column after column, the originals' place left out; the copy `j`
/// columns and `i` rows away moves `j·dx` east and `i·dy` north. Nothing
/// for fewer than one row or column, or more than [`GRID_PLACES_MAX`] places.
pub fn grid_array_transforms(rows: f64, cols: f64, dx: f64, dy: f64) -> Vec<Affine> {
    let mut out = Vec::new();
    if !(rows >= 1.0 && cols >= 1.0 && rows * cols <= GRID_PLACES_MAX) {
        return out;
    }
    let mut i = 0.0;
    while i < rows {
        let mut j = 0.0;
        while j < cols {
            if i != 0.0 || j != 0.0 {
                out.push(translation(j * dx, i * dy));
            }
            j += 1.0;
        }
        i += 1.0;
    }
    out
}

/// The middle of the box around `shapes`, text measured in `font`, as the
/// geometry store's `extent` takes it and the web's polar array tool took
/// its middle: where copies that do not turn are placed by. None when there
/// is nothing to measure.
pub fn shapes_middle(shapes: &[Shape], font: Font) -> Option<Vec2> {
    let mut b = empty_bounds();
    for s in shapes {
        let e = entity_bounds_in(s, font);
        b.min_x = js_min(b.min_x, e.min_x);
        b.min_y = js_min(b.min_y, e.min_y);
        b.max_x = js_max(b.max_x, e.max_x);
        b.max_y = js_max(b.max_y, e.max_y);
    }
    if is_empty_bounds(&b) {
        return None;
    }
    Some(midpoint(
        Vec2::new(b.min_x, b.min_y),
        Vec2::new(b.max_x, b.max_y),
    ))
}

/// The copies' affines of an array, in the order its copies are written
/// (the product command `cad.entities.array`, docs/adr/0047): `grid` with
/// rows, cols, dx, dy ([`grid_array_transforms`]); `polar` with cx, cy,
/// count, fill in degrees and rotate (1 turns the copies, 0 does not), the
/// copies that do not turn placed by the middle of `shapes`
/// ([`shapes_middle`], text measured in `font`). None for another kind or
/// count of numbers, counts out of their ranges (whole numbers; 2 to
/// [`GRID_PLACES_MAX`] places, 2 to [`POLAR_COUNT_MAX`] items), or a polar
/// array that does not turn with nothing to measure. Whether the numbers are
/// finite and the spacing or the angle make sense is the command's to check
/// first.
pub fn array_transforms(
    kind: &str,
    p: &[f64],
    shapes: &[Shape],
    font: Font,
) -> Option<Vec<Affine>> {
    let whole = |n: f64| n.fract() == 0.0;
    match (kind, p) {
        ("grid", &[rows, cols, dx, dy]) => {
            let fits = whole(rows)
                && whole(cols)
                && rows >= 1.0
                && cols >= 1.0
                && (2.0..=GRID_PLACES_MAX).contains(&(rows * cols));
            fits.then(|| grid_array_transforms(rows, cols, dx, dy))
        }
        ("polar", &[cx, cy, count, fill, rotate]) => {
            if !(whole(count) && (2.0..=POLAR_COUNT_MAX).contains(&count)) {
                return None;
            }
            let turns = rotate != 0.0;
            // A copy that turns does not need the middle; one that does not is placed by it.
            let reference = if turns {
                Vec2::new(cx, cy)
            } else {
                shapes_middle(shapes, font)?
            };
            Some(polar_array_transforms(
                Vec2::new(cx, cy),
                count,
                fill,
                turns,
                reference,
            ))
        }
        _ => None,
    }
}

/// ALIGN: the first source point onto the first destination; with a second
/// pair the source direction turns onto the destination direction, and
/// scales to fit when `scale`. `pts` is s1, d1, s2, d2 as far as given
/// ([`align`]: the transform command builds its alignment there too).
pub fn align_transform(pts: &[Vec2], scale: bool) -> Option<Affine> {
    align(pts, scale)
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

/// Where a corner under the cursor is: a vertex of a path, or where two
/// lines meet end to end. Objects are named by their place in the
/// candidates given to [`corner_near`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerSite {
    /// Vertex `vertex` of the path `object`.
    Vertex { object: usize, vertex: usize },
    /// The lines `first` and `second`, whose ends meet; each keeps the side
    /// of its pick point (its far end), as a fillet of two picked lines does.
    Lines {
        first: usize,
        pick1: Vec2,
        second: usize,
        pick2: Vec2,
    },
}

crate::json_tagged!(
    CornerSite,
    "kind",
    Vertex => "vertex" { object, vertex },
    Lines => "lines" { first, pick1, second, pick2 },
);

/// The corner nearest the cursor and its geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerHit {
    pub site: CornerSite,
    pub corner: CornerGeom,
}

crate::json_struct!(out CornerHit { site, corner });

/// The corner a fillet or chamfer finds under the cursor (the corner tools'
/// `cornerAt`, docs/adr/0047): among `candidates` (lines, polylines and
/// closed areas; anything else is passed over), a path vertex within `tol`
/// of `p`, or two lines whose ends lie within `same` of each other with one
/// of those ends within `tol` of `p`. The nearest corner wins, the first on
/// a tie; a vertex beside an arc, on a straight run or where the path turns
/// back is no corner (`vertex_corner`, `lines_corner_at`).
pub fn corner_near(candidates: &[Shape], p: Vec2, tol: f64, same: f64) -> Option<CornerHit> {
    let mut best: Option<(CornerHit, f64)> = None;
    let mut consider = |site: CornerSite, corner: Option<CornerGeom>| {
        let Some(corner) = corner else { return };
        let d = dist(corner.at, p);
        if d <= tol && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
            best = Some((CornerHit { site, corner }, d));
        }
    };
    // The end of a line away from `near`: the side a pick there keeps.
    let far_end = |a: Vec2, b: Vec2, near: Vec2| if dist(a, near) <= dist(b, near) { b } else { a };
    for (i, e) in candidates.iter().enumerate() {
        match e {
            Shape::Line { a, b } => {
                for end in [*a, *b] {
                    if dist(end, p) > tol {
                        continue;
                    }
                    for (j, o) in candidates.iter().enumerate() {
                        let Shape::Line { a: oa, b: ob } = o else {
                            continue;
                        };
                        if j == i {
                            continue;
                        }
                        let o_end = if dist(*oa, end) <= same {
                            *oa
                        } else if dist(*ob, end) <= same {
                            *ob
                        } else {
                            continue;
                        };
                        let (pick1, pick2) = (far_end(*a, *b, end), far_end(*oa, *ob, o_end));
                        consider(
                            CornerSite::Lines {
                                first: i,
                                pick1,
                                second: j,
                                pick2,
                            },
                            lines_corner_at(*a, *b, pick1, *oa, *ob, pick2),
                        );
                    }
                }
            }
            Shape::Polyline { pts, bulges, .. } | Shape::Polygon { pts, bulges, .. } => {
                let closed = matches!(e, Shape::Polygon { .. });
                let n = pts.len();
                for (k, q) in pts.iter().enumerate() {
                    if dist(*q, p) > tol || (!closed && (k == 0 || k + 1 >= n)) {
                        continue;
                    }
                    let prev = (k + n - 1) % n;
                    let corner = vertex_corner(
                        pts[prev],
                        *q,
                        pts[(k + 1) % n],
                        bulge_at(bulges.as_deref(), prev),
                        bulge_at(bulges.as_deref(), k),
                    );
                    consider(
                        CornerSite::Vertex {
                            object: i,
                            vertex: k,
                        },
                        corner,
                    );
                }
            }
            _ => {}
        }
    }
    best.map(|(hit, _)| hit)
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
    op!(
        "gridArrayTransforms",
        |rows: f64, cols: f64, dx: f64, dy: f64| { grid_array_transforms(rows, cols, dx, dy) }
    ),
    op!(
        "shapesMiddle",
        |objects: Vec<Entity>, font: Option<String>| {
            let shapes: Vec<Shape> = objects.into_iter().map(|e| e.shape).collect();
            shapes_middle(&shapes, font.map_or(Font::DEFAULT, |id| Font::from_id(&id)))
        }
    ),
    op!(
        "arrayTransforms",
        |kind: String, params: Vec<f64>, objects: Vec<Entity>, font: Option<String>| {
            let shapes: Vec<Shape> = objects.into_iter().map(|e| e.shape).collect();
            let font = font.map_or(Font::DEFAULT, |id| Font::from_id(&id));
            array_transforms(&kind, &params, &shapes, font)
        }
    ),
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
    op!("cornerNear", |candidates: Vec<Entity>,
                       p: Vec2,
                       tol: f64,
                       same: f64| {
        let shapes: Vec<Shape> = candidates.into_iter().map(|e| e.shape).collect();
        corner_near(&shapes, p, tol, same)
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

    /// A 2 × 3 grid: five copies, row after row, each a product of its
    /// place and the spacing; one row or column alone is a line of copies;
    /// nothing outside the places the web's tool took.
    #[test]
    fn a_grid_array_goes_row_after_row() {
        let t = |x: f64, y: f64| translation(x, y);
        assert_eq!(
            grid_array_transforms(2.0, 3.0, 12.5, -4.0),
            vec![
                t(12.5, 0.0),
                t(25.0, 0.0),
                t(0.0, -4.0),
                t(12.5, -4.0),
                t(25.0, -4.0)
            ]
        );
        assert_eq!(
            grid_array_transforms(3.0, 1.0, 7.0, 2.0),
            vec![t(0.0, 2.0), t(0.0, 4.0)]
        );
        assert!(grid_array_transforms(1.0, 1.0, 1.0, 1.0).is_empty());
        assert!(grid_array_transforms(0.0, 5.0, 1.0, 1.0).is_empty());
        assert!(grid_array_transforms(101.0, 100.0, 1.0, 1.0).is_empty());
        assert_eq!(grid_array_transforms(100.0, 100.0, 1.0, 1.0).len(), 9_999);
    }

    /// The middle of a line and a circle's box; text measured in the face
    /// it is drawn in; nothing to measure is none.
    #[test]
    fn the_middle_is_the_middle_of_the_box() {
        let line = Shape::Line {
            a: Vec2::new(487000.0, 4420000.0),
            b: Vec2::new(487010.0, 4420000.0),
        };
        let circle = Shape::Circle {
            c: Vec2::new(487020.0, 4420010.0),
            r: 2.5,
        };
        assert_eq!(
            shapes_middle(&[line.clone(), circle], Font::DEFAULT),
            Some(Vec2::new(487011.25, 4420006.25))
        );
        let text = Shape::Text {
            p: Vec2::new(0.0, 0.0),
            text: "Ada 101".into(),
            height: 2.0,
            rotation: 0.0,
        };
        let wide = shapes_middle(std::slice::from_ref(&text), Font::from_id("courier-prime"));
        let narrow = shapes_middle(std::slice::from_ref(&text), Font::DEFAULT);
        assert_ne!(wide, narrow, "the face measures the text");
        assert_eq!(shapes_middle(&[], Font::DEFAULT), None);
    }

    /// An array's affines by kind: the grid's, the polar array's (turning,
    /// or placed by the middle); counts outside their ranges, another kind
    /// or count of numbers, and a polar array with nothing to place by are none.
    #[test]
    fn an_array_is_its_kind_of_transforms() {
        let line = Shape::Line {
            a: Vec2::new(487000.0, 4420000.0),
            b: Vec2::new(487010.0, 4420000.0),
        };
        let f = Font::DEFAULT;
        assert_eq!(
            array_transforms("grid", &[2.0, 3.0, 12.5, -4.0], &[], f),
            Some(grid_array_transforms(2.0, 3.0, 12.5, -4.0))
        );
        let c = Vec2::new(487005.0, 4420010.0);
        assert_eq!(
            array_transforms("polar", &[c.x, c.y, 4.0, 360.0, 1.0], &[], f),
            Some(polar_array_transforms(c, 4.0, 360.0, true, c))
        );
        assert_eq!(
            array_transforms(
                "polar",
                &[c.x, c.y, 3.0, 90.0, 0.0],
                std::slice::from_ref(&line),
                f
            ),
            Some(polar_array_transforms(
                c,
                3.0,
                90.0,
                false,
                Vec2::new(487005.0, 4420000.0)
            ))
        );
        for (kind, p) in [
            ("grid", vec![1.0, 1.0, 1.0, 1.0]),
            ("grid", vec![2.5, 2.0, 1.0, 1.0]),
            ("grid", vec![0.0, 3.0, 1.0, 1.0]),
            ("grid", vec![10_001.0, 1.0, 1.0, 1.0]),
            ("grid", vec![2.0, 2.0, 1.0]),
            ("polar", vec![c.x, c.y, 1.0, 360.0, 1.0]),
            ("polar", vec![c.x, c.y, 1_001.0, 360.0, 1.0]),
            ("polar", vec![c.x, c.y, 3.5, 360.0, 1.0]),
            ("polar", vec![c.x, c.y, 3.0, 90.0, 0.0]),
            ("spiral", vec![1.0, 2.0, 3.0, 4.0]),
        ] {
            assert_eq!(array_transforms(kind, &p, &[], f), None, "{kind} {p:?}");
        }
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
