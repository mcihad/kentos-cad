//! Many routes asked about many places (docs/adr/0224 §4): every route made
//! ready once and all their edges in one packed R-tree, so a point finds the
//! routes within a radius without walking every route.

use super::{Place, Route, Walk};
use crate::geom::arrangement::tight_box;
use crate::geometry::Bounds;
use crate::store::rtree::PackedTree;
use crate::vec2::Vec2;

/// A route's closest place to a point, the route by its place in the list.
#[derive(Clone, Debug, PartialEq)]
pub struct Near {
    pub route: usize,
    pub place: Place,
}

crate::json_struct!(Near { route, place });

pub struct RouteIndex {
    walks: Vec<Walk>,
    tree: PackedTree,
    /// Each tree item's route and edge (walked order).
    owner: Vec<(u32, u32)>,
}

impl RouteIndex {
    pub fn new(routes: &[Route]) -> RouteIndex {
        let walks: Vec<Walk> = routes.iter().map(Walk::new).collect();
        let mut owner = Vec::new();
        let mut boxes = Vec::new();
        for (r, w) in walks.iter().enumerate() {
            for (k, s) in w.steps.iter().enumerate() {
                let b = tight_box(&s.edge);
                if [b.min_x, b.min_y, b.max_x, b.max_y]
                    .iter()
                    .all(|v| v.is_finite())
                {
                    boxes.push((owner.len() as u32, b));
                    owner.push((r as u32, k as u32));
                }
            }
        }
        RouteIndex {
            walks,
            tree: PackedTree::build(&boxes),
            owner,
        }
    }

    /// The route at `i` made ready.
    pub fn walk(&self, i: usize) -> &Walk {
        &self.walks[i]
    }

    pub fn len(&self) -> usize {
        self.walks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.walks.is_empty()
    }

    /// The routes with an edge whose box comes within `radius` of `b`, in
    /// route order (the candidates of a line or an area).
    pub fn routes_near(&self, b: &Bounds, radius: f64, hits: &mut Vec<u32>) -> Vec<usize> {
        let q = Bounds {
            min_x: b.min_x - radius,
            min_y: b.min_y - radius,
            max_x: b.max_x + radius,
            max_y: b.max_y + radius,
        };
        hits.clear();
        self.tree.search(&q, hits);
        let mut routes: Vec<usize> = hits
            .iter()
            .map(|&i| self.owner[i as usize].0 as usize)
            .collect();
        routes.sort_unstable();
        routes.dedup();
        routes
    }

    /// Each route within `radius` of `p` with its closest place (as
    /// [`Walk::locate`] finds it), nearest first, equally near ones in route
    /// order; only the nearest unless `all`.
    pub fn near(&self, p: Vec2, radius: f64, all: bool, hits: &mut Vec<u32>) -> Vec<Near> {
        let q = Bounds {
            min_x: p.x - radius,
            min_y: p.y - radius,
            max_x: p.x + radius,
            max_y: p.y + radius,
        };
        hits.clear();
        self.tree.search(&q, hits);
        // The best edge of each route: the nearest, the first in walked order on a tie.
        let mut best: Vec<(usize, usize, f64)> = Vec::new();
        for &i in hits.iter() {
            let (r, k) = self.owner[i as usize];
            let (r, k) = (r as usize, k as usize);
            let d = self.walks[r].distance_to(k, p);
            match best.iter_mut().find(|b| b.0 == r) {
                Some(b) => {
                    if d < b.2 || (d == b.2 && k < b.1) {
                        *b = (r, k, d);
                    }
                }
                None => best.push((r, k, d)),
            }
        }
        best.retain(|b| b.2 <= radius);
        best.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.0.cmp(&b.0)));
        if !all {
            best.truncate(1);
        }
        best.into_iter()
            .map(|(r, k, _)| Near {
                route: r,
                place: self.walks[r].place_on(k, p).1,
            })
            .collect()
    }
}
