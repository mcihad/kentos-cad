//! Digitizing locks (docs/adr/0166): the next point's length and direction
//! kept by the user, the direction of an angle typed in the project's way,
//! the deflection from the previous edge, and the corner that closes a
//! right-angled shape square. The tools of both apps hold the locks and pick
//! the edges; the arithmetic is here, with an independent reference in
//! `scripts/fixtures/lock_cases.py` (`fixtures/locks/v1/cases.json`).

use crate::api::Op;
use crate::geom::intersect::{Edge, on_edge_arc, point_at};
use crate::jsmath::{PI, atan2, cos, js_hypot, sin};
use crate::op;
use crate::tools::point_input::{AngleFrom, Angles, Constrained, constrain_cursor};
use crate::tools::point_text::{is_js_space, js_trim};
use crate::vec2::Vec2;

/// Two points closer than this give no direction (the core's `toward_point`).
const SAME: f64 = 1e-9;

/// A locked direction: its unit vector, and whether the point may lie the
/// other way along the line too (Paralel, Dik, the right angle: the cursor
/// picks the side) or only this way (Açı, Sapma: the angle names the way).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Direction {
    pub u: Vec2,
    pub both: bool,
}

/// What is locked for the next point: a length from the reference, a direction, or both.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Locks {
    pub length: Option<f64>,
    pub direction: Option<Direction>,
}

/// The locked point from the reference `o` with the cursor (or the snapped
/// or tracked point) at `c` (docs/adr/0166 §2):
///
/// - a direction and a length: `o + s·L·u`, `s` 1 one way, the side of `c` both ways (1 when `c` is square to it);
/// - a direction alone: `c` projected on the line, never behind `o` one way;
/// - a length alone: that far from `o` towards `c`; none when `c` is on `o`;
/// - nothing locked: `c`.
pub fn lock_point(
    o: Vec2,
    c: Vec2,
    length: Option<f64>,
    direction: Option<Direction>,
) -> Option<Vec2> {
    let dx = c.x - o.x;
    let dy = c.y - o.y;
    match (direction, length) {
        (Some(d), Some(l)) => {
            let side = if d.both && dx * d.u.x + dy * d.u.y < 0.0 {
                -1.0
            } else {
                1.0
            };
            Some(Vec2::new(o.x + side * l * d.u.x, o.y + side * l * d.u.y))
        }
        (Some(d), None) => {
            let t = dx * d.u.x + dy * d.u.y;
            let t = if !d.both && t < 0.0 { 0.0 } else { t };
            Some(Vec2::new(o.x + t * d.u.x, o.y + t * d.u.y))
        }
        (None, Some(l)) => {
            let n = js_hypot(dx, dy);
            if n < SAME {
                return None;
            }
            Some(Vec2::new(o.x + (dx / n) * l, o.y + (dy / n) * l))
        }
        (None, None) => Some(c),
    }
}

/// The effective cursor for the next point from `from` with the locks (§2):
/// a locked direction leaves ortho and polar tracking out and projects the
/// cursor (a snapped or tracked point too); a length alone comes after them,
/// along the way they chose. Without a reference point nothing is locked.
/// None when a length alone has no way to go (the cursor on the reference).
pub fn constrain_locked(
    from: Option<Vec2>,
    world: Vec2,
    exact: bool,
    ortho: bool,
    polar_step: Option<f64>,
    tol: f64,
    locks: Locks,
) -> Option<Constrained> {
    let Some(o) = from else {
        return Some(Constrained {
            point: world,
            tracking: None,
        });
    };
    if locks.direction.is_some() {
        let point = lock_point(o, world, locks.length, locks.direction)?;
        return Some(Constrained {
            point,
            tracking: None,
        });
    }
    let free = constrain_cursor(Some(o), world, exact, ortho, polar_step, tol);
    let point = lock_point(o, free.point, locks.length, None)?;
    Some(Constrained {
        point,
        tracking: free.tracking,
    })
}

/// An angle in the project's unit, in radians.
fn radians(angle: f64, angles: Angles) -> f64 {
    (angle * PI) / if angles.grads { 200.0 } else { 180.0 }
}

/// The unit direction of a typed angle in a project's way (ADR 0165 §4): a
/// CAD project's from east counter-clockwise, a CBS project's semt from
/// north clockwise; in degrees or grads.
pub fn direction_of(angle: f64, angles: Angles) -> Vec2 {
    let a = radians(angle, angles);
    match angles.from {
        AngleFrom::East => Vec2::new(cos(a), sin(a)),
        AngleFrom::North => Vec2::new(sin(a), cos(a)),
    }
}

/// Sapma: the unit direction turned by `angle` from the previous edge's
/// (`prev` to `from`), the way the project's angles run: counter-clockwise
/// (to the left) in a CAD project, clockwise (to the right) in a CBS
/// project. None when the previous edge has no length.
pub fn deflected(prev: Vec2, from: Vec2, angle: f64, angles: Angles) -> Option<Vec2> {
    let dx = from.x - prev.x;
    let dy = from.y - prev.y;
    let n = js_hypot(dx, dy);
    if n < SAME {
        return None;
    }
    let (ux, uy) = (dx / n, dy / n);
    let a = radians(angle, angles);
    let (c, s) = (cos(a), sin(a));
    Some(match angles.from {
        AngleFrom::East => Vec2::new(ux * c - uy * s, ux * s + uy * c),
        AngleFrom::North => Vec2::new(ux * c + uy * s, uy * c - ux * s),
    })
}

/// A direction turned a quarter counter-clockwise: Dik's from an edge's.
pub fn perpendicular(u: Vec2) -> Vec2 {
    Vec2::new(-u.y, u.x)
}

/// The unit direction from `a` to `b`; none when they coincide.
pub fn unit(a: Vec2, b: Vec2) -> Option<Vec2> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let n = js_hypot(dx, dy);
    (n >= SAME).then(|| Vec2::new(dx / n, dy / n))
}

/// Dik kapat (§4): the corner that closes a right-angled shape, where the
/// line through `last` square to the last edge (`prev` to `last`) meets the
/// line through `first` square to the first edge (`first` to `second`).
/// None when an edge has no length or the two lines run parallel (the first
/// and the last edge do).
pub fn square_corner(first: Vec2, second: Vec2, prev: Vec2, last: Vec2) -> Option<Vec2> {
    // Each line runs square to its edge: along (−ey, ex).
    let (ax, ay) = (last.x - prev.x, last.y - prev.y);
    let (bx, by) = (second.x - first.x, second.y - first.y);
    let (pa, pb) = (Vec2::new(-ay, ax), Vec2::new(-by, bx));
    let la = js_hypot(ax, ay);
    let lb = js_hypot(bx, by);
    if la < SAME || lb < SAME {
        return None;
    }
    let cross = pa.x * pb.y - pa.y * pb.x;
    // Parallel within a hair of the edges' own sizes: no corner.
    if cross.abs() <= 1e-12 * la * lb {
        return None;
    }
    // last + s·pa = first + t·pb, solved for s.
    let (wx, wy) = (first.x - last.x, first.y - last.y);
    let s = (wx * pb.y - wy * pb.x) / cross;
    Some(Vec2::new(last.x + s * pa.x, last.y + s * pa.y))
}

/// The direction a lock takes from a picked edge (§3, Nesneye paralel ve
/// dik): a straight edge's own way, `a` to `b`; an arc's tangent at its
/// point nearest `p` (`p`'s own angle from the centre when that lies on the
/// arc, else the nearer end's), the way it sweeps: a full circle's counter-
/// clockwise. Unit; none for an edge of no length or an arc of no radius.
/// The tangent comes from the angle, not from the nearest point less the
/// centre, which would lose the radius's digits to TM coordinates.
pub fn edge_direction(edge: &Edge, p: Vec2) -> Option<Vec2> {
    match *edge {
        Edge::Seg { a, b } => unit(a, b),
        Edge::Arc { c, r, a0, sweep } => {
            if r <= 0.0 {
                return None;
            }
            let ang = atan2(p.y - c.y, p.x - c.x);
            let theta = if on_edge_arc(a0, sweep, ang) {
                ang
            } else {
                let start = point_at(edge, 0.0);
                let end = point_at(edge, 1.0);
                if js_hypot(p.x - start.x, p.y - start.y) <= js_hypot(p.x - end.x, p.y - end.y) {
                    a0
                } else {
                    a0 + sweep
                }
            };
            let k = if sweep < 0.0 { -1.0 } else { 1.0 };
            Some(Vec2::new(-k * sin(theta), k * cos(theta)))
        }
    }
}

/// What typed lock text says (§6): `<45` locks the direction at that angle,
/// in the project's way and unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LockText {
    Angle(f64),
}

/// Reads lock text: `^\s*<\s*NUM\s*$` with the point grammar's number
/// (`tools::point_text`); none for anything else (`@10<45`, a bare `<`).
pub fn parse_lock_text(text: &str) -> Option<LockText> {
    let rest = js_trim(text).strip_prefix('<')?;
    let rest = rest.trim_start_matches(is_js_space);
    crate::tools::point_text::parse_plain_number(rest).map(LockText::Angle)
}

pub(crate) static OPS: &[Op] = &[
    op!("lockPoint", |o: Vec2,
                      c: Vec2,
                      length: Option<f64>,
                      u: Option<Vec2>,
                      both: bool| {
        lock_point(o, c, length, u.map(|u| Direction { u, both }))
    }),
    op!("constrainLocked", |from: Option<Vec2>,
                            world: Vec2,
                            exact: bool,
                            ortho: bool,
                            polar_step: Option<f64>,
                            tol: f64,
                            length: Option<f64>,
                            u: Option<Vec2>,
                            both: bool| {
        constrain_locked(
            from,
            world,
            exact,
            ortho,
            polar_step,
            tol,
            Locks {
                length,
                direction: u.map(|u| Direction { u, both }),
            },
        )
    }),
    op!("lockDirection", |angle: f64,
                          from_north: bool,
                          grads: bool| {
        direction_of(angle, angles(from_north, grads))
    }),
    op!("lockDeflected", |prev: Vec2,
                          from: Vec2,
                          angle: f64,
                          from_north: bool,
                          grads: bool| {
        deflected(prev, from, angle, angles(from_north, grads))
    }),
    op!("squareCorner", |first: Vec2,
                         second: Vec2,
                         prev: Vec2,
                         last: Vec2| {
        square_corner(first, second, prev, last)
    }),
    op!("lockEdgeDirection", |e: Edge, p: Vec2| {
        edge_direction(&e, p)
    }),
];

fn angles(from_north: bool, grads: bool) -> Angles {
    Angles {
        from: if from_north {
            AngleFrom::North
        } else {
            AngleFrom::East
        },
        grads,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const O: Vec2 = Vec2::new(0.0, 0.0);

    #[test]
    fn a_length_and_a_one_way_direction_ignore_the_cursor() {
        let d = Direction {
            u: Vec2::new(0.0, 1.0),
            both: false,
        };
        let p = lock_point(O, Vec2::new(3.0, -9.0), Some(5.0), Some(d));
        assert_eq!(p, Some(Vec2::new(0.0, 5.0)));
        // A direction alone: never behind the reference one way, the other side both ways.
        assert_eq!(lock_point(O, Vec2::new(2.0, -4.0), None, Some(d)), Some(O));
        let both = Direction { both: true, ..d };
        assert_eq!(
            lock_point(O, Vec2::new(2.0, -4.0), None, Some(both)),
            Some(Vec2::new(0.0, -4.0))
        );
        assert_eq!(
            lock_point(O, Vec2::new(2.0, -4.0), Some(5.0), Some(both)),
            Some(Vec2::new(0.0, -5.0))
        );
    }

    #[test]
    fn a_length_alone_needs_a_way() {
        assert_eq!(lock_point(O, O, Some(5.0), None), None);
        assert_eq!(
            lock_point(O, Vec2::new(3.0, 4.0), Some(10.0), None),
            Some(Vec2::new(6.0, 8.0))
        );
    }

    #[test]
    fn a_locked_direction_leaves_ortho_out_and_a_length_follows_it() {
        let locks = Locks {
            length: Some(4.0),
            direction: None,
        };
        let c = constrain_locked(Some(O), Vec2::new(10.0, 2.0), false, true, None, 1.0, locks);
        assert_eq!(c.map(|c| c.point), Some(Vec2::new(4.0, 0.0)));
        let locks = Locks {
            length: None,
            direction: Some(Direction {
                u: Vec2::new(0.0, 1.0),
                both: false,
            }),
        };
        let c = constrain_locked(Some(O), Vec2::new(10.0, 2.0), false, true, None, 1.0, locks);
        assert_eq!(c.map(|c| c.point), Some(Vec2::new(0.0, 2.0)));
    }

    #[test]
    fn deflection_turns_left_in_cad_and_right_in_cbs() {
        let east = Vec2::new(10.0, 0.0);
        let cad = deflected(O, east, 90.0, angles(false, false)).expect("a direction");
        assert!(
            (cad.x).abs() < 1e-15 && (cad.y - 1.0).abs() < 1e-15,
            "{cad:?}"
        );
        let cbs = deflected(O, east, 100.0, angles(true, true)).expect("a direction");
        assert!(
            (cbs.x).abs() < 1e-15 && (cbs.y + 1.0).abs() < 1e-15,
            "{cbs:?}"
        );
        assert_eq!(deflected(O, O, 90.0, angles(false, false)), None);
    }

    #[test]
    fn a_square_corner_closes_the_shape() {
        let c = square_corner(
            O,
            Vec2::new(8.0, 6.0),
            Vec2::new(8.0, 6.0),
            Vec2::new(5.0, 10.0),
        );
        assert_eq!(c, Some(Vec2::new(-3.0, 4.0)));
        let parallel = square_corner(
            O,
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(30.0, 0.0),
        );
        assert_eq!(parallel, None);
    }

    #[test]
    fn lock_text_is_an_angle_after_a_less_than() {
        assert_eq!(parse_lock_text("<45"), Some(LockText::Angle(45.0)));
        assert_eq!(parse_lock_text(" < 12.5 "), Some(LockText::Angle(12.5)));
        assert_eq!(parse_lock_text("<-30"), Some(LockText::Angle(-30.0)));
        for bad in ["<", "45", "@10<45", "10<45", "<4,5", "<<45", "<45x"] {
            assert_eq!(parse_lock_text(bad), None, "{bad:?}");
        }
    }
}
