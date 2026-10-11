//! Küçükleri komşuya kat (docs/adr/0221 §7): ArcGIS's and QGIS's
//! Eliminate. Areas smaller than the least area join, smallest first, the
//! large neighbour they share the longest boundary with (or the largest
//! one); a small one with no large neighbour waits for a later pass, when
//! the small ones around it may have joined one. The joined shape is the
//! overlay's union.

use crate::geom::arrangement::Area;
use crate::geom::region::net_area;
use crate::geometry::Bounds;
use crate::ops::geoprocess::union_all;
use crate::ops::proximity::shared_boundary;
use crate::ops::spatial_query::{Geometry, TOLERANCE, geometry_of_areas};
use crate::store::rtree::PackedTree;

/// Küçükleri komşuya kat's settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Elimination {
    /// The least area, m².
    pub min_area: f64,
    /// Join the largest neighbour, not the one sharing the longest boundary.
    pub largest: bool,
}

/// A small area joined: which, into which, and the boundary they shared (m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joined {
    pub small: usize,
    pub into: usize,
    pub shared: f64,
}

/// The result: each object's areas now (none for a small one that joined),
/// the joinings in order, the small ones that stayed, each object's net area before.
#[derive(Clone, Debug, PartialEq)]
pub struct Eliminated {
    pub areas: Vec<Option<Vec<Area>>>,
    pub joined: Vec<Joined>,
    pub left: Vec<usize>,
    pub sizes: Vec<f64>,
}

/// Küçükleri komşuya kat on objects' areas (input order).
pub fn eliminate(objects: &[Vec<Area>], rule: &Elimination) -> Eliminated {
    let n = objects.len();
    let size: Vec<f64> = objects
        .iter()
        .map(|a| a.iter().map(net_area).sum())
        .collect();
    let small: Vec<bool> = size.iter().map(|&a| a < rule.min_area).collect();
    let mut current: Vec<Option<Vec<Area>>> = objects.iter().map(|a| Some(a.clone())).collect();
    let mut geoms: Vec<Option<Geometry>> = objects
        .iter()
        .map(|a| geometry_of_areas(a.clone()))
        .collect();
    let mut area_now = size.clone();
    // Who holds each object now: a large one itself, a small one the large one it joined.
    let mut owner: Vec<Option<usize>> = (0..n).map(|i| (!small[i]).then_some(i)).collect();
    let boxes: Vec<(u32, Bounds)> = geoms
        .iter()
        .enumerate()
        .filter_map(|(i, g)| {
            g.as_ref().map(|g| {
                let b = g.bounds;
                (
                    i as u32,
                    Bounds {
                        min_x: b.min_x - TOLERANCE,
                        min_y: b.min_y - TOLERANCE,
                        max_x: b.max_x + TOLERANCE,
                        max_y: b.max_y + TOLERANCE,
                    },
                )
            })
        })
        .collect();
    let tree = PackedTree::build(&boxes);
    let mut order: Vec<usize> = (0..n).filter(|&i| small[i] && geoms[i].is_some()).collect();
    order.sort_by(|&a, &b| size[a].total_cmp(&size[b]).then(a.cmp(&b)));
    let mut joined = Vec::new();
    let mut found = Vec::new();
    loop {
        let mut any = false;
        for &s in &order {
            if owner[s].is_some() {
                continue;
            }
            let Some(g) = geoms[s].as_ref() else { continue };
            found.clear();
            tree.search(&g.bounds, &mut found);
            let mut owners: Vec<usize> = found.iter().filter_map(|&j| owner[j as usize]).collect();
            owners.sort_unstable();
            owners.dedup();
            let mut best: Option<(usize, f64, f64)> = None;
            for o in owners {
                let Some(h) = geoms[o].as_ref() else { continue };
                let shared = shared_boundary(g, h, TOLERANCE).length;
                if shared <= 0.0 {
                    continue;
                }
                let score = if rule.largest { area_now[o] } else { shared };
                if best.is_none_or(|(_, b, _)| score > b) {
                    best = Some((o, score, shared));
                }
            }
            let Some((o, _, shared)) = best else { continue };
            let mut both = current[o].take().unwrap_or_default();
            both.extend(current[s].take().unwrap_or_default());
            let union = union_all(&both);
            area_now[o] = union.iter().map(net_area).sum();
            geoms[o] = geometry_of_areas(union.clone());
            current[o] = Some(union);
            owner[s] = Some(o);
            joined.push(Joined {
                small: s,
                into: o,
                shared,
            });
            any = true;
        }
        if !any {
            break;
        }
    }
    let left: Vec<usize> = order
        .iter()
        .copied()
        .filter(|&s| owner[s].is_none())
        .collect();
    Eliminated {
        areas: current,
        joined,
        left,
        sizes: size,
    }
}
