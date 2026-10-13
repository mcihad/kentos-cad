//! The 2D user coordinate system (docs/adr/0261): a frame in the world, an
//! origin and the angle of its x axis, counter-clockwise from east. Points
//! are kept in the world; the frame only says how they are typed, shown and
//! constrained. Here: the two maps, the frame an object gives (§3), typed
//! points read in a frame and the cursor rules turned with it (§4).

use crate::entity::Shape;
use crate::geom::bulge::bulge_path_edges;
use crate::geom::intersect::closest_on_edge;
use crate::jsmath::{PI, atan, atan2, cos, js_hypot, sin};
use crate::tools::point_input::{Angles, Constrained, Tracking, constrain_cursor};
use crate::vec2::Vec2;

/// A frame: origin in the world, x axis's angle from east, counter-clockwise, radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub origin: Vec2,
    pub angle: f64,
    /// cos and sin of the angle, once.
    c: f64,
    s: f64,
}

impl Frame {
    pub const WORLD: Frame = Frame {
        origin: Vec2 { x: 0.0, y: 0.0 },
        angle: 0.0,
        c: 1.0,
        s: 0.0,
    };

    pub fn new(origin: Vec2, angle: f64) -> Frame {
        if angle == 0.0 {
            return Frame {
                origin,
                angle,
                c: 1.0,
                s: 0.0,
            };
        }
        Frame {
            origin,
            angle,
            c: cos(angle),
            s: sin(angle),
        }
    }

    pub fn is_world(&self) -> bool {
        self.origin.x == 0.0 && self.origin.y == 0.0 && self.angle == 0.0
    }

    /// A direction of the frame in the world.
    pub fn dir_to_world(&self, v: Vec2) -> Vec2 {
        Vec2::new(v.x * self.c - v.y * self.s, v.x * self.s + v.y * self.c)
    }

    /// A world direction in the frame.
    pub fn dir_to_frame(&self, v: Vec2) -> Vec2 {
        Vec2::new(v.x * self.c + v.y * self.s, -v.x * self.s + v.y * self.c)
    }

    /// A point of the frame in the world: origin + R(angle)·p.
    pub fn to_world(&self, p: Vec2) -> Vec2 {
        if self.is_world() {
            return p;
        }
        let d = self.dir_to_world(p);
        Vec2::new(self.origin.x + d.x, self.origin.y + d.y)
    }

    /// A world point in the frame: R(−angle)·(w − origin).
    pub fn to_frame(&self, w: Vec2) -> Vec2 {
        if self.is_world() {
            return w;
        }
        self.dir_to_frame(Vec2::new(w.x - self.origin.x, w.y - self.origin.y))
    }
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(b.x - a.x, b.y - a.y)
}

/// A path of a shape: its points, bulges and whether it is closed.
type Path<'a> = (&'a [Vec2], Option<&'a [f64]>, bool);

/// The paths of a shape, each (points, bulges, closed), in the ADR's order:
/// an area's outer ring, its holes, then each further part's ring and holes;
/// a polyline and its parts; a hatch's ring and holes.
fn rings(shape: &Shape) -> Vec<Path<'_>> {
    let mut out = Vec::new();
    match shape {
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            out.push((pts.as_slice(), bulges.as_deref(), true));
            for h in holes.iter().flatten() {
                out.push((h.pts.as_slice(), h.bulges.as_deref(), true));
            }
            for part in parts.iter().flatten() {
                out.push((part.pts.as_slice(), part.bulges.as_deref(), true));
                for h in part.holes.iter().flatten() {
                    out.push((h.pts.as_slice(), h.bulges.as_deref(), true));
                }
            }
        }
        Shape::Polyline {
            pts,
            bulges,
            parts,
            closed,
            ..
        } => {
            out.push((pts.as_slice(), bulges.as_deref(), *closed));
            for part in parts.iter().flatten() {
                out.push((part.pts.as_slice(), part.bulges.as_deref(), false));
            }
        }
        Shape::Hatch { ring, holes, .. } => {
            out.push((ring.as_slice(), None, true));
            for h in holes.iter().flatten() {
                out.push((h.as_slice(), None, true));
            }
        }
        _ => {}
    }
    out
}

/// The nearest edge's near end and the x axis along it (docs/adr/0261 §3):
/// on a bulged edge the arc's tangent at that end, into the arc.
fn edge_frame(shape: &Shape, at: Vec2) -> Option<(Vec2, f64)> {
    let mut best: Option<(f64, Vec2, Vec2, f64)> = None;
    for (pts, bulges, closed) in rings(shape) {
        let n = pts.len();
        for (i, edge) in bulge_path_edges(pts, bulges, closed).iter().enumerate() {
            let d = closest_on_edge(edge, at).d;
            if best.is_none_or(|(bd, ..)| d < bd) {
                let bulge = bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0);
                best = Some((d, pts[i], pts[(i + 1) % n], bulge));
            }
        }
    }
    let (_, a, b, bulge) = best?;
    let chord = atan2(b.y - a.y, b.x - a.x);
    let half = if bulge == 0.0 { 0.0 } else { 2.0 * atan(bulge) };
    Some(if dist(at, a) <= dist(at, b) {
        (a, chord - half)
    } else {
        (b, chord + PI + half)
    })
}

/// The frame an object gives at the place clicked, its angle not yet
/// brought into [0, 2π) (docs/adr/0261 §3); none for a kind without an axis.
/// A point keeps `current`'s angle.
pub fn object_frame(shape: &Shape, at: Vec2, current: f64) -> Option<(Vec2, f64)> {
    match shape {
        Shape::Line { a, b } => Some(if dist(at, *a) <= dist(at, *b) {
            (*a, atan2(b.y - a.y, b.x - a.x))
        } else {
            (*b, atan2(a.y - b.y, a.x - b.x))
        }),
        Shape::Polyline { .. } | Shape::Polygon { .. } | Shape::Hatch { .. } => {
            edge_frame(shape, at)
        }
        Shape::Arc { c, r, a0, a1 } => {
            let end = |a: f64| Vec2::new(c.x + r * cos(a), c.y + r * sin(a));
            Some((
                *c,
                if dist(at, end(*a0)) <= dist(at, end(*a1)) {
                    *a0
                } else {
                    *a1
                },
            ))
        }
        Shape::Circle { c, .. } => Some((*c, atan2(at.y - c.y, at.x - c.x))),
        Shape::Ellipse { c, major, .. } => Some((*c, atan2(major.y, major.x))),
        Shape::Text { p, rotation, .. } => Some((*p, (rotation * PI) / 180.0)),
        Shape::Insert { p, rotation, .. } => Some((*p, *rotation)),
        Shape::Point { p, .. } => Some((*p, current)),
        _ => None,
    }
}

/// Typed point text read in `frame` (docs/adr/0261 §4): an absolute point is
/// the frame's, a relative one goes along its axes, a polar angle is from
/// its axes; a distance along the tracking line or toward the cursor is the
/// same in any frame. Points in and out are the world's.
pub fn point_from_text_framed(
    text: &str,
    last: Option<Vec2>,
    cursor: Option<Vec2>,
    along: impl Fn(f64) -> Option<Vec2>,
    to_metres: impl Fn(f64) -> f64,
    angles: Angles,
    frame: &Frame,
) -> Option<Vec2> {
    use crate::tools::point_text::point_from_text_in;
    if frame.is_world() {
        return point_from_text_in(text, last, cursor, along, to_metres, angles);
    }
    point_from_text_in(
        text,
        last.map(|p| frame.to_frame(p)),
        cursor.map(|p| frame.to_frame(p)),
        |d| along(d).map(|p| frame.to_frame(p)),
        to_metres,
        angles,
    )
    .map(|p| frame.to_world(p))
}

/// The cursor rule (ortho, polar tracking) with the frame's axes: the
/// pointer turned into the frame about `from`, the rule, the result back.
pub fn constrain_cursor_in(
    frame: &Frame,
    from: Option<Vec2>,
    world: Vec2,
    exact: bool,
    ortho: bool,
    polar_step: Option<f64>,
    tol: f64,
) -> Constrained {
    if frame.angle == 0.0 {
        return constrain_cursor(from, world, exact, ortho, polar_step, tol);
    }
    let Some(f) = from else {
        return constrain_cursor(from, world, exact, ortho, polar_step, tol);
    };
    let turned = |p: Vec2| {
        let d = frame.dir_to_frame(Vec2::new(p.x - f.x, p.y - f.y));
        Vec2::new(f.x + d.x, f.y + d.y)
    };
    let back = |p: Vec2| {
        let d = frame.dir_to_world(Vec2::new(p.x - f.x, p.y - f.y));
        Vec2::new(f.x + d.x, f.y + d.y)
    };
    let c = constrain_cursor(Some(f), turned(world), exact, ortho, polar_step, tol);
    Constrained {
        point: if exact { world } else { back(c.point) },
        // The ray's angle, degrees from east, turned with the frame; it starts at `from` still.
        tracking: c.tracking.map(|t| Tracking {
            origin: back(t.origin),
            angle: t.angle + (frame.angle * 180.0) / PI,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: Vec2, b: Vec2) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    }

    #[test]
    fn the_two_maps_undo_each_other_and_the_world_is_no_map() {
        let f = Frame::new(Vec2::new(100.0, 200.0), PI / 2.0);
        assert!(near(
            f.to_world(Vec2::new(10.0, 0.0)),
            Vec2::new(100.0, 210.0)
        ));
        assert!(near(
            f.to_frame(Vec2::new(100.0, 210.0)),
            Vec2::new(10.0, 0.0)
        ));
        let p = Vec2::new(487012.345, 4420123.456);
        assert_eq!(Frame::WORLD.to_world(p), p);
        assert_eq!(Frame::WORLD.to_frame(p), p);
    }

    #[test]
    fn ortho_follows_the_frames_axes() {
        let f = Frame::new(Vec2::new(0.0, 0.0), PI / 4.0);
        let from = Vec2::new(10.0, 10.0);
        // Nearly along the frame's x axis (45°): held on it.
        let c = constrain_cursor_in(
            &f,
            Some(from),
            Vec2::new(17.0, 17.5),
            false,
            true,
            None,
            0.1,
        );
        let d = Vec2::new(c.point.x - from.x, c.point.y - from.y);
        assert!((d.x - d.y).abs() < 1e-9, "{d:?}");
        // A snapped point stays where it is.
        let c = constrain_cursor_in(&f, Some(from), Vec2::new(17.0, 17.5), true, true, None, 0.1);
        assert_eq!(c.point, Vec2::new(17.0, 17.5));
    }

    #[test]
    fn an_object_gives_its_frame() {
        let line = Shape::Line {
            a: Vec2::new(0.0, 0.0),
            b: Vec2::new(0.0, 10.0),
        };
        assert_eq!(
            object_frame(&line, Vec2::new(0.0, 9.0), 0.0),
            Some((Vec2::new(0.0, 10.0), atan2(-10.0, 0.0)))
        );
        let point = Shape::Point {
            p: Vec2::new(1.0, 2.0),
            z: None,
            parts: None,
        };
        assert_eq!(
            object_frame(&point, Vec2::new(0.0, 0.0), 0.5),
            Some((Vec2::new(1.0, 2.0), 0.5))
        );
    }
}
