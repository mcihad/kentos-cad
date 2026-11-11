//! A railing's plan (docs/adr/8045 §3): its top rail (küpeşte) as two thin lines its width apart, its line between
//! them, as a balcony's railing (docs/adr/0722 §4: "a railing's two lines `width` apart"); closed round a gallery's
//! void, two rings. Beyond 1:150 schematic (1:200) its line alone, as a slab's edge draws its outer line alone there
//! (docs/adr/8011).

use crate::Vec2;
use crate::block::{Piece, PieceStroke};
use crate::entity::Shape;
use crate::geom::arch::Level;
use crate::geom::arrangement::Ring;

/// A railing's fields beside its line, as its plan reads them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RailingSpec {
    /// Its top rail's width, metres.
    pub width: f64,
    /// Its line runs on from its last point back to its first.
    pub closed: bool,
}

/// Its top rail's two faces, `width / 2` to the left of its line and to the right, mitred at its turns; none for a
/// line too short (fewer than two points, three when closed).
pub fn faces(pts: &[Vec2], spec: &RailingSpec) -> Option<(Vec<Vec2>, Vec<Vec2>)> {
    if pts.len() < if spec.closed { 3 } else { 2 } {
        return None;
    }
    let half = spec.width / 2.0;
    let left = crate::geom::offset::offset_path(pts, half, spec.closed);
    let right = crate::geom::offset::offset_path(pts, -half, spec.closed);
    (left.len() >= 2 && right.len() >= 2).then_some((left, right))
}

/// Its top rail as one shape: an open railing's band (its left face from its start, then its right face back), a
/// closed one's ring with its inner face as a hole.
pub fn outline(pts: &[Vec2], spec: &RailingSpec) -> Shape {
    let Some((left, mut right)) = faces(pts, spec) else {
        return Shape::Polygon {
            pts: Vec::new(),
            bulges: None,
            holes: None,
            parts: None,
        };
    };
    if spec.closed {
        // The outer face first, the inner one its hole: which side is outer, the larger ring says.
        let area = |r: &[Vec2]| -> f64 {
            (0..r.len())
                .map(|i| {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    a.x * b.y - b.x * a.y
                })
                .sum::<f64>()
                .abs()
        };
        let (outer, inner) = if area(&left) >= area(&right) {
            (left, right)
        } else {
            (right, left)
        };
        return Shape::Polygon {
            pts: outer,
            bulges: None,
            holes: Some(vec![Ring {
                pts: inner,
                bulges: None,
            }]),
            parts: None,
        };
    }
    right.reverse();
    let mut band = left;
    band.extend(right);
    Shape::Polygon {
        pts: band,
        bulges: None,
        holes: None,
        parts: None,
    }
}

fn piece(shape: Shape) -> Piece {
    Piece {
        shape,
        color: None,
        line_weight: None,
        by_layer: Vec::new(),
        gates: Vec::new(),
        stroke: PieceStroke::default(),
        attribute: None,
        annotation_origin: None,
    }
}

fn line(pts: Vec<Vec2>, closed: bool) -> Shape {
    Shape::Polyline {
        pts,
        bulges: None,
        holes: None,
        parts: None,
        closed,
        widths: None,
    }
}

/// Its pieces at `level`: its top rail's two faces, the line's ends closed for an open railing; beyond 1:150 schematic
/// ([`Level::Simple`]) its line alone.
pub(crate) fn pieces_at(pts: &[Vec2], spec: &RailingSpec, level: Level) -> Vec<Piece> {
    if level == Level::Simple {
        return if pts.len() >= 2 {
            vec![piece(line(pts.to_vec(), spec.closed))]
        } else {
            Vec::new()
        };
    }
    let Some((left, right)) = faces(pts, spec) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if !spec.closed
        && let (Some(&l0), Some(&r0), Some(&l1), Some(&r1)) =
            (left.first(), right.first(), left.last(), right.last())
    {
        out.push(piece(Shape::Line { a: l0, b: r0 }));
        out.push(piece(Shape::Line { a: l1, b: r1 }));
    }
    out.push(piece(line(left, spec.closed)));
    out.push(piece(line(right, spec.closed)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(closed: bool) -> RailingSpec {
        RailingSpec {
            width: 0.05,
            closed,
        }
    }

    #[test]
    fn a_railing_s_top_rail_is_two_lines_its_width_apart() {
        let l = [
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 3.0),
        ];
        let (left, right) = faces(&l, &spec(false)).expect("its faces");
        assert_eq!(
            left,
            vec![
                Vec2::new(0.0, 0.025),
                Vec2::new(3.975, 0.025),
                Vec2::new(3.975, 3.0)
            ]
        );
        assert_eq!(
            right,
            vec![
                Vec2::new(0.0, -0.025),
                Vec2::new(4.025, -0.025),
                Vec2::new(4.025, 3.0)
            ]
        );
        // Open: its two ends closed and its two faces; at 1:200 its line alone.
        assert_eq!(pieces_at(&l, &spec(false), Level::Construction).len(), 4);
        assert_eq!(pieces_at(&l, &spec(false), Level::Simple).len(), 1);
        // Closed round a 4 × 3 m void: two rings, the outer one first in its outline.
        let void = [
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 3.0),
            Vec2::new(0.0, 3.0),
        ];
        assert_eq!(pieces_at(&void, &spec(true), Level::Construction).len(), 2);
        let Shape::Polygon { pts, holes, .. } = outline(&void, &spec(true)) else {
            panic!("a polygon");
        };
        let near = |r: &[Vec2], x: f64, y: f64| {
            r.iter()
                .any(|q| (q.x - x).abs() < 1e-12 && (q.y - y).abs() < 1e-12)
        };
        assert!(near(&pts, -0.025, -0.025), "{pts:?}");
        assert!(near(&holes.expect("its hole")[0].pts, 0.025, 0.025));
        assert!(faces(&l[..1], &spec(false)).is_none());
    }
}
