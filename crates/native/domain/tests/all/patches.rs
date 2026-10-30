//! A job's patch written into the document it was copied from (docs/adr/0253 §9): the objects it added keep the copy's
//! slots, and the document's next slot comes after them (docs/adr/0757: a bind's 50 000 parcels left the next drawn
//! object on the first parcel's slot).

use kentos_domain::Document;
use kentos_domain::contracts::{DocumentSnapshotV1, Entity, EntityBase, PointEntity, Vec2};

fn drawing() -> Document {
    let text = r#"{"format":"kentos.document","version":1,"name":"Deneme","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
        "layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
    Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
}

fn point(x: f64) -> Entity {
    Entity::Point(PointEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: "a".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: Vec2 { x, y: 0.0 },
        z: None,
        parts: None,
    })
}

fn x_of(doc: &Document, slot: kentos_domain::Slot) -> Option<f64> {
    match doc.get(slot) {
        Some(Entity::Point(p)) => Some(p.p.x),
        _ => None,
    }
}

#[test]
fn objects_a_patch_adds_move_the_next_slot_on() {
    let mut doc = drawing();
    let kept = doc.add_many(vec![point(1.0)], "Önce").expect("adds");
    let base = (doc.revision(), doc.generation());
    let mut copy = doc.reading_copy();
    let added = copy
        .add_many(vec![point(2.0), point(3.0)], "İş")
        .expect("the job adds");
    let patch = copy.take_patch();
    doc.apply_patch(patch, base).expect("written");
    assert_eq!(
        doc.next_slot(),
        copy.next_slot(),
        "the next slot after the job's"
    );
    // What comes next takes a new slot: the job's objects stay.
    let next = doc.add_many(vec![point(4.0)], "Sonra").expect("adds");
    assert!(
        !added.contains(&next[0]) && !kept.contains(&next[0]),
        "{next:?} among {added:?}"
    );
    assert_eq!(x_of(&doc, added[0]), Some(2.0));
    assert_eq!(x_of(&doc, added[1]), Some(3.0));
    assert_eq!(x_of(&doc, next[0]), Some(4.0));
    assert_eq!(doc.len(), 4);
    // Undone and done again, the same.
    assert!(doc.undo().is_some());
    assert!(doc.undo().is_some());
    assert_eq!(doc.len(), 1);
    assert!(doc.redo().is_some());
    assert!(doc.redo().is_some());
    assert_eq!(doc.len(), 4);
}
