//! Bitişik alan and the overlap control (docs/adr/0162): a new area less
//! the neighbours it overlaps (`Neighbours::avoid`, §2), the region a
//! drawn path closes with them (`Neighbours::fill`, §3) and, with Topoloji
//! on, the corners it shares with them (`junctions`, §4). The web reaches
//! it through WASM (`AdjoinWork`; ops `adjoinAvoid`, `adjoinFill`,
//! `adjoinJunctions`); the independent reference is
//! `scripts/fixtures/adjoin_cases.py`.

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{Entity, Shape, area_parts};
use crate::geom::arrangement::{Area, Ring, Source, TOL, edge_box};
use crate::geom::bulge::{bulge_of_sweep, bulge_path_edges};
use crate::geom::intersect::{Edge, closest_on_edge};
use crate::geom::overlay::adjoin_faces;
use crate::geom::region::{area_source, intersect_area_sets, ring_edges, subtract_areas};
use crate::geometry::{Bounds, dist};
use crate::jsmath::{js_max, js_min};
use crate::op;
use crate::ops::areas::areas_of_entity;
use crate::ops::topology_edit::{Neighbour, Path, SAME, padded, paths_of, shape_of};
use crate::vec2::Vec2;

/// What the overlap control leaves of a new area (§2).
pub struct Avoided {
    /// The area less the neighbours: the area as given when none overlaps
    /// it, none when they cover it, more than one when they cut it apart.
    pub areas: Vec<Area>,
    /// The neighbours whose inside meets the area's, by their places.
    pub overlapped: Vec<usize>,
}

crate::json_struct!(out Avoided { areas, overlapped });

const EMPTY: Bounds = Bounds {
    min_x: f64::INFINITY,
    min_y: f64::INFINITY,
    max_x: f64::NEG_INFINITY,
    max_y: f64::NEG_INFINITY,
};

fn grow(b: Bounds, e: Bounds) -> Bounds {
    Bounds {
        min_x: js_min(b.min_x, e.min_x),
        min_y: js_min(b.min_y, e.min_y),
        max_x: js_max(b.max_x, e.max_x),
        max_y: js_max(b.max_y, e.max_y),
    }
}

fn area_box(a: &Area) -> Bounds {
    ring_edges(&a.outer)
        .iter()
        .fold(EMPTY, |b, e| grow(b, edge_box(e)))
}

/// Boxes that meet, touching (within 1 µm) included.
fn meet(a: &Bounds, b: &Bounds) -> bool {
    a.min_x <= b.max_x + TOL
        && b.min_x <= a.max_x + TOL
        && a.min_y <= b.max_y + TOL
        && b.min_y <= a.max_y + TOL
}

/// The neighbours: each object's areas (a multi-part area's parts,
/// docs/adr/0143) and their box.
pub struct Neighbours {
    sets: Vec<Vec<Area>>,
    boxes: Vec<Bounds>,
}

impl Neighbours {
    pub fn new(sets: Vec<Vec<Area>>) -> Neighbours {
        let boxes = sets
            .iter()
            .map(|set| set.iter().fold(EMPTY, |b, a| grow(b, area_box(a))))
            .collect();
        Neighbours { sets, boxes }
    }

    /// The areas of entities (the area tools' kinds, `ops::areas`); the ones
    /// that enclose none are left out.
    pub fn of_entities(list: &[Entity]) -> Neighbours {
        Neighbours::new(
            list.iter()
                .map(|e| areas_of_entity(&e.shape))
                .filter(|set| !set.is_empty())
                .collect(),
        )
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// Every edge of every neighbour: what a preview has to cut through.
    pub fn edge_count(&self) -> usize {
        self.sets
            .iter()
            .flatten()
            .map(|a| {
                ring_edges(&a.outer).len()
                    + a.holes.iter().map(|h| ring_edges(h).len()).sum::<usize>()
            })
            .sum()
    }

    /// The new area less the neighbours that overlap it (§2); one that only
    /// touches it cuts nothing and adds no corner (joining corners is
    /// Topoloji's, §4).
    pub fn avoid(&self, area: &Area) -> Avoided {
        let bx = area_box(area);
        let near: Vec<usize> = (0..self.sets.len())
            .filter(|&k| meet(&self.boxes[k], &bx))
            .collect();
        let overlapped: Vec<usize> = near
            .iter()
            .copied()
            .filter(|&k| {
                !intersect_area_sets(&[vec![area.clone()], self.sets[k].clone()]).is_empty()
            })
            .collect();
        if overlapped.is_empty() {
            return Avoided {
                areas: vec![area.clone()],
                overlapped,
            };
        }
        let cutters: Vec<Area> = overlapped
            .iter()
            .flat_map(|&k| self.sets[k].iter().cloned())
            .collect();
        Avoided {
            areas: subtract_areas(std::slice::from_ref(area), &cutters),
            overlapped,
        }
    }

    /// The region the open path `pts` (DXF bulges, one per segment) closes
    /// with the neighbours (§3); none when it closes none.
    pub fn fill(&self, pts: &[Vec2], bulges: Option<&[f64]>) -> Vec<Area> {
        if pts.len() < 2 {
            return Vec::new();
        }
        let path = Source {
            edges: bulge_path_edges(pts, bulges, false),
            points: Some(pts.to_vec()),
            cut: Some(true),
        };
        let sources: Vec<Source> = self.sets.iter().map(|set| area_source(set)).collect();
        adjoin_faces(&sources, &path)
    }
}

/// What joining a new area with its neighbours corner by corner gives
/// (§4): the corners each lacks where the other's lie on its edges.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Joined {
    /// The new area's parts, the neighbours' corners on their edges added.
    pub areas: Vec<Area>,
    /// How many corners the new area took.
    pub taken: usize,
    /// The neighbours given corners of the new area: their places and shapes.
    pub edited: Vec<(usize, Shape)>,
    /// How many corners the neighbours were given, all together.
    pub given: usize,
    /// How many neighbours would have been given one but lie on a locked layer.
    pub locked: usize,
}

impl ToJson for Joined {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "areas", &self.areas);
        field(out, &mut first, "taken", &(self.taken as f64));
        out.push_str(",\"edited\":[");
        for (k, (i, shape)) in self.edited.iter().enumerate() {
            if k > 0 {
                out.push(',');
            }
            out.push('{');
            let mut first = true;
            field(out, &mut first, "index", &(*i as f64));
            field(out, &mut first, "shape", shape);
            out.push('}');
        }
        out.push(']');
        field(out, &mut first, "given", &(self.given as f64));
        field(out, &mut first, "locked", &(self.locked as f64));
        out.push('}');
    }
}

/// A path (one bulge per edge) with each point of `extra` that lies on one
/// of its edges, within 1 µm and farther than that from the edge's ends,
/// added in its place along the edge: the point as given, an arc split on
/// its circle. The path, and how many points were added.
fn with_corners(path: &Path, extra: &[Vec2]) -> (Path, usize) {
    let n = path.pts.len();
    let edges = bulge_path_edges(&path.pts, Some(&path.bulges), path.closed);
    let mut pts = Vec::with_capacity(n);
    let mut bulges = Vec::with_capacity(path.bulges.len());
    let mut added = 0;
    for (j, edge) in edges.iter().enumerate() {
        let (a, b) = (path.pts[j], path.pts[(j + 1) % n]);
        let mut on: Vec<(f64, Vec2)> = extra
            .iter()
            .filter_map(|&p| {
                let c = closest_on_edge(edge, p);
                (c.d <= SAME && dist(p, a) > SAME && dist(p, b) > SAME).then_some((c.t, p))
            })
            .collect();
        on.sort_by(|x, y| x.0.total_cmp(&y.0));
        // Corners within 1 µm of each other are one.
        on.dedup_by(|later, kept| dist(later.1, kept.1) <= SAME);
        pts.push(a);
        let bulge = path.bulges.get(j).copied().unwrap_or(0.0);
        match *edge {
            _ if on.is_empty() => bulges.push(bulge),
            Edge::Seg { .. } => {
                for &(_, p) in &on {
                    bulges.push(0.0);
                    pts.push(p);
                }
                bulges.push(0.0);
            }
            Edge::Arc { sweep, .. } => {
                let mut from = 0.0;
                for &(t, p) in &on {
                    bulges.push(bulge_of_sweep(sweep * (t - from)));
                    pts.push(p);
                    from = t;
                }
                bulges.push(bulge_of_sweep(sweep * (1.0 - from)));
            }
        }
        added += on.len();
    }
    if !path.closed
        && let Some(&last) = path.pts.last()
    {
        pts.push(last);
    }
    let path = Path {
        pts,
        bulges,
        closed: path.closed,
    };
    (path, added)
}

/// A ring of the new area with `extra`'s points on its edges added; a ring
/// without bulges stays without when its new edges are straight.
fn ring_with(ring: &Ring, extra: &[Vec2]) -> (Ring, usize) {
    let path = Path {
        pts: ring.pts.clone(),
        bulges: padded(&ring.bulges, ring.pts.len()),
        closed: true,
    };
    let (path, added) = with_corners(&path, extra);
    let straight = ring.bulges.is_none() && path.bulges.iter().all(|&b| b == 0.0);
    let ring = Ring {
        pts: path.pts,
        bulges: (!straight).then_some(path.bulges),
    };
    (ring, added)
}

/// The new area (its parts) and its neighbours joined corner by corner
/// (§4): a corner of the new area on a neighbour's edge is added to the
/// neighbour, a neighbour's corner on the new area's edge to the new area;
/// with `points`, a point on its edge too (Topoloji's Noktalar da). Only
/// the new area's own corners are given; a locked neighbour is counted and
/// left as it is, though the new area takes its corners. The neighbours'
/// elevations are their command's (`cad.entities.edit` carries them along
/// the edge, docs/adr/0142).
pub fn junctions(areas: &[Area], neighbours: &[Neighbour], points: bool) -> Joined {
    let corners: Vec<Vec2> = areas
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(&a.holes))
        .flat_map(|r| r.pts.iter().copied())
        .collect();
    let mut joined = Joined::default();
    let mut theirs = Vec::new();
    for (i, n) in neighbours.iter().enumerate() {
        if let Shape::Point { .. } = &n.shape {
            // Every point of a multi-point object (docs/adr/0174).
            if points {
                theirs.extend(area_parts(&n.shape).iter().filter_map(|s| match s {
                    Shape::Point { p, .. } => Some(*p),
                    _ => None,
                }));
            }
            continue;
        }
        let Some((paths, plan)) = paths_of(&n.shape) else {
            continue;
        };
        theirs.extend(paths.iter().flat_map(|p| p.pts.iter().copied()));
        let mut given = 0;
        let paths: Vec<Path> = paths
            .iter()
            .map(|path| {
                let (path, added) = with_corners(path, &corners);
                given += added;
                path
            })
            .collect();
        if given == 0 {
            continue;
        }
        if n.locked {
            joined.locked += 1;
            continue;
        }
        joined.given += given;
        joined.edited.push((i, shape_of(paths, plan)));
    }
    for a in areas {
        let (outer, taken) = ring_with(&a.outer, &theirs);
        joined.taken += taken;
        let holes = a
            .holes
            .iter()
            .map(|h| {
                let (hole, taken) = ring_with(h, &theirs);
                joined.taken += taken;
                hole
            })
            .collect();
        joined.areas.push(Area { outer, holes });
    }
    joined
}

pub(crate) static OPS: &[Op] = &[
    op!("adjoinAvoid", |area: Area, neighbours: Vec<Entity>| {
        Neighbours::of_entities(&neighbours).avoid(&area)
    }),
    op!(
        "adjoinAvoidAreas",
        |area: Area, neighbours: Vec<Vec<Area>>| { Neighbours::new(neighbours).avoid(&area) }
    ),
    op!(
        "adjoinFill",
        |pts: Vec<Vec2>, bulges: Option<Vec<f64>>, neighbours: Vec<Entity>| {
            Neighbours::of_entities(&neighbours).fill(&pts, bulges.as_deref())
        }
    ),
    op!(
        "adjoinFillAreas",
        |pts: Vec<Vec2>, bulges: Option<Vec<f64>>, neighbours: Vec<Vec<Area>>| {
            Neighbours::new(neighbours).fill(&pts, bulges.as_deref())
        }
    ),
    op!(
        "adjoinJunctions",
        |areas: Vec<Area>, neighbours: Vec<Neighbour>, points: bool| {
            junctions(&areas, &neighbours, points)
        }
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Shape;
    use crate::geom::arrangement::Ring;

    fn square(x: f64, y: f64, w: f64) -> Area {
        Area {
            outer: Ring {
                pts: vec![
                    Vec2::new(x, y),
                    Vec2::new(x + w, y),
                    Vec2::new(x + w, y + w),
                    Vec2::new(x, y + w),
                ],
                bulges: None,
            },
            holes: Vec::new(),
        }
    }

    #[test]
    fn entities_that_enclose_no_area_are_no_neighbours() {
        let line = Entity::new(Shape::Line {
            a: Vec2::new(0.0, 0.0),
            b: Vec2::new(5.0, 0.0),
        });
        let circle = Entity::new(Shape::Circle {
            c: Vec2::new(10.0, 5.0),
            r: 3.0,
        });
        let n = Neighbours::of_entities(&[line, circle]);
        assert_eq!(n.len(), 1);
        // A circle is two half circles.
        assert_eq!(n.edge_count(), 2);
        let cut = n.avoid(&square(0.0, 0.0, 10.0));
        assert_eq!(cut.overlapped, [0]);
        assert_eq!(cut.areas.len(), 1);
    }

    #[test]
    fn a_neighbour_far_away_leaves_the_area_as_given() {
        let n = Neighbours::new(vec![vec![square(100.0, 100.0, 10.0)]]);
        let area = square(0.0, 0.0, 10.0);
        let cut = n.avoid(&area);
        assert!(cut.overlapped.is_empty());
        assert_eq!(cut.areas.len(), 1);
        assert_eq!(cut.areas[0].outer.pts, area.outer.pts);
    }

    fn neighbour(shape: Shape) -> Neighbour {
        Neighbour {
            shape,
            locked: false,
        }
    }

    fn polygon(pts: &[(f64, f64)]) -> Shape {
        Shape::Polygon {
            pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
            bulges: None,
            holes: None,
            parts: None,
        }
    }

    #[test]
    fn corners_go_both_ways_and_a_locked_neighbour_only_gives() {
        // The new square 0..10. The neighbour on its right (y −5..4) takes
        // the square's corner (10, 0) and gives it its own (10, 4); the
        // locked one above it (y 4..20) would take (10, 10).
        let right = neighbour(polygon(&[
            (10.0, -5.0),
            (20.0, -5.0),
            (20.0, 4.0),
            (10.0, 4.0),
        ]));
        let above = Neighbour {
            shape: polygon(&[(10.0, 4.0), (20.0, 4.0), (20.0, 20.0), (10.0, 20.0)]),
            locked: true,
        };
        let joined = junctions(&[square(0.0, 0.0, 10.0)], &[right, above], false);
        assert_eq!(
            joined.areas[0].outer.pts,
            [
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 4.0),
                Vec2::new(10.0, 10.0),
                Vec2::new(0.0, 10.0)
            ]
        );
        assert_eq!(joined.areas[0].outer.bulges, None);
        assert_eq!(joined.taken, 1);
        // The right one takes (10, 0) on its left edge; the locked one would take (10, 10).
        assert_eq!(joined.given, 1);
        assert_eq!(joined.locked, 1);
        let [(0, Shape::Polygon { pts, .. })] = joined.edited.as_slice() else {
            panic!("one edited polygon: {:?}", joined.edited);
        };
        assert_eq!(
            pts,
            &[
                Vec2::new(10.0, -5.0),
                Vec2::new(20.0, -5.0),
                Vec2::new(20.0, 4.0),
                Vec2::new(10.0, 4.0),
                Vec2::new(10.0, 0.0)
            ]
        );
    }

    #[test]
    fn an_arc_is_split_on_its_circle() {
        // A half disc of radius 5 about (5, 10), its arc running from
        // (10, 10) round to (0, 10) (bulge 1); the new area's corner (5, 15)
        // lies half way along it.
        let half = neighbour(Shape::Polygon {
            pts: vec![Vec2::new(0.0, 10.0), Vec2::new(10.0, 10.0)],
            bulges: Some(vec![0.0, 1.0]),
            holes: None,
            parts: None,
        });
        let top = Vec2::new(5.0, 15.0);
        let spike = Area {
            outer: Ring {
                pts: vec![
                    Vec2::new(4.0, 20.0),
                    Vec2::new(5.0, 15.0),
                    Vec2::new(6.0, 20.0),
                ],
                bulges: None,
            },
            holes: Vec::new(),
        };
        let joined = junctions(&[spike], &[half], false);
        let [(0, Shape::Polygon { pts, bulges, .. })] = joined.edited.as_slice() else {
            panic!("one edited polygon: {:?}", joined.edited);
        };
        assert_eq!(pts[2], top);
        let b = bulges.as_deref().unwrap_or_default();
        let quarter = crate::jsmath::tan(crate::jsmath::PI / 8.0);
        assert!(
            (b[1] - quarter).abs() < 1e-12 && (b[2] - quarter).abs() < 1e-12,
            "{b:?}"
        );
    }

    #[test]
    fn a_path_of_one_point_closes_nothing() {
        let n = Neighbours::new(vec![vec![square(0.0, 0.0, 10.0)]]);
        assert!(n.fill(&[Vec2::new(5.0, 5.0)], None).is_empty());
        assert!(Neighbours::new(Vec::new()).is_empty());
    }
}
