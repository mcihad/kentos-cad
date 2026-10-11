//! Where points stand to an object's areas (docs/adr/0220 §3, §4): inside,
//! on the boundary (within the overlay's 1 µm: Kırp's rule, docs/adr/0201
//! §3) or outside; many points against one set, the boundary's edges in a
//! packed tree, the inside from a winding index (exact on straight edges).

use crate::geom::arrangement::{Area, TOL, WindingIndex, tight_box};
use crate::geom::intersect::closest_on_edge;
use crate::geom::region::area_source;
use crate::geometry::{Bounds, empty_bounds};
use crate::jsmath::{js_max, js_min};
use crate::ops::geoprocess::Place;
use crate::store::rtree::PackedTree;
use crate::vec2::Vec2;

pub struct Inside {
    index: WindingIndex<'static>,
    tree: PackedTree,
    /// The areas' box (their arcs' ends and the extremes they pass).
    pub bounds: Bounds,
}

impl Inside {
    /// None when the areas have no boundary.
    pub fn new(areas: &[Area]) -> Option<Inside> {
        let edges = area_source(areas).edges;
        if edges.is_empty() {
            return None;
        }
        let boxes: Vec<(u32, Bounds)> = edges
            .iter()
            .enumerate()
            .map(|(i, e)| (i as u32, tight_box(e)))
            .collect();
        let mut bounds = empty_bounds();
        for (_, b) in &boxes {
            bounds.min_x = js_min(bounds.min_x, b.min_x);
            bounds.min_y = js_min(bounds.min_y, b.min_y);
            bounds.max_x = js_max(bounds.max_x, b.max_x);
            bounds.max_y = js_max(bounds.max_y, b.max_y);
        }
        Some(Inside {
            tree: PackedTree::build(&boxes),
            index: WindingIndex::owned(edges),
            bounds,
        })
    }

    /// Where `p` stands: on the boundary first, so a point there is the boundary's.
    pub fn place(&self, p: Vec2, near: &mut Vec<u32>) -> Place {
        near.clear();
        let q = Bounds {
            min_x: p.x - TOL,
            min_y: p.y - TOL,
            max_x: p.x + TOL,
            max_y: p.y + TOL,
        };
        self.tree.search(&q, near);
        let edges = self.index.edges();
        if near
            .iter()
            .any(|&i| closest_on_edge(&edges[i as usize], p).d <= TOL)
        {
            return Place::On;
        }
        if self.index.winding(p) != 0.0 {
            Place::Inside
        } else {
            Place::Outside
        }
    }

    /// Inside or on the boundary.
    pub fn holds(&self, p: Vec2, near: &mut Vec<u32>) -> bool {
        self.place(p, near) != Place::Outside
    }
}
