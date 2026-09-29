//! A multi-part area's parts (docs/adr/0143): which part a grip is on, the
//! parts as areas of their own, and areas joined into one. The rest of the
//! core takes an area part by part through `entity::area_parts`.

use crate::api::Op;
use crate::entity::{Entity, Shape, area_parts, join_parts};
use crate::op;
use crate::ops::areas::areas_of_entity;
use crate::ops::grips::grip_part;

/// Where a grip is: its part (0: the area's own fields) and its index there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GripPart {
    pub part: usize,
    pub index: usize,
}

crate::json_struct!(GripPart { part, index });

/// The part grip `index` is on, and its index within that part.
pub fn grip_part_of(e: &Shape, index: usize) -> Option<GripPart> {
    grip_part(e, index).map(|(part, index)| GripPart { part, index })
}

/// Each part of an area as an area of its own (the entity's other fields
/// kept); anything else as itself.
pub fn split_parts(e: &Entity) -> Vec<Entity> {
    area_parts(&e.shape)
        .iter()
        .map(|part| e.with(part.clone()))
        .collect()
}

/// The areas as one, their parts in order, with the first entity's other
/// fields; none when one of them is not an area.
pub fn joined_parts(entities: &[Entity]) -> Option<Entity> {
    let first = entities.first()?;
    let shapes: Vec<Shape> = entities.iter().map(|e| e.shape.clone()).collect();
    Some(first.with(join_parts(&shapes)?))
}

pub(crate) static OPS: &[Op] = &[
    op!("gripPart", |e: Entity, index: usize| grip_part_of(
        &e.shape, index
    )),
    op!("splitParts", |e: Entity| split_parts(&e)),
    op!("joinParts", |entities: Vec<Entity>| joined_parts(&entities)),
    op!("areasOfEntity", |e: Entity| areas_of_entity(&e.shape)),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Part, entity_area, entity_length};
    use crate::ops::grips::{entity_grips, hole_grip, mid_grip_segment, move_grip};
    use crate::vec2::Vec2;

    fn square(x: f64, side: f64) -> Vec<Vec2> {
        vec![
            Vec2::new(x, 0.0),
            Vec2::new(x + side, 0.0),
            Vec2::new(x + side, side),
            Vec2::new(x, side),
        ]
    }

    /// A 10 m square with a 2 m hole, and a 4 m square part 20 m east.
    fn two() -> Shape {
        Shape::Polygon {
            pts: square(0.0, 10.0),
            bulges: None,
            holes: Some(vec![crate::geom::arrangement::Ring {
                pts: square(4.0, 2.0)
                    .iter()
                    .map(|p| Vec2::new(p.x, p.y + 4.0))
                    .collect(),
                bulges: None,
            }]),
            parts: Some(vec![Part {
                pts: square(20.0, 4.0),
                bulges: None,
                holes: None,
            }]),
        }
    }

    #[test]
    fn an_area_measures_as_the_sum_of_its_parts() {
        // 100 − 4 and 16 m²; perimeters 40 + 8 (the hole) and 16 m.
        assert_eq!(entity_area(&two()), Some(96.0 + 16.0));
        assert_eq!(entity_length(&two()), Some(48.0 + 16.0));
    }

    #[test]
    fn grips_run_part_after_part_and_move_in_their_own_part() {
        let s = two();
        // First part: 4 vertices, 4 mids, 4 hole vertices; then the part: 4 and 4.
        assert_eq!(entity_grips(&s).len(), 12 + 8);
        assert_eq!(grip_part_of(&s, 11), Some(GripPart { part: 0, index: 11 }));
        assert_eq!(grip_part_of(&s, 12), Some(GripPart { part: 1, index: 0 }));
        assert_eq!(grip_part_of(&s, 20), None);
        assert_eq!(
            mid_grip_segment(&s, 16),
            Some(0),
            "the part's first mid grip"
        );
        assert!(hole_grip(&s, 13).is_none(), "the part has no hole");
        assert!(hole_grip(&s, 8).is_some(), "the first part's hole");
        // Moving the part's first vertex leaves the first part as it was.
        let moved = move_grip(&Entity::new(s.clone()), 12, Vec2::new(19.0, -1.0)).expect("moves");
        let parts = area_parts(&moved.shape);
        assert_eq!(parts[0], area_parts(&s)[0]);
        let Shape::Polygon { pts, .. } = &parts[1] else {
            panic!()
        };
        assert_eq!(pts[0], Vec2::new(19.0, -1.0));
    }

    #[test]
    fn the_store_draws_picks_snaps_and_packs_every_part() {
        use crate::store::draw::{FILL, FILLS};
        use crate::store::snap::SnapKind;
        use crate::store::{Packer, Store};
        let mut store = Store::new();
        store.put(1.0, "a", false, two());
        // One record, two parts: the first part's ring and hole, the second's ring.
        let d = store.drawn(&[1.0], true, None);
        assert_eq!((d[0], d[1], d[2]), (FILLS, 2.0, 2.0));
        assert_ne!(d[0], FILL);
        // Inside the second part picks the area; inside the first part's hole does not.
        assert_eq!(store.hit(Vec2::new(22.0, 2.0), 0.01), Some(1.0));
        assert_eq!(store.hit(Vec2::new(5.0, 5.0), 0.01), None);
        // The second part's corner snaps.
        let hit = store
            .snap(Vec2::new(24.05, 4.05), 0.2, SnapKind::Endpoint.bit(), None)
            .expect("a corner");
        assert_eq!(hit.point, Vec2::new(24.0, 4.0));
        // A crossing window over the second part alone takes the area; a window must hold both.
        let r = |x0: f64, x1: f64| crate::geometry::Bounds {
            min_x: x0,
            min_y: -1.0,
            max_x: x1,
            max_y: 11.0,
        };
        assert_eq!(store.in_rect(&r(19.0, 25.0), true), vec![1.0]);
        assert!(store.in_rect(&r(19.0, 25.0), false).is_empty());
        assert_eq!(store.in_rect(&r(-1.0, 25.0), false), vec![1.0]);
        // Packed and read back as it was (kind 13).
        let mut out = Packer::default();
        out.object(1.0, "a", false, &two());
        assert_eq!(out.nums[3], 13.0);
        let mut back = Store::new();
        back.put_packed(&out.nums, &out.strings).expect("reads");
        assert_eq!(back.get(1.0).map(|it| &it.shape), Some(&two()));
    }

    #[test]
    fn edits_that_run_along_one_ring_refuse_a_multi_part_area() {
        use crate::entity::MULTI_PART_REFUSED;
        use crate::ops::breaking::break_entity;
        use crate::ops::curve_cuts::Cut;
        use crate::ops::trim::trim_entity;
        let e = Entity::new(two());
        let refused = |c: Cut| matches!(c, Cut::Error(m) if m == MULTI_PART_REFUSED);
        assert!(refused(break_entity(
            &e,
            Vec2::new(22.0, 0.0),
            Vec2::new(24.0, 2.0)
        )));
        assert!(refused(trim_entity(&e, Vec2::new(22.0, 0.0), &[])));
    }

    #[test]
    fn a_move_an_offset_and_a_reversal_take_every_part() {
        use crate::geom::affine::translation;
        use crate::ops::curve_cuts::Geometry;
        use crate::ops::offset::offset_entity;
        use crate::ops::reshape::reverse;
        use crate::ops::transform::transform_entity;
        let e = Entity::new(two());
        let moved = transform_entity(&e, &translation(100.0, 0.0));
        let parts = area_parts(&moved.shape);
        let Shape::Polygon { pts, .. } = &parts[1] else {
            panic!()
        };
        assert_eq!(pts[0], Vec2::new(120.0, 0.0));
        // Outward by 1 m from outside: both rings grow, 12 × 12 and 6 × 6 (the first part's hole goes).
        let Geometry::Ok(out) = offset_entity(&e.shape, 1.0, Vec2::new(50.0, 50.0)) else {
            panic!("offsets")
        };
        let mut areas: Vec<f64> = area_parts(&out.shape)
            .iter()
            .filter_map(crate::entity::entity_area)
            .collect();
        areas.sort_by(f64::total_cmp);
        assert_eq!(areas.len(), 2);
        assert!(
            (areas[0] - 36.0).abs() < 1e-9 && (areas[1] - 144.0).abs() < 1e-9,
            "{areas:?}"
        );
        // Reversed: each part's ring runs the other way, the parts stay in order.
        let back = reverse(&e).expect("reverses");
        let parts = area_parts(&back.shape);
        let Shape::Polygon { pts, .. } = &parts[1] else {
            panic!()
        };
        assert_eq!(pts.len(), 4);
        assert!(crate::geometry::signed_area(pts) < 0.0);
    }

    #[test]
    fn parts_split_into_areas_and_join_back() {
        let e = Entity::new(two());
        let split = split_parts(&e);
        assert_eq!(split.len(), 2);
        assert!(
            split
                .iter()
                .all(|p| !crate::entity::is_multi_part(&p.shape))
        );
        let joined = joined_parts(&split).expect("joins");
        assert_eq!(joined.shape, e.shape);
        // A one-part area is itself.
        assert_eq!(split_parts(&split[1]), vec![split[1].clone()]);
    }

    #[test]
    fn a_corner_is_found_and_done_on_its_own_part() {
        use crate::ops::curve_cuts::Geometry;
        use crate::ops::fillet::{CornerOp, corner_in_path};
        use crate::tools::editing::{CornerSite, corner_near, path_corner_at, shared_corner};
        let e = two();
        // The second part's third vertex (24, 4) is the area's seventh outer vertex.
        let hit = corner_near(std::slice::from_ref(&e), Vec2::new(24.1, 4.1), 0.5, 0.01)
            .expect("a corner");
        assert_eq!(
            hit.site,
            CornerSite::Vertex {
                object: 0,
                vertex: 6
            }
        );
        assert_eq!(path_corner_at(&e, 6), Some(hit.corner));
        assert_eq!(path_corner_at(&e, 8), None);
        // Its two edges, picked on the second part, share that vertex; a hole's do not.
        assert_eq!(
            shared_corner(&e, Vec2::new(24.0, 2.0), Vec2::new(22.0, 4.0)),
            Some(6)
        );
        assert_eq!(
            shared_corner(&e, Vec2::new(5.0, 4.0), Vec2::new(6.0, 5.0)),
            None
        );
        assert_eq!(
            shared_corner(&e, Vec2::new(10.0, 5.0), Vec2::new(22.0, 4.0)),
            None
        );
        // Cut there, the second part gains a vertex; the first keeps its hole.
        let Ok(Geometry::Ok(done)) = corner_in_path(&e, 6, &CornerOp::Chamfer(1.0, 1.0)) else {
            panic!("cuts")
        };
        let parts = area_parts(&done.shape);
        assert_eq!(parts[0], area_parts(&e)[0]);
        let Shape::Polygon { pts, .. } = &parts[1] else {
            panic!()
        };
        assert_eq!(pts.len(), 5);
        assert!(corner_in_path(&e, 8, &CornerOp::Radius(1.0)).is_err());
    }
}
