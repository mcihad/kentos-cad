//! What the proximity tools ask the store (docs/adr/0215 §4): each input's
//! nearest targets, `k` of them or all, within a distance or not, edge to
//! edge or centre to centre; and the areas' neighbours with the boundary
//! they share and whether their insides overlap. A run builds a store of
//! the objects it reads, in the page and in the processing worker alike
//! (`apps/web/src/processing/geometry.ts`), and the desktop's run does the
//! same (`RunGeometry`), so both run this code. Targets are found in a tree
//! of their own, searched best first; neighbours by the store's index.

use std::collections::HashMap;

use super::rtree::{Nearer, NearestQueue, PackedTree};
use super::{Store, padded};
use crate::geometry::Bounds;
use crate::jsmath::js_max;
use crate::ops::proximity::{
    center_distance, interiors_overlap, nearest_points, overlap_area, shared_boundary,
};
use crate::ops::spatial_query::{Geometry, TOLERANCE, geometry_of};
use crate::survey::bearing;
use crate::vec2::Vec2;

/// Numbers per `nearest` record: the input's place, the target's, the
/// distance, the input's nearest point (x, y), the target's (x, y) and the
/// bearing from the first to the second (radians, clockwise from north;
/// NaN when they are 0 apart).
pub const NEAREST_STRIDE: usize = 8;
/// Numbers per `neighbors` record: the area's place, its neighbour's, the
/// kind (`NEIGHBOR_EDGE`, `NEIGHBOR_CORNER`, `NEIGHBOR_OVERLAP`), the shared
/// boundary's length and the overlapping area.
pub const NEIGHBOR_STRIDE: usize = 5;
pub const NEIGHBOR_EDGE: f64 = 0.0;
pub const NEIGHBOR_CORNER: f64 = 1.0;
pub const NEIGHBOR_OVERLAP: f64 = 2.0;

/// How a distance is measured (docs/adr/0215 §2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Measure {
    /// Edge to edge: Konuma göre seç's distance (0 when they meet).
    Edges,
    /// Centre to centre (`$merkez`).
    Centers,
}

impl Measure {
    /// The code the web's store binding passes.
    pub fn from_code(code: u32) -> Option<Measure> {
        match code {
            0 => Some(Measure::Edges),
            1 => Some(Measure::Centers),
            _ => None,
        }
    }
}

/// One target measured from an input.
#[derive(Clone, Copy, Debug)]
struct Found {
    j: usize,
    d: f64,
    a: (f64, f64),
    b: (f64, f64),
}

/// A pair of areas' answer: how they neighbour, the boundary they share and
/// the area they overlap; none: they are not neighbours.
type Answer = Option<(f64, f64, f64)>;

/// What a measure may take off a gap between boxes: objects within
/// `TOLERANCE` meet (0 apart), and the rounding of either way of measuring.
const SLACK: f64 = 2.0 * TOLERANCE;

/// The least distance two boxes' contents can have, less `SLACK`.
fn gap(a: &Bounds, b: &Bounds) -> f64 {
    let dx = js_max(0.0, js_max(b.min_x - a.max_x, a.min_x - b.max_x));
    let dy = js_max(0.0, js_max(b.min_y - a.max_y, a.min_y - b.max_y));
    js_max(0.0, (dx * dx + dy * dy).sqrt() - SLACK)
}

fn point_box(p: Vec2) -> Bounds {
    Bounds {
        min_x: p.x,
        min_y: p.y,
        max_x: p.x,
        max_y: p.y,
    }
}

fn finite(b: &Bounds) -> bool {
    b.min_x.is_finite() && b.min_y.is_finite() && b.max_x.is_finite() && b.max_y.is_finite()
}

/// One input's search for its nearest targets (`Store::nearest`).
struct Search<'a> {
    /// Its place, id, geometry and the box it is measured from.
    input: (usize, f64, &'a Geometry, Bounds),
    geoms: &'a [Option<Geometry>],
    targets: &'a [f64],
    measure: Measure,
    k: usize,
    taken: usize,
    found: &'a mut Vec<Found>,
    out: &'a mut Vec<f64>,
}

impl Nearer for Search<'_> {
    fn gap(&self, b: &Bounds) -> f64 {
        gap(&self.input.3, b)
    }

    fn measure(&mut self, j: u32) -> Option<(f64, u32, u32)> {
        let (_, id, g, _) = self.input;
        let at = j as usize;
        if self.targets[at] == id {
            return None;
        }
        let t = self.geoms[at].as_ref()?;
        let f = match self.measure {
            Measure::Edges => {
                let n = nearest_points(g, t)?;
                Found {
                    j: at,
                    d: n.d,
                    a: (n.a.x, n.a.y),
                    b: (n.b.x, n.b.y),
                }
            }
            Measure::Centers => {
                let (ca, cb) = (g.center?, t.center?);
                Found {
                    j: at,
                    d: center_distance(g, t)?,
                    a: (ca.x, ca.y),
                    b: (cb.x, cb.y),
                }
            }
        };
        self.found.push(f);
        Some((f.d, j, (self.found.len() - 1) as u32))
    }

    fn take(&mut self, token: u32) -> bool {
        let f = self.found[token as usize];
        let t = if f.d > 0.0 {
            bearing(Vec2::new(f.a.0, f.a.1), Vec2::new(f.b.0, f.b.1))
        } else {
            f64::NAN
        };
        self.out.extend([
            self.input.0 as f64,
            f.j as f64,
            f.d,
            f.a.0,
            f.a.1,
            f.b.0,
            f.b.1,
            t,
        ]);
        self.taken += 1;
        self.k == 0 || self.taken < self.k
    }
}

impl Store {
    /// The geometries of these ids (none for an unknown id or a
    /// construction line) and where each id is first in the list.
    fn geometries(&self, ids: &[f64]) -> (Vec<Option<Geometry>>, HashMap<u64, usize>) {
        let list = ids
            .iter()
            .map(|&id| self.get(id).and_then(|it| geometry_of(&it.shape)))
            .collect();
        let mut place = HashMap::new();
        for (j, id) in ids.iter().enumerate() {
            place.entry(id.to_bits()).or_insert(j);
        }
        (list, place)
    }

    /// Each input's nearest targets (docs/adr/0215 §2.1): `k` of them (0:
    /// all) within `max` (infinite: no bound), by distance then the targets'
    /// order, `NEAREST_STRIDE` numbers each, inputs in their order. An
    /// object is never its own target; unknown ids and construction lines
    /// take no part; a centre measure skips objects without a centre. The
    /// targets' own tree is searched best first (`PackedTree::nearest`), its
    /// boxes' gaps less `SLACK` never more than a measure.
    pub fn nearest(
        &self,
        inputs: &[f64],
        targets: &[f64],
        k: usize,
        max: f64,
        measure: Measure,
    ) -> Vec<f64> {
        let (geoms, place) = self.geometries(targets);
        let key = |g: &Geometry| match measure {
            Measure::Edges => (!g.is_empty()).then_some(g.bounds),
            Measure::Centers => g.center.map(point_box),
        };
        let entries: Vec<(u32, Bounds)> = geoms
            .iter()
            .enumerate()
            .filter(|&(j, _)| place.get(&targets[j].to_bits()) == Some(&j))
            .filter_map(|(j, g)| Some((j as u32, key(g.as_ref()?)?)))
            .filter(|(_, b)| finite(b))
            .collect();
        let tree = PackedTree::build(&entries);
        let mut queue = NearestQueue::default();
        let mut found: Vec<Found> = Vec::new();
        let mut out = Vec::new();
        for (i, &id) in inputs.iter().enumerate() {
            let Some(g) = self.get(id).and_then(|it| geometry_of(&it.shape)) else {
                continue;
            };
            let Some(from) = key(&g) else {
                continue;
            };
            found.clear();
            let mut search = Search {
                input: (i, id, &g, from),
                geoms: &geoms,
                targets,
                measure,
                k,
                taken: 0,
                found: &mut found,
                out: &mut out,
            };
            tree.nearest(&mut queue, max, &mut search);
        }
        out
    }

    /// The areas' neighbours (docs/adr/0215 §2.2): for each area in the list's
    /// order its neighbours in theirs, `NEIGHBOR_STRIDE` numbers each, both
    /// ways round: an edge neighbour shares more than `tol` of boundary, a
    /// corner neighbour meets it with less (only when `corners`), an
    /// overlapping one shares inside (only when `overlaps`; its area by the
    /// overlay). Objects without an area take no part.
    pub fn neighbors(&self, ids: &[f64], tol: f64, corners: bool, overlaps: bool) -> Vec<f64> {
        let (geoms, place) = self.geometries(ids);
        // A pair's answer, worked out once when the first of the two meets the other.
        let mut pairs: HashMap<(usize, usize), Answer> = HashMap::new();
        let mut out = Vec::new();
        for (i, g) in geoms.iter().enumerate() {
            let Some(g) = g.as_ref().filter(|g| !g.areas.is_empty()) else {
                continue;
            };
            let mut near: Vec<usize> = self
                .candidates(&padded(g.bounds, tol))
                .into_iter()
                .filter_map(|it| place.get(&it.id.to_bits()).copied())
                .filter(|&j| j != i && ids[j] != ids[i])
                .collect();
            near.sort_unstable();
            near.dedup();
            for j in near {
                let key = (i.min(j), i.max(j));
                let answer = *pairs.entry(key).or_insert_with(|| {
                    let other = geoms[j].as_ref().filter(|o| !o.areas.is_empty())?;
                    let shared = shared_boundary(g, other, tol);
                    if overlaps && interiors_overlap(g, other) {
                        return Some((NEIGHBOR_OVERLAP, shared.length, overlap_area(g, other)));
                    }
                    if shared.length > tol {
                        Some((NEIGHBOR_EDGE, shared.length, 0.0))
                    } else if corners && shared.meet {
                        Some((NEIGHBOR_CORNER, shared.length, 0.0))
                    } else {
                        None
                    }
                });
                if let Some((kind, length, area)) = answer {
                    out.extend([i as f64, j as f64, kind, length, area]);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(objects: &[String]) -> Store {
        let mut s = Store::new();
        s.put_json(&format!("[{}]", objects.join(",")))
            .expect("the store reads the objects");
        s
    }

    fn square(id: u32, x: f64, y: f64, side: f64) -> String {
        format!(
            r#"{{"id":{id},"layerId":"a","kind":"polygon","pts":[{{"x":{x},"y":{y}}},{{"x":{},"y":{y}}},{{"x":{},"y":{}}},{{"x":{x},"y":{}}}]}}"#,
            x + side,
            x + side,
            y + side,
            y + side
        )
    }

    fn point(id: u32, x: f64, y: f64) -> String {
        format!(r#"{{"id":{id},"layerId":"b","kind":"point","p":{{"x":{x},"y":{y}}}}}"#)
    }

    #[test]
    fn the_nearest_targets_come_by_distance_then_order() {
        let s = store(&[
            square(1, 0.0, 0.0, 10.0),
            point(2, 30.0, 5.0),
            point(3, 13.0, 5.0),
            point(4, -3.0, 5.0),
            point(5, 500.0, 5.0),
        ]);
        let r = s.nearest(
            &[1.0],
            &[2.0, 3.0, 4.0, 5.0],
            2,
            f64::INFINITY,
            Measure::Edges,
        );
        // The point 3 m right (place 1) and 3 m left (place 2): equal, by order.
        assert_eq!(r.len(), 2 * NEAREST_STRIDE);
        let second = NEAREST_STRIDE;
        assert_eq!(
            (r[1], r[2], r[second + 1], r[second + 2]),
            (1.0, 3.0, 2.0, 3.0)
        );
        // All of them, the far one too; within 20 m only the two.
        assert_eq!(
            s.nearest(
                &[1.0],
                &[2.0, 3.0, 4.0, 5.0],
                0,
                f64::INFINITY,
                Measure::Edges
            )
            .len(),
            4 * NEAREST_STRIDE
        );
        assert_eq!(
            s.nearest(&[1.0], &[2.0, 3.0, 4.0, 5.0], 0, 20.0, Measure::Edges)
                .len(),
            3 * NEAREST_STRIDE
        );
        // Centre to centre: from (5, 5).
        let c = s.nearest(&[1.0], &[2.0, 3.0], 1, f64::INFINITY, Measure::Centers);
        assert_eq!((c[1], c[2]), (1.0, 8.0));
        // Never itself.
        assert!(
            s.nearest(&[1.0], &[1.0], 1, f64::INFINITY, Measure::Edges)
                .is_empty()
        );
    }

    #[test]
    fn neighbours_share_edges_corners_or_insides() {
        let s = store(&[
            square(1, 0.0, 0.0, 10.0),
            square(2, 10.0, 0.0, 10.0),
            square(3, 20.0, 10.0, 10.0),
            square(4, 5.0, 5.0, 10.0),
        ]);
        let r = s.neighbors(&[1.0, 2.0, 3.0], 0.001, true, true);
        // 1–2 an edge of 10, 2–3 a corner; both ways round.
        let rows: Vec<(f64, f64, f64, f64)> = r
            .chunks(NEIGHBOR_STRIDE)
            .map(|c| (c[0], c[1], c[2], c[3]))
            .collect();
        assert_eq!(
            rows,
            [
                (0.0, 1.0, NEIGHBOR_EDGE, 10.0),
                (1.0, 0.0, NEIGHBOR_EDGE, 10.0),
                (1.0, 2.0, NEIGHBOR_CORNER, 0.0),
                (2.0, 1.0, NEIGHBOR_CORNER, 0.0),
            ]
        );
        // 4 overlaps 1 by a 5 × 5 square.
        let o = s.neighbors(&[1.0, 4.0], 0.001, false, true);
        assert_eq!((o[2], o[4]), (NEIGHBOR_OVERLAP, 25.0));
    }
}
