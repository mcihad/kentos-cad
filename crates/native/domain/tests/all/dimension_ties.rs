//! Associative dimensions (docs/adr/0249 §3): a dimension's points follow the
//! places they name before a step is recorded, in the same undo step; a
//! reference that cannot follow breaks and keeps its point; a point the
//! dimension's own edit moved lets its reference go.

use kentos_domain::contracts::{DimensionRefs, Entity, EntityId, SubPlace, SubRef, Vec2};
use kentos_domain::{Document, Slot};
use serde_json::json;

fn doc() -> Document {
    let text = r#"{"format":"kentos.document","version":1,"name":"Ölçü","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
        "layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
    Document::from_snapshot(
        kentos_domain::contracts::DocumentSnapshotV1::from_json(text).expect("reads"),
    )
    .expect("opens")
}

fn entity(v: serde_json::Value) -> Entity {
    serde_json::from_value(v).expect("an object")
}

/// A wall: an open polyline (0,0) (10,0) (10,5).
fn wall() -> Entity {
    entity(
        json!({"kind":"polyline","id":0,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":5}],
        "closed":false}),
    )
}

fn vertex(index: u32) -> SubPlace {
    SubPlace::Vertex {
        part: 0,
        ring: 0,
        index,
    }
}

fn tie(doc: &Document, of: Slot, place: SubPlace) -> SubRef {
    SubRef {
        uid: EntityId(*doc.uid(of).expect("known").as_bytes()),
        place,
        broken: None,
    }
}

/// An aligned dimension from vertex 0 to vertex 1 of the wall.
fn dimension(doc: &Document, wall: Slot) -> Entity {
    let mut d = entity(
        json!({"kind":"dimension","id":0,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":10,"y":0},
        "offset":1.0,"height":0.25}),
    );
    if let Entity::Dimension(d) = &mut d {
        d.refs = Some(DimensionRefs {
            a: Some(tie(doc, wall, vertex(0))),
            b: Some(tie(doc, wall, vertex(1))),
            c: None,
        });
    }
    d
}

fn dim(doc: &Document, slot: Slot) -> kentos_domain::contracts::DimensionEntity {
    match doc.get(slot) {
        Some(Entity::Dimension(d)) => d.clone(),
        other => panic!("not a dimension: {other:?}"),
    }
}

fn wall_with(doc: &Document, slot: Slot, edit: impl FnOnce(&mut Vec<Vec2>)) -> Entity {
    let mut e = doc.get(slot).expect("known").clone();
    if let Entity::Polyline(p) = &mut e {
        edit(&mut p.pts);
    }
    e
}

#[test]
fn a_dragged_vertex_takes_the_dimension_along_in_the_same_step() {
    let mut doc = doc();
    let w = doc.add(wall()).expect("slot");
    let d = doc.add(dimension(&doc, w)).expect("slot");
    let moved = wall_with(&doc, w, |p| p[1] = Vec2 { x: 12.0, y: 0.0 });
    assert!(doc.update(w, moved));
    let after = dim(&doc, d);
    assert_eq!(after.b, Vec2 { x: 12.0, y: 0.0 });
    assert_eq!(after.a, Vec2 { x: 0.0, y: 0.0 });
    assert!(after.refs.as_ref().unwrap().each().all(|(_, r)| r.live()));
    // One undo takes the wall and the dimension back.
    doc.undo();
    assert_eq!(dim(&doc, d).b, Vec2 { x: 10.0, y: 0.0 });
    doc.redo();
    assert_eq!(dim(&doc, d).b, Vec2 { x: 12.0, y: 0.0 });
}

#[test]
fn an_inserted_vertex_rebinds_the_place_by_its_coordinates() {
    let mut doc = doc();
    let w = doc.add(wall()).expect("slot");
    let d = doc.add(dimension(&doc, w)).expect("slot");
    let more = wall_with(&doc, w, |p| p.insert(1, Vec2 { x: 5.0, y: 0.0 }));
    assert!(doc.update(w, more));
    let refs = dim(&doc, d).refs.unwrap();
    assert_eq!(refs.a.as_ref().unwrap().place, vertex(0));
    assert_eq!(refs.b.as_ref().unwrap().place, vertex(2));
    assert!(refs.b.unwrap().live());
}

#[test]
fn a_deleted_vertex_breaks_the_reference_and_keeps_the_point() {
    let mut doc = doc();
    let w = doc.add(wall()).expect("slot");
    let d = doc.add(dimension(&doc, w)).expect("slot");
    let fewer = wall_with(&doc, w, |p| {
        p.remove(1);
    });
    assert!(doc.update(w, fewer));
    let after = dim(&doc, d);
    assert_eq!(after.b, Vec2 { x: 10.0, y: 0.0 });
    let refs = after.refs.unwrap();
    assert_eq!(refs.b.unwrap().broken.as_deref(), Some("vertex_gone"));
    assert!(refs.a.unwrap().live());
}

#[test]
fn a_removed_object_breaks_both_references() {
    let mut doc = doc();
    let w = doc.add(wall()).expect("slot");
    let d = doc.add(dimension(&doc, w)).expect("slot");
    assert_eq!(doc.remove(&[w]), 1);
    let refs = dim(&doc, d).refs.unwrap();
    assert!(
        refs.each()
            .all(|(_, r)| r.broken.as_deref() == Some("object_gone"))
    );
    doc.undo();
    assert!(dim(&doc, d).refs.unwrap().each().all(|(_, r)| r.live()));
}

#[test]
fn a_dimension_moved_by_itself_lets_the_moved_point_go() {
    let mut doc = doc();
    let w = doc.add(wall()).expect("slot");
    let d = doc.add(dimension(&doc, w)).expect("slot");
    let mut own = dim(&doc, d);
    own.b = Vec2 { x: 8.0, y: 0.0 };
    assert!(doc.update(d, Entity::Dimension(own)));
    let refs = dim(&doc, d).refs.unwrap();
    assert!(refs.b.is_none());
    assert!(refs.a.unwrap().live());
    // Both points moved: no reference is left, the field goes.
    let mut both = dim(&doc, d);
    both.a = Vec2 { x: 1.0, y: 0.0 };
    assert!(doc.update(d, Entity::Dimension(both)));
    assert!(dim(&doc, d).refs.is_none());
}
