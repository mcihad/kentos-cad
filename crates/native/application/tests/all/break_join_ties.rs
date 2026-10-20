//! Kır, Birleştir and Yönü çevir over a drawing whose objects are tied to
//! others (docs/adr/0296 §3): what the shared cases cannot set up, since their
//! drawing holds no persistent ids. A linked label text follows the object
//! that keeps its id and goes with a removed one; an associative hatch follows
//! a reversed boundary and lets go of a broken one.

use kentos_commands::{
    CommandResult, DocumentSnapshotV1, EntitiesBreak, EntitiesJoin, EntitiesReverse, Entity,
    EntityId, HatchAssoc, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{
    ExecutionContext, entities_break, entities_join, entities_reverse,
};
use serde_json::json;

/// An area (1), two lines meeting end to end (2, 3), a labelled polyline (4),
/// a text linked to 4 (5), one linked to 3 (6) and a hatch tied to 1 (7).
fn drawing() -> Document {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Bağlar","settings":{"srid":5256,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"polygon","id":1,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}]},
        {"kind":"line","id":2,"layerId":"a","attrs":{},"label":"Sol","a":{"x":0,"y":20},"b":{"x":10,"y":20}},
        {"kind":"line","id":3,"layerId":"a","attrs":{},"label":"Sağ","a":{"x":10,"y":20},"b":{"x":20,"y":20}},
        {"kind":"polyline","id":4,"layerId":"a","attrs":{},"label":"Yol","pts":[{"x":0,"y":40},{"x":10,"y":40},{"x":10,"y":50}]},
        {"kind":"text","id":5,"layerId":"a","attrs":{},"p":{"x":5,"y":40},"text":"Yol","height":2.5,"rotation":0},
        {"kind":"text","id":6,"layerId":"a","attrs":{},"p":{"x":15,"y":20},"text":"Sağ","height":2.5,"rotation":0},
        {"kind":"hatch","id":7,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}],
         "pattern":{"type":"lines","angle":0,"spacing":1.5}}],
        "styles":{"items":[],"categories":[]}});
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"),
    )
    .expect("opens");
    let id = |doc: &Document, s: u32| EntityId(doc.uid(Slot(s)).expect("known").into_bytes());
    for (text, of) in [(5, 4), (6, 3)] {
        let Some(Entity::Text(t)) = doc.get(Slot(text)).cloned() else {
            panic!("a text")
        };
        let linked = Entity::Text(kentos_commands::TextEntity {
            label_of: Some(id(&doc, of)),
            label_scale: Some(100.0),
            ..t
        });
        assert!(doc.update(Slot(text), linked));
    }
    let Some(Entity::Hatch(h)) = doc.get(Slot(7)).cloned() else {
        panic!("a hatch")
    };
    let tied = Entity::Hatch(kentos_commands::HatchEntity {
        boundary: None,
        assoc: Some(HatchAssoc {
            gap: None,
            outer: Some(id(&doc, 1)),
            bounds: Vec::new(),
            islands: Vec::new(),
            cutouts: Vec::new(),
            seed: Vec2 { x: 5.0, y: 5.0 },
        }),
        ..h
    });
    assert!(doc.update(Slot(7), tied));
    doc
}

fn uid(doc: &Document, s: u32) -> String {
    doc.uid(Slot(s)).expect("known").to_string()
}

fn completed<T: std::fmt::Debug>(r: CommandResult<T>) {
    assert!(
        matches!(r, CommandResult::Completed { .. }),
        "not completed: {r:?}"
    );
}

fn label_of(doc: &Document, s: u32) -> Option<EntityId> {
    match doc.get(Slot(s)) {
        Some(Entity::Text(t)) => t.label_of,
        other => panic!("not a text: {other:?}"),
    }
}

fn hatch_tied(doc: &Document) -> bool {
    match doc.get(Slot(7)) {
        Some(Entity::Hatch(h)) => h.assoc.is_some(),
        other => panic!("not a hatch: {other:?}"),
    }
}

#[test]
fn a_linked_text_follows_the_first_piece_of_a_break() {
    let mut doc = drawing();
    let of = label_of(&doc, 5);
    let input = EntitiesBreak {
        uid: uid(&doc, 4),
        first: Vec2 { x: 10.0, y: 40.0 },
        second: None,
        expected_revision: None,
    };
    completed(entities_break::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ));
    assert_eq!(
        label_of(&doc, 5),
        of,
        "still the polyline's, which kept its id"
    );
    doc.undo();
    assert_eq!(label_of(&doc, 5), of);
}

#[test]
fn a_joined_member_takes_its_linked_text_the_kept_one_keeps_its_own() {
    let mut doc = drawing();
    let input = EntitiesJoin {
        uids: vec![uid(&doc, 2), uid(&doc, 3)],
        tolerance: None,
        chain: None,
        expected_revision: None,
    };
    completed(entities_join::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ));
    assert!(doc.get(Slot(3)).is_none(), "joined into 2");
    assert!(doc.get(Slot(6)).is_none(), "3's label went with it");
    assert!(
        matches!(doc.get(Slot(2)), Some(Entity::Line(_))),
        "two lines in a row: one line"
    );
    doc.undo();
    assert!(
        label_of(&doc, 6).is_some(),
        "undo gives the text back, linked"
    );
}

#[test]
fn a_reversed_object_keeps_its_linked_text_and_its_hatch() {
    let mut doc = drawing();
    let of = label_of(&doc, 5);
    let input = EntitiesReverse {
        uids: vec![uid(&doc, 4), uid(&doc, 1)],
        expected_revision: None,
    };
    completed(entities_reverse::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ));
    assert_eq!(label_of(&doc, 5), of);
    assert!(
        hatch_tied(&doc),
        "the area is closed as before: the hatch follows it"
    );
}

#[test]
fn a_hatch_lets_go_of_a_broken_boundary() {
    let mut doc = drawing();
    let input = EntitiesBreak {
        uid: uid(&doc, 1),
        first: Vec2 { x: 5.0, y: 0.0 },
        second: Some(Vec2 { x: 10.0, y: 5.0 }),
        expected_revision: None,
    };
    completed(entities_break::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ));
    assert!(
        !hatch_tied(&doc),
        "the boundary is open now: the hatch stays and follows nothing"
    );
    doc.undo();
    assert!(hatch_tied(&doc));
}
