//! Alanları kümele (docs/adr/0221 §5): areas no farther apart than the
//! aggregation distance join through bridges. A cluster's closing (its
//! union taken out by half the distance and back in, the exact buffer of
//! ADR 0201) is what fills between them; of the closing's excess over the
//! union only the pieces that share a boundary with two objects or more are
//! taken (the bridges), unless notches are filled too. Clusters far apart
//! do not meet in their closings, so each is closed on its own.

use crate::entity::Shape;
use crate::geom::arrangement::Area;
use crate::geom::region::{net_area, ring_area};
use crate::geometry::Bounds;
use crate::ops::areas::{areas_of_entity, polygon_of_area};
use crate::ops::convert::points::interior_point;
use crate::ops::geoprocess::buffer::{Side, buffer};
use crate::ops::geoprocess::{Class, subtract_all, union_all};
use crate::ops::proximity::shared_boundary;
use crate::ops::spatial_query::{
    Geometry, TOLERANCE, distance as distance_between, geometry_of_areas, inside,
};
use crate::store::rtree::PackedTree;

/// Alanları kümele's settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aggregation {
    /// The aggregation distance, m.
    pub distance: f64,
    /// The least area of a result, m² (smaller ones are not written).
    pub min_area: f64,
    /// The least hole, m² (smaller ones are filled).
    pub min_hole: f64,
    /// The closing's notches in one object are filled too.
    pub fill_notches: bool,
}

/// A result: its areas (one area and its holes) and the objects in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Aggregate {
    pub area: Area,
    pub members: Vec<usize>,
}

/// A group's results, and how many were smaller than the least area.
#[derive(Clone, Debug, PartialEq)]
pub struct Aggregated {
    pub results: Vec<Aggregate>,
    pub dropped: usize,
}

fn find(parent: &mut [usize], i: usize) -> usize {
    let mut r = i;
    while parent[r] != r {
        r = parent[r];
    }
    let mut k = i;
    while parent[k] != r {
        let up = parent[k];
        parent[k] = r;
        k = up;
    }
    r
}

/// Whether an area is one convex ring of straight edges, no holes: its closing is itself.
fn convex(a: &Area) -> bool {
    if !a.holes.is_empty()
        || a.outer
            .bulges
            .as_ref()
            .is_some_and(|b| b.iter().any(|&x| x != 0.0))
    {
        return false;
    }
    let p = &a.outer.pts;
    let n = p.len();
    if n < 3 {
        return false;
    }
    let sign = if ring_area(&a.outer) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    (0..n).all(|i| {
        let (a, b, c) = (p[i], p[(i + 1) % n], p[(i + 2) % n]);
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        cross * sign >= 0.0
    })
}

/// The bridges and what else the closing adds to a cluster's areas; their union.
fn closed(objects: &[(usize, &Vec<Area>, Option<&Geometry>)], rule: &Aggregation) -> Vec<Area> {
    let all: Vec<Area> = objects
        .iter()
        .flat_map(|(_, a, _)| a.iter().cloned())
        .collect();
    // One object has no bridges; a convex one has no notches either.
    if objects.len() == 1 && (!rule.fill_notches || (all.len() == 1 && convex(&all[0]))) {
        return if all.len() == 1 { all } else { union_all(&all) };
    }
    let r = rule.distance / 2.0;
    // The buffer takes the areas' union itself; the excess is the closing less every area.
    let out = buffer(&Class::Areas(all.clone()), r, Side::Both);
    let closing = buffer(&Class::Areas(out), -r, Side::Both);
    let extra = subtract_all(&closing, &all);
    let mut taken = all;
    for piece in extra {
        let keep = rule.fill_notches || {
            let g = geometry_of_areas(vec![piece.clone()]);
            let touching = objects
                .iter()
                .filter(|(_, _, o)| match (&g, o) {
                    (Some(g), Some(o)) => shared_boundary(g, o, TOLERANCE).length > 0.0,
                    _ => false,
                })
                .count();
            touching >= 2
        };
        if keep {
            taken.push(piece);
        }
    }
    union_all(&taken)
}

fn first_corner(a: &Area) -> (f64, f64) {
    let mut b = (f64::INFINITY, f64::INFINITY);
    for p in &a.outer.pts {
        if (p.y, p.x) < b {
            b = (p.y, p.x);
        }
    }
    b
}

/// A group's objects as the clusters read them: their areas and query geometries.
struct Objects {
    areas: Vec<Vec<Area>>,
    geoms: Vec<Option<Geometry>>,
}

impl Objects {
    fn of(shapes: &[&Shape]) -> Objects {
        let areas: Vec<Vec<Area>> = shapes.iter().map(|s| areas_of_entity(s)).collect();
        let geoms = areas.iter().map(|a| geometry_of_areas(a.clone())).collect();
        Objects { areas, geoms }
    }
}

/// A group's clusters (§5): objects no farther apart than the distance, by
/// their first object, each in input order. A cluster's closing does not
/// reach another's: each can be closed on its own (the desktop closes them
/// on several threads).
pub fn clusters(shapes: &[&Shape], distance: f64) -> Vec<Vec<usize>> {
    let o = Objects::of(shapes);
    let d = distance;
    let boxes: Vec<(u32, Bounds)> = o
        .geoms
        .iter()
        .enumerate()
        .filter_map(|(i, g)| {
            g.as_ref().map(|g| {
                (
                    i as u32,
                    Bounds {
                        min_x: g.bounds.min_x - d,
                        min_y: g.bounds.min_y - d,
                        max_x: g.bounds.max_x + d,
                        max_y: g.bounds.max_y + d,
                    },
                )
            })
        })
        .collect();
    let tree = PackedTree::build(&boxes);
    let mut parent: Vec<usize> = (0..shapes.len()).collect();
    let mut near = Vec::new();
    for (i, g) in o.geoms.iter().enumerate() {
        let Some(g) = g else { continue };
        near.clear();
        tree.search(&g.bounds, &mut near);
        near.sort_unstable();
        for &j in &near {
            let j = j as usize;
            if j <= i || find(&mut parent, i) == find(&mut parent, j) {
                continue;
            }
            if let Some(h) = &o.geoms[j]
                && distance_between(g, h) <= d
            {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut slot = vec![usize::MAX; shapes.len()];
    for i in 0..shapes.len() {
        if o.geoms[i].is_none() {
            continue;
        }
        let root = find(&mut parent, i);
        if slot[root] == usize::MAX {
            slot[root] = out.len();
            out.push(Vec::new());
        }
        out[slot[root]].push(i);
    }
    out
}

/// One cluster closed (§5): its results in order and how many were smaller than the least area.
pub fn close(shapes: &[&Shape], members: &[usize], rule: &Aggregation) -> (Vec<Aggregate>, usize) {
    let picked: Vec<&Shape> = members.iter().map(|&i| shapes[i]).collect();
    let o = Objects::of(&picked);
    let objects: Vec<(usize, &Vec<Area>, Option<&Geometry>)> = members
        .iter()
        .enumerate()
        .map(|(k, &i)| (i, &o.areas[k], o.geoms[k].as_ref()))
        .collect();
    let mut pieces = closed(&objects, rule);
    pieces.sort_by(|a, b| {
        first_corner(a)
            .partial_cmp(&first_corner(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Each area's interior point: a result counts the objects with one inside it.
    let inner: Vec<Vec<_>> = o
        .areas
        .iter()
        .map(|list| {
            list.iter()
                .filter_map(|a| interior_point(&polygon_of_area(a).shape))
                .collect()
        })
        .collect();
    let mut results = Vec::new();
    let mut dropped = 0;
    for mut piece in pieces {
        piece.holes.retain(|h| ring_area(h).abs() >= rule.min_hole);
        if net_area(&piece) < rule.min_area {
            dropped += 1;
            continue;
        }
        let inside_it: Vec<usize> = members
            .iter()
            .enumerate()
            .filter(|&(k, _)| {
                inner[k]
                    .iter()
                    .any(|&p| inside(std::slice::from_ref(&piece), p))
            })
            .map(|(_, &i)| i)
            .collect();
        results.push(Aggregate {
            area: piece,
            members: inside_it,
        });
    }
    (results, dropped)
}

/// Alanları kümele on a group's objects (their shapes, in input order).
pub fn aggregate(shapes: &[&Shape], rule: &Aggregation) -> Aggregated {
    let mut results = Vec::new();
    let mut dropped = 0;
    for members in clusters(shapes, rule.distance) {
        let (got, small) = close(shapes, &members, rule);
        results.extend(got);
        dropped += small;
    }
    Aggregated { results, dropped }
}
