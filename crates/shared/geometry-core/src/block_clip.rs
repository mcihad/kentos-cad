//! An insert's clipping (docs/adr/0351 §1; AutoCAD's XCLIP): the insert's pieces, placed in the drawing, cut by its
//! boundary. A filled area is cut as an area; a line, an arc, a polyline and every outline (a circle, an ellipse, a
//! closed curve: drawn as their boundaries) as paths, cut where they cross the boundary and kept by where their pieces'
//! middles stand; a text, a point, a hatch or an image whole or not at all, by the middle of its box. With `inverted`
//! the outside is kept. A plotted frame is one more piece, a closed polyline along the boundary.

use crate::block::Piece;
use crate::entity::{Clipping, Shape, entity_bounds};
use crate::geom::arrangement::{Area, WindingIndex};
use crate::geometry::Bounds;
use crate::ops::geoprocess::clip::within;
use crate::ops::geoprocess::{Class, Place, Region, boundary_edges, chain, class_of, shape_of};
use crate::geom::affine::{Affine, apply};
use crate::vec2::Vec2;

/// The boundary in the drawing: its corners in the definition's coordinates through the insert's placement (`m`
/// takes base-relative points, so the base point comes off first).
pub fn placed_ring(clip: &Clipping, base: Vec2, m: &Affine) -> Vec<Vec2> {
    clip.ring
        .iter()
        .map(|p| apply(m, Vec2::new(p.x - base.x, p.y - base.y)))
        .collect()
}

fn ring_area(ring: &[Vec2]) -> Vec<Area> {
    let polygon = Shape::Polygon {
        pts: ring.to_vec(),
        bulges: None,
        holes: None,
        parts: None,
    };
    match class_of(&polygon) {
        Class::Areas(a) => a,
        _ => Vec::new(),
    }
}

fn bounds_of(ring: &[Vec2]) -> Bounds {
    let mut b = Bounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    };
    for p in ring {
        b.min_x = crate::jsmath::js_min(b.min_x, p.x);
        b.min_y = crate::jsmath::js_min(b.min_y, p.y);
        b.max_x = crate::jsmath::js_max(b.max_x, p.x);
        b.max_y = crate::jsmath::js_max(b.max_y, p.y);
    }
    b
}

fn apart(a: &Bounds, b: &Bounds) -> bool {
    a.max_x < b.min_x || b.max_x < a.min_x || a.max_y < b.min_y || b.max_y < a.min_y
}

/// How a piece's shape is cut: a filled area as an area, an outline as its boundary paths, a path as itself; none for
/// a shape kept whole or not at all.
fn cut_class(shape: &Shape) -> Option<Class> {
    match (shape, class_of(shape)) {
        (_, Class::None) => None,
        (Shape::Polygon { .. }, c) => Some(c),
        (_, Class::Areas(areas)) => Some(Class::Paths(chain(boundary_edges(&areas)))),
        (_, c) => Some(c),
    }
}

/// `pieces` (in the drawing) cut by `ring` (in the drawing): the inside kept, or the outside when `inverted`; the
/// frame added when it is plotted.
pub fn clipped(pieces: Vec<Piece>, ring: &[Vec2], inverted: bool, plot_frame: bool) -> Vec<Piece> {
    let cut = ring_area(ring);
    if cut.is_empty() {
        return pieces;
    }
    let ring_box = bounds_of(ring);
    let region = Region::of(&cut);
    let index = WindingIndex::new(region.edges());
    let mut out = Vec::with_capacity(pieces.len() + 1);
    for piece in pieces {
        let b = entity_bounds(&piece.shape);
        if !(b.min_x.is_finite() && b.max_x.is_finite()) {
            continue;
        }
        // Wholly away from the boundary: wholly outside.
        if apart(&b, &ring_box) {
            if inverted {
                out.push(piece);
            }
            continue;
        }
        match cut_class(&piece.shape) {
            Some(class) => {
                let kept = within(&class, &cut, !inverted);
                if let Some(shape) = shape_of(&kept) {
                    out.push(Piece { shape, ..piece });
                }
            }
            None => {
                let middle = Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
                let inside = region.place(&index, middle) != Place::Outside;
                if inside != inverted {
                    out.push(piece);
                }
            }
        }
    }
    if plot_frame {
        out.push(Piece {
            shape: Shape::Polyline {
                pts: ring.to_vec(),
                bulges: None,
                holes: None,
                parts: None,
                closed: true,
                widths: None,
            },
            color: None,
            line_weight: None,
            by_layer: Vec::new(),
            gates: Vec::new(),
            stroke: Default::default(),
            attribute: None,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(shape: Shape) -> Piece {
        Piece {
            shape,
            color: None,
            line_weight: None,
            by_layer: Vec::new(),
            gates: Vec::new(),
            stroke: Default::default(),
            attribute: None,
        }
    }

    fn line(a: (f64, f64), b: (f64, f64)) -> Shape {
        Shape::Line {
            a: Vec2::new(a.0, a.1),
            b: Vec2::new(b.0, b.1),
        }
    }

    const SQUARE: [Vec2; 4] = [
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 10.0, y: 0.0 },
        Vec2 { x: 10.0, y: 10.0 },
        Vec2 { x: 0.0, y: 10.0 },
    ];

    #[test]
    fn a_line_across_the_boundary_keeps_its_inside_or_its_outside() {
        let kept = clipped(vec![piece(line((-5.0, 5.0), (15.0, 5.0)))], &SQUARE, false, false);
        let Shape::Polyline { pts, .. } = &kept[0].shape else { panic!("{kept:?}") };
        assert_eq!(pts, &[Vec2::new(0.0, 5.0), Vec2::new(10.0, 5.0)]);
        let outside = clipped(vec![piece(line((-5.0, 5.0), (15.0, 5.0)))], &SQUARE, true, false);
        let Shape::Polyline { pts, parts, .. } = &outside[0].shape else { panic!("{outside:?}") };
        assert_eq!(pts, &[Vec2::new(-5.0, 5.0), Vec2::new(0.0, 5.0)]);
        assert_eq!(parts.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn a_piece_away_from_the_boundary_and_the_frame() {
        let far = clipped(vec![piece(line((20.0, 20.0), (30.0, 20.0)))], &SQUARE, false, true);
        assert_eq!(far.len(), 1, "only the frame");
        assert!(matches!(far[0].shape, Shape::Polyline { closed: true, .. }));
        let kept = clipped(vec![piece(line((20.0, 20.0), (30.0, 20.0)))], &SQUARE, true, false);
        assert_eq!(kept.len(), 1);
    }
}
