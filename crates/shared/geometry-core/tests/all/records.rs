//! A record without a geometry in the store (docs/adr/0410 §1): kept, never drawn, picked, snapped to,
//! boxed or counted in an extent; its JSON reads back.

use kentos_geometry_core::api::json::{FromJson, Json, to_string};
use kentos_geometry_core::entity::{Entity, Shape, entity_vertices};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::polygon::PolygonMode;
use kentos_geometry_core::vec2::Vec2;

fn store() -> Store {
    let mut s = Store::new();
    s.put_json(
        r#"[
        { "kind": "line", "id": 1, "layerId": "0", "attrs": {}, "a": { "x": 0, "y": 0 }, "b": { "x": 10, "y": 0 } },
        { "kind": "record", "id": 2, "layerId": "0", "attrs": { "Ad": "Ayşe" } }
    ]"#,
    )
    .expect("the objects");
    s
}

#[test]
fn a_record_reads_and_writes_as_its_kind_alone() {
    let e = Entity::from_json(
        &Json::parse(r#"{ "kind": "record", "id": 2, "layerId": "0", "attrs": {} }"#).unwrap(),
    )
    .expect("a record");
    assert_eq!(e.shape, Shape::Record {});
    assert!(entity_vertices(&e.shape).is_empty());
    assert!(to_string(&e).contains(r#""kind":"record""#));
}

#[test]
fn a_record_is_kept_but_never_found_by_place() {
    let s = store();
    assert_eq!(s.len(), 2);
    assert!(s.get(2.0).is_some());
    // Nowhere: no hit, no snap, in no window, circle, fence or polygon.
    for p in [
        Vec2::new(0.0, 0.0),
        Vec2::new(5.0, 3.0),
        Vec2::new(f64::MAX, 0.0),
    ] {
        assert!(!s.hits(p, 1e9).contains(&2.0));
    }
    let all = Bounds {
        min_x: -1e12,
        min_y: -1e12,
        max_x: 1e12,
        max_y: 1e12,
    };
    assert_eq!(s.in_rect(&all, true), vec![1.0]);
    assert_eq!(s.in_rect(&all, false), vec![1.0]);
    assert!(!s.in_circle(Vec2::new(0.0, 0.0), 1e9, true).contains(&2.0));
    let ring = [
        Vec2::new(-1e9, -1e9),
        Vec2::new(1e9, -1e9),
        Vec2::new(1e9, 1e9),
        Vec2::new(-1e9, 1e9),
    ];
    for mode in [PolygonMode::of(0), PolygonMode::of(1), PolygonMode::of(2)] {
        assert!(!s.in_polygon(&ring, mode).contains(&2.0));
    }
    if let Some(hit) = s.snap(Vec2::new(0.0, 0.0), 1e9, u32::MAX, None) {
        assert_eq!(hit.id, 1.0);
    }
    // Its extent is none; the drawing's is the line's.
    assert_eq!(s.extent(Some(&[2.0])), None);
    let b = s.extent(None).expect("the line's");
    assert_eq!((b.min_x, b.max_x), (0.0, 10.0));
    // Drawn as nothing; its measures none.
    let drawn = s.drawn(&[2.0], false, None);
    assert!(
        drawn.iter().all(|v| v.is_nan()) || drawn.len() <= 2,
        "{drawn:?}"
    );
}
