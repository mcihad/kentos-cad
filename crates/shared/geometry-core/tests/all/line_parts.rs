//! Multi-part polylines and multi-point objects in the core (docs/adr/0174
//! §3): measured, drawn, picked, snapped and packed part by part; grips,
//! transforms, offsets and explosions take every part; the edits that run
//! along one path refuse them.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::{
    Entity, MULTI_PART_REFUSED, Part, PointPart, Shape, area_parts, entity_anchor, entity_bounds,
    entity_length, entity_vertices, is_multi_part,
};
use kentos_geometry_core::geom::affine::translation;
use kentos_geometry_core::geom::centroid::shape_centroid;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::curve_cuts::{Cut, Geometry};
use kentos_geometry_core::store::draw::{LINE, MARKERS};
use kentos_geometry_core::store::snap::SnapKind;
use kentos_geometry_core::store::{Packer, Store};
use kentos_geometry_core::text::Font;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

/// A road of two strips: (0,0)→(10,0)→(10,5) and, 20 m east, (30,0)→(36,0).
fn road() -> Shape {
    Shape::Polyline {
        pts: vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 5.0)],
        bulges: None,
        holes: None,
        parts: Some(vec![Part {
            pts: vec![v(30.0, 0.0), v(36.0, 0.0)],
            bulges: None,
            holes: None,
        }]),
    }
}

/// Three survey marks: (0,0) at 100 m, (6,0) and (0,9) at 102 m.
fn marks() -> Shape {
    Shape::Point {
        p: v(0.0, 0.0),
        z: Some(100.0),
        parts: Some(vec![
            PointPart {
                p: v(6.0, 0.0),
                z: None,
            },
            PointPart {
                p: v(0.0, 9.0),
                z: Some(102.0),
            },
        ]),
    }
}

#[test]
fn a_multi_part_object_is_measured_part_by_part() {
    let r = road();
    assert!(is_multi_part(&r) && is_multi_part(&marks()));
    assert_eq!(entity_length(&r), Some(15.0 + 6.0));
    assert_eq!(entity_vertices(&r).len(), 5);
    let b = entity_bounds(&r);
    assert_eq!((b.min_x, b.min_y, b.max_x, b.max_y), (0.0, 0.0, 36.0, 5.0));
    // The label goes on the longest part: the first (15 m) here, its middle vertex.
    assert_eq!(entity_anchor(&r), Some(v(10.0, 0.0)));
    let Shape::Polyline { pts, parts, .. } = road() else {
        unreachable!()
    };
    let long_second = Shape::Polyline {
        pts: vec![pts[0], v(2.0, 0.0)],
        bulges: None,
        holes: None,
        parts,
    };
    assert_eq!(entity_anchor(&long_second), Some(v(36.0, 0.0)));
    // The parts come back as one-part objects of the same kind, the first the object's own fields.
    let m = marks();
    let parts = area_parts(&m);
    assert_eq!(parts.len(), 3);
    assert!(
        matches!(parts[2], Shape::Point { p, z: Some(z), parts: None } if p == v(0.0, 9.0) && z == 102.0)
    );
    // A multi-point object's centre is its points' mean; it has no length.
    assert_eq!(shape_centroid(&marks()), Some(v(2.0, 3.0)));
    assert_eq!(entity_length(&marks()), None);
    assert_eq!(entity_vertices(&marks()).len(), 3);
}

#[test]
fn the_store_draws_picks_snaps_selects_and_packs_every_part() {
    let mut store = Store::new();
    store.put(1.0, "a", false, road());
    store.put(2.0, "a", false, marks());
    // One record each: the road's two paths with their points, the marks' three points.
    let d = store.drawn(&[1.0], true, None);
    assert_eq!(
        d,
        vec![
            LINE, 2.0, 0.0, 3.0, 0.0, 0.0, 10.0, 0.0, 10.0, 5.0, 0.0, 2.0, 30.0, 0.0, 36.0, 0.0
        ]
    );
    let d = store.drawn(&[2.0], false, None);
    assert_eq!(d, vec![MARKERS, 3.0, 0.0, 0.0, 6.0, 0.0, 0.0, 9.0]);
    // Near the second strip, the road; near the third mark, the marks.
    assert_eq!(store.hit(v(33.0, 0.05), 0.1), Some(1.0));
    assert_eq!(store.hit(v(0.05, 9.0), 0.1), Some(2.0));
    assert_eq!(store.hit(v(20.0, 0.0), 0.1), None);
    // The second strip's end and the second mark snap.
    let end = store
        .snap(v(36.05, 0.05), 0.2, SnapKind::Endpoint.bit(), None)
        .expect("an end");
    assert_eq!(end.point, v(36.0, 0.0));
    let node = store
        .snap(v(6.05, 0.05), 0.2, SnapKind::Node.bit(), None)
        .expect("a mark");
    assert_eq!((node.point, node.id), (v(6.0, 0.0), 2.0));
    // A window must hold every part; a crossing window catches one.
    let r = |x0: f64, x1: f64, y1: f64| Bounds {
        min_x: x0,
        min_y: -1.0,
        max_x: x1,
        max_y: y1,
    };
    assert_eq!(store.in_rect(&r(29.0, 37.0, 1.0), true), vec![1.0]);
    assert!(store.in_rect(&r(29.0, 37.0, 1.0), false).is_empty());
    assert_eq!(store.in_rect(&r(-1.0, 37.0, 10.0), false), vec![1.0, 2.0]);
    assert_eq!(store.in_rect(&r(-1.0, 1.0, 10.0), true), vec![1.0, 2.0]);
    // A fence over the second strip alone; one past the third mark alone.
    assert_eq!(
        store.in_fence(&[v(33.0, -1.0), v(33.0, 1.0)], 0.1),
        vec![1.0]
    );
    assert_eq!(
        store.in_fence(&[v(-1.0, 9.05), v(1.0, 9.05)], 0.1),
        vec![2.0]
    );
    // Packed and read back as they were (kinds 16 and 17).
    let mut out = Packer::default();
    out.object(1.0, "a", false, &road());
    let kind = out.nums[3];
    out.object(2.0, "a", false, &marks());
    let mut back = Store::new();
    back.put_packed(&out.nums, &out.strings).expect("reads");
    assert_eq!(kind, 16.0);
    assert_eq!(back.get(1.0).map(|it| &it.shape), Some(&road()));
    assert_eq!(back.get(2.0).map(|it| &it.shape), Some(&marks()));
}

#[test]
fn grips_and_moves_take_every_part() {
    use kentos_geometry_core::ops::grips::{entity_grips, move_grip};
    use kentos_geometry_core::ops::transform::transform_entity;
    // The first strip's 3 vertices and 2 middles, then the second's 2 and 1.
    let g = entity_grips(&road());
    assert_eq!(g.len(), 8);
    assert_eq!(g[5], v(30.0, 0.0));
    assert_eq!(g[7], v(33.0, 0.0));
    // The second strip's first vertex moves; the first strip stays.
    let moved = move_grip(&Entity::new(road()), 5, v(31.0, 1.0)).expect("moves");
    let parts = area_parts(&moved.shape);
    let r = road();
    assert_eq!(parts[0], area_parts(&r)[0]);
    assert!(matches!(&parts[1], Shape::Polyline { pts, .. } if pts[0] == v(31.0, 1.0)));
    // The second strip's middle becomes a vertex there.
    let bent = move_grip(&Entity::new(road()), 7, v(33.0, 2.0)).expect("bends");
    assert_eq!(entity_vertices(&bent.shape).len(), 6);
    // Each mark is a grip of its own; the third moves with its elevation.
    assert_eq!(
        entity_grips(&marks()),
        vec![v(0.0, 0.0), v(6.0, 0.0), v(0.0, 9.0)]
    );
    let moved = move_grip(&Entity::new(marks()), 2, v(1.0, 9.0)).expect("moves");
    assert!(
        matches!(&area_parts(&moved.shape)[2], Shape::Point { p, z: Some(z), .. } if *p == v(1.0, 9.0) && *z == 102.0)
    );
    // A move takes every part and every mark.
    let moved = transform_entity(&Entity::new(road()), &translation(100.0, 0.0));
    assert_eq!(entity_bounds(&moved.shape).max_x, 136.0);
    let moved = transform_entity(&Entity::new(marks()), &translation(0.0, -9.0));
    assert_eq!(
        entity_vertices(&moved.shape),
        vec![v(0.0, -9.0), v(6.0, -9.0), v(0.0, 0.0)]
    );
}

#[test]
fn an_offset_an_explosion_and_a_reversal_take_every_part() {
    use kentos_geometry_core::ops::explode::explode_entity;
    use kentos_geometry_core::ops::offset::offset_entity;
    use kentos_geometry_core::ops::reshape::reverse;
    // From below the first strip (right of its travel), both strips go 1 m right of theirs: down.
    let Geometry::Ok(out) = offset_entity(&road(), 1.0, v(5.0, -0.5)) else {
        panic!("offsets")
    };
    let parts = area_parts(&out.shape);
    assert_eq!(parts.len(), 2);
    assert!(
        matches!(&parts[1], Shape::Polyline { pts, .. } if pts == &vec![v(30.0, -1.0), v(36.0, -1.0)])
    );
    assert!(
        matches!(&parts[0], Shape::Polyline { pts, .. } if pts[0] == v(0.0, -1.0) && pts[1] == v(11.0, -1.0))
    );
    // Every strip's edges; every mark a point.
    let Cut::Pieces(pieces) = explode_entity(&road(), "", Font::DEFAULT) else {
        panic!("explodes")
    };
    assert_eq!(pieces.len(), 3);
    assert!(
        matches!(pieces[2].shape, Shape::Line { a, b } if a == v(30.0, 0.0) && b == v(36.0, 0.0))
    );
    let Cut::Pieces(points) = explode_entity(&marks(), "", Font::DEFAULT) else {
        panic!("comes apart")
    };
    assert_eq!(points.len(), 3);
    assert!(points.iter().all(|e| !is_multi_part(&e.shape)));
    assert!(matches!(points[2].shape, Shape::Point { z: Some(z), .. } if z == 102.0));
    // Reversed: each strip runs the other way, the strips stay in order.
    let back = reverse(&Entity::new(road())).expect("reverses");
    let parts = area_parts(&back.shape);
    assert!(matches!(&parts[0], Shape::Polyline { pts, .. } if pts[0] == v(10.0, 5.0)));
    assert!(matches!(&parts[1], Shape::Polyline { pts, .. } if pts[0] == v(36.0, 0.0)));
}

#[test]
fn edits_that_run_along_one_path_refuse_a_multi_part_polyline() {
    use kentos_geometry_core::ops::breaking::break_entity;
    use kentos_geometry_core::ops::continuation::path_ends;
    use kentos_geometry_core::ops::join::join_entities;
    use kentos_geometry_core::ops::lengthen::lengthen_entity;
    use kentos_geometry_core::ops::reshape_by::{ReshapeRefusal, reshape};
    use kentos_geometry_core::ops::split::split_equal;
    use kentos_geometry_core::ops::trim::{extend_entity, trim_entity};
    let e = Entity::new(road());
    let refused = |c: Cut| matches!(c, Cut::Error(m) if m == MULTI_PART_REFUSED);
    assert!(refused(break_entity(&e, v(31.0, 0.0), v(32.0, 0.0))));
    assert!(refused(trim_entity(&e, v(31.0, 0.0), &[])));
    assert!(
        matches!(extend_entity(&e, v(35.0, 0.0), &[]), Geometry::Error(m) if m == MULTI_PART_REFUSED)
    );
    assert!(
        matches!(lengthen_entity(&e, true, 30.0), Geometry::Error(m) if m == MULTI_PART_REFUSED)
    );
    assert!(split_equal(&e, 2.0).is_none());
    assert!(path_ends(&road()).is_none());
    assert_eq!(
        reshape(&road(), &[v(32.0, -1.0), v(32.0, 1.0)]),
        Err(ReshapeRefusal::MultiPart)
    );
    // Birleştir leaves it out, and joins the others.
    let line = |a: Vec2, b: Vec2, id: f64| {
        let mut e = Entity::new(Shape::Line { a, b });
        e.rest
            .push(("id".into(), kentos_geometry_core::api::json::Json::Num(id)));
        e
    };
    let mut road = Entity::new(road());
    road.rest
        .push(("id".into(), kentos_geometry_core::api::json::Json::Num(1.0)));
    let joined = join_entities(
        &[
            road,
            line(v(36.0, 0.0), v(40.0, 0.0), 2.0),
            line(v(40.0, 0.0), v(40.0, 3.0), 3.0),
        ],
        1e-6,
    )
    .expect("joins");
    assert_eq!(joined.groups.len(), 1);
    assert_eq!(joined.skipped.len(), 1);
}

#[test]
fn toplu_alan_takes_a_part_as_a_path_of_its_own() {
    use kentos_geometry_core::ops::polygonize::polygonize;
    // Two strips meeting end to end close nothing, and where they meet is no free end.
    let corner = Shape::Polyline {
        pts: vec![v(0.0, 0.0), v(10.0, 0.0)],
        bulges: None,
        holes: None,
        parts: Some(vec![Part {
            pts: vec![v(10.0, 0.0), v(10.0, 10.0)],
            bulges: None,
            holes: None,
        }]),
    };
    let found = polygonize(&[Entity::new(corner)], &[], false);
    assert!(found.regions.is_empty());
    assert_eq!(found.free_ends, vec![v(0.0, 0.0), v(10.0, 10.0)]);
}
