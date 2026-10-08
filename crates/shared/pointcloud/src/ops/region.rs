//! The areas Kırp and Alan sorgusu read points against (docs/adr/0207 §7):
//! areas with holes and parts, arcs exact (the overlay engine's winding
//! with its exact orientation, `geom::arrangement::WindingIndex`). A point
//! on a straight edge is inside; elsewhere the winding decides.

use kentos_geometry_core::geom::arrangement::{Area, WindingIndex};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::region::{net_area, orient_ring, ring_edges};
use kentos_geometry_core::predicates::orient2d;
use kentos_geometry_core::vec2::Vec2;

/// The cells a side of the boundary's grid has, at most.
const GRID: usize = 256;

/// Areas taken together: a point is in the region when it is in any of them.
#[derive(Clone, Debug)]
pub struct Region {
    /// Outer rings counter-clockwise, holes clockwise: a point inside has a winding above 0.
    edges: Vec<Edge>,
    bbox: [f64; 4],
    /// The straight edges by the cells of a grid over the box, for the boundary rule.
    segs: Vec<(Vec2, Vec2)>,
    cells: Vec<Vec<u32>>,
    side: [usize; 2],
    cell: [f64; 2],
    area: f64,
}

impl Region {
    /// The region of `areas`; none when they enclose nothing.
    pub fn new(areas: &[Area]) -> Option<Region> {
        let mut edges = Vec::new();
        let mut area = 0.0;
        for a in areas {
            if a.outer.pts.len() < 2 {
                continue;
            }
            edges.extend(ring_edges(&orient_ring(&a.outer, true)));
            for h in &a.holes {
                edges.extend(ring_edges(&orient_ring(h, false)));
            }
            area += net_area(a);
        }
        if edges.is_empty() {
            return None;
        }
        let mut bbox = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for e in &edges {
            let b = kentos_geometry_core::geom::arrangement::edge_box(e);
            bbox[0] = bbox[0].min(b.min_x);
            bbox[1] = bbox[1].min(b.min_y);
            bbox[2] = bbox[2].max(b.max_x);
            bbox[3] = bbox[3].max(b.max_y);
        }
        if !bbox.iter().all(|v| v.is_finite()) {
            return None;
        }
        let segs: Vec<(Vec2, Vec2)> = edges
            .iter()
            .filter_map(|e| match *e {
                Edge::Seg { a, b } => Some((a, b)),
                Edge::Arc { .. } => None,
            })
            .collect();
        let w = (bbox[2] - bbox[0]).max(1e-9);
        let h = (bbox[3] - bbox[1]).max(1e-9);
        let side = [
            GRID.min(1 + segs.len() / 4).max(1),
            GRID.min(1 + segs.len() / 4).max(1),
        ];
        let cell = [w / side[0] as f64, h / side[1] as f64];
        let mut cells = vec![Vec::new(); side[0] * side[1]];
        for (i, (a, b)) in segs.iter().enumerate() {
            let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
            let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
            let c0 = Self::at(bbox[0], cell[0], side[0], x0);
            let c1 = Self::at(bbox[0], cell[0], side[0], x1);
            let r0 = Self::at(bbox[1], cell[1], side[1], y0);
            let r1 = Self::at(bbox[1], cell[1], side[1], y1);
            for r in r0..=r1 {
                for c in c0..=c1 {
                    cells[r * side[0] + c].push(i as u32);
                }
            }
        }
        Some(Region {
            edges,
            bbox,
            segs,
            cells,
            side,
            cell,
            area,
        })
    }

    #[inline]
    fn at(low: f64, cell: f64, n: usize, v: f64) -> usize {
        let t = ((v - low) / cell).floor();
        if t.is_nan() || t < 0.0 {
            0
        } else {
            (t as usize).min(n - 1)
        }
    }

    /// Its box `[x₁, y₁, x₂, y₂]`.
    pub fn bbox(&self) -> [f64; 4] {
        self.bbox
    }

    /// Its net area (m²): the areas' outer rings less their holes, summed.
    pub fn area(&self) -> f64 {
        self.area
    }

    /// Whether `p` lies on a straight edge, exactly.
    fn on_edge(&self, p: Vec2) -> bool {
        let c = Self::at(self.bbox[0], self.cell[0], self.side[0], p.x);
        let r = Self::at(self.bbox[1], self.cell[1], self.side[1], p.y);
        self.cells[r * self.side[0] + c].iter().any(|&i| {
            let (a, b) = self.segs[i as usize];
            p.x >= a.x.min(b.x)
                && p.x <= a.x.max(b.x)
                && p.y >= a.y.min(b.y)
                && p.y <= a.y.max(b.y)
                && orient2d(a, b, p) == 0.0
        })
    }

    /// Its winding index, for [`Region::contains`]: made once for many points.
    pub fn index(&self) -> WindingIndex<'_> {
        WindingIndex::new(&self.edges)
    }

    /// Whether `p` is in the region (`index` its own).
    #[inline]
    pub fn contains(&self, index: &WindingIndex<'_>, p: Vec2) -> bool {
        let [x1, y1, x2, y2] = self.bbox;
        if !(p.x >= x1 && p.x <= x2 && p.y >= y1 && p.y <= y2) {
            return false;
        }
        index.winding(p) > 0.0 || self.on_edge(p)
    }

    /// Whether each of `pts` is in the region, into `out` (cleared).
    pub fn inside_many(&self, pts: &[[f64; 2]], out: &mut Vec<bool>) {
        out.clear();
        out.reserve(pts.len());
        let index = self.index();
        for &[x, y] in pts {
            out.push(self.contains(&index, Vec2::new(x, y)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::geom::arrangement::Ring;

    fn square(x: f64, y: f64, s: f64) -> Ring {
        Ring {
            pts: vec![
                Vec2::new(x, y),
                Vec2::new(x + s, y),
                Vec2::new(x + s, y + s),
                Vec2::new(x, y + s),
            ],
            bulges: None,
        }
    }

    #[test]
    fn holes_parts_edges_and_arcs() {
        let areas = vec![
            Area {
                outer: square(0.0, 0.0, 10.0),
                holes: vec![square(4.0, 4.0, 2.0)],
            },
            // A half disc on top of the square: an arc of bulge 1 from (10, 10) to (0, 10).
            Area {
                outer: Ring {
                    pts: vec![Vec2::new(0.0, 10.0), Vec2::new(10.0, 10.0)],
                    bulges: Some(vec![0.0, 1.0]),
                },
                holes: vec![],
            },
        ];
        let r = Region::new(&areas).unwrap();
        let mut out = Vec::new();
        r.inside_many(
            &[
                [1.0, 1.0],
                [5.0, 5.0],
                [4.0, 5.0],
                [0.0, 5.0],
                [5.0, 14.9],
                [5.0, 15.1],
                [11.0, 1.0],
            ],
            &mut out,
        );
        assert_eq!(out, vec![true, false, true, true, true, false, false]);
        assert!((r.area() - (96.0 + std::f64::consts::PI * 12.5)).abs() < 1e-9);
    }
}
