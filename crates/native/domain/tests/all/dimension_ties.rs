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
        path: Vec::new(),
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
        d.refs = Some(Box::new(DimensionRefs {
            a: Some(tie(doc, wall, vertex(0))),
            b: Some(tie(doc, wall, vertex(1))),
            c: None,
        }));
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

/// A door block: a line (0,0)–(2,0); an insert of it at (20,10); a dimension from the line's start to its end.
fn door_drawing() -> (Document, Slot, Slot) {
    let mut doc = doc();
    let block: kentos_domain::contracts::BlockDefinition = serde_json::from_value(json!({
        "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "name": "Kapı", "base": {"x": 0, "y": 0},
        "entities": [{"kind":"line","id":1,"layerId":"0","attrs":{},"a":{"x":0,"y":0},"b":{"x":2,"y":0}}]
    }))
    .expect("a block");
    doc.add_block(block).expect("added");
    let ins = doc
        .add(entity(json!({"kind":"insert","id":0,"layerId":"a","attrs":{},
            "block":"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002","p":{"x":20,"y":10},"scale":1,"rotation":0})))
        .expect("slot");
    let mut d = entity(
        json!({"kind":"dimension","id":0,"layerId":"a","attrs":{},"a":{"x":20,"y":10},"b":{"x":22,"y":10},
        "offset":1.0,"height":0.25}),
    );
    if let Entity::Dimension(d) = &mut d {
        let piece = |place| SubRef {
            path: vec![0],
            ..tie(&doc, ins, place)
        };
        d.refs = Some(Box::new(DimensionRefs {
            a: Some(piece(SubPlace::Start)),
            b: Some(piece(SubPlace::Along {
                part: 0,
                ring: 0,
                index: 0,
                t: 0.5,
            })),
            c: None,
        }));
    }
    let d = doc.add(d).expect("slot");
    (doc, ins, d)
}

#[test]
fn a_moved_insert_takes_the_dimension_on_its_piece_along() {
    let (mut doc, ins, d) = door_drawing();
    let mut moved = doc.get(ins).expect("known").clone();
    if let Entity::Insert(i) = &mut moved {
        i.p = Vec2 { x: 25.0, y: 12.0 };
    }
    assert!(doc.update(ins, moved));
    let after = dim(&doc, d);
    assert_eq!(after.a, Vec2 { x: 25.0, y: 12.0 });
    assert_eq!(
        after.b,
        Vec2 { x: 26.0, y: 12.0 },
        "the line's middle, t 0.5"
    );
    assert!(after.refs.as_ref().unwrap().each().all(|(_, r)| r.live()));
    doc.undo();
    assert_eq!(dim(&doc, d).a, Vec2 { x: 20.0, y: 10.0 });
}

#[test]
fn a_redefined_block_moves_its_pieces_and_a_removed_piece_breaks() {
    let (mut doc, _, d) = door_drawing();
    let mut wider = doc.blocks()[0].as_ref().clone();
    if let Entity::Line(l) = &mut wider.entities[0] {
        l.b = Vec2 { x: 4.0, y: 0.0 };
    }
    assert!(doc.update_block(wider.clone()).expect("redefined"));
    let after = dim(&doc, d);
    assert_eq!(
        after.b,
        Vec2 { x: 22.0, y: 10.0 },
        "the wider door's middle"
    );
    assert_eq!(after.a, Vec2 { x: 20.0, y: 10.0 });
    // A definition without the line: the pieces are gone, the points stay.
    let mut empty = wider;
    empty.entities = vec![entity(
        json!({"kind":"point","id":1,"layerId":"0","attrs":{},"p":{"x":9,"y":9}}),
    )];
    assert!(doc.update_block(empty).expect("redefined"));
    let refs = dim(&doc, d).refs.unwrap();
    assert_eq!(
        refs.a.as_ref().unwrap().broken.as_deref(),
        Some("kind_changed")
    );
    assert_eq!(dim(&doc, d).b, Vec2 { x: 22.0, y: 10.0 });
}
