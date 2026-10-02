//! Bitişik alan and the overlap control (docs/adr/0162): a new area less
//! the neighbours it overlaps (`Neighbours::avoid`, §2) and the region a
//! drawn path closes with them (`Neighbours::fill`, §3). The web reaches it
//! through WASM (`AdjoinWork`; ops `adjoinAvoid`, `adjoinFill`); the
//! independent reference is `scripts/fixtures/adjoin_cases.py`.

use crate::api::Op;
use crate::entity::Entity;
use crate::geom::arrangement::{Area, Source, TOL, edge_box};
use crate::geom::bulge::bulge_path_edges;
use crate::geom::overlay::adjoin_faces;
use crate::geom::region::{area_source, intersect_area_sets, ring_edges, subtract_areas};
use crate::geometry::Bounds;
use crate::jsmath::{js_max, js_min};
use crate::op;
use crate::ops::areas::areas_of_entity;
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

    #[test]
    fn a_path_of_one_point_closes_nothing() {
        let n = Neighbours::new(vec![vec![square(0.0, 0.0, 10.0)]]);
        assert!(n.fill(&[Vec2::new(5.0, 5.0)], None).is_empty());
        assert!(Neighbours::new(Vec::new()).is_empty());
    }
}
