//! Areas, lengths and bounds of whole shapes (`entityArea`, `entityLength`
//! and `entityBounds` in `apps/web/src/model/entities.ts`), for the shapes whose
//! definition is exact on both sides. Bulged paths and arcs are bounded by
//! a tessellated outline in TypeScript; their exact bounds come with the
//! Rust port of the outline (docs/adr/0002-contracts-fixtures.md).

use crate::geom::bulge::{bulge_path_length, bulge_ring_area};
use crate::jsmath::{js_max, js_min};
use crate::vec2::Vec2;

/// A ring in vertex + bulge form (a polygon's outer ring or one of its holes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ring {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
}

/// Net area of a polygon with holes ("adalı alan"): the outer ring minus every hole.
pub fn polygon_area(outer: &Ring, holes: &[Ring]) -> f64 {
    bulge_ring_area(&outer.pts, outer.bulges.as_deref()).abs()
        - holes
            .iter()
            .map(|h| bulge_ring_area(&h.pts, h.bulges.as_deref()).abs())
            .sum::<f64>()
}

/// Perimeter of a polygon, holes included (as in GIS).
pub fn polygon_perimeter(outer: &Ring, holes: &[Ring]) -> f64 {
    bulge_path_length(&outer.pts, outer.bulges.as_deref(), true)
        + holes
            .iter()
            .map(|h| bulge_path_length(&h.pts, h.bulges.as_deref(), true))
            .sum::<f64>()
}

pub fn circle_area(r: f64) -> f64 {
    std::f64::consts::PI * r * r
}

pub fn circle_length(r: f64) -> f64 {
    2.0 * std::f64::consts::PI * r
}

/// Axis-aligned bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Bounds {
    pub const EMPTY: Bounds = Bounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    };

    pub fn is_empty(&self) -> bool {
        self.min_x > self.max_x || self.min_y > self.max_y
    }

    /// Grows the box to hold `p`, padded by `pad` on every side.
    pub fn extend(&mut self, p: Vec2, pad: f64) {
        self.min_x = js_min(self.min_x, p.x - pad);
        self.min_y = js_min(self.min_y, p.y - pad);
        self.max_x = js_max(self.max_x, p.x + pad);
        self.max_y = js_max(self.max_y, p.y + pad);
    }
}

/// Bounds of straight vertices (lines, straight polylines and polygons, points).
pub fn vertex_bounds(pts: &[Vec2]) -> Bounds {
    let mut b = Bounds::EMPTY;
    for &p in pts {
        b.extend(p, 0.0);
    }
    b
}

pub fn circle_bounds(c: Vec2, r: f64) -> Bounds {
    let mut b = Bounds::EMPTY;
    b.extend(c, r);
    b
}
