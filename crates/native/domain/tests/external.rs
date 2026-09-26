//! Changes from outside the drawing (`apply_external`, docs/adr/0040): the
//! web's `applyExternal` rules, which the shared fixtures leave out
//! (fixtures/document-ops/README.md): not an edit, not undoable, and the undo
//! steps of the objects they change are dropped.

use kentos_domain::contracts::{
    DocumentSnapshotV1, Entity, EntityBase, LayerNode, PointEntity, Vec2,
};
use kentos_domain::{Changes, Document, External, ExternalMeta, Slot, Uuid};

/// A small drawing: group g (layers a, b) and layer c, no objects.
fn empty() -> Document {
    let text = r#"{"format":"kentos.document","version":1,"name":"Deneme","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
        "layers":[{"id":"g","name":"G","type":"group","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[
        {"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]},
        {"id":"b","name":"B","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}]},
        {"id":"c","name":"C","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"b","entities":[],"styles":{"items":[],"categories":[]}}"#;
    Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
}

fn point(layer: &str, x: f64) -> Entity {
    Entity::Point(PointEntity {
        base: EntityBase {
            id: 0,
            layer_id: layer.into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
        },
        p: Vec2 { x, y: 0.0 },
        z: None,
    })
}

fn x_of(doc: &Document, slot: Slot) -> f64 {
    match doc.get(slot) {
        Some(Entity::Point(p)) => p.p.x,
        other => panic!("not a point: {other:?}"),
    }
}

/// A drawing with two saved points, as a project opened from the server is.
fn saved_two() -> (Document, Slot, Slot) {
    let mut doc = empty();
    let a = doc.add(point("a", 1.0)).unwrap();
    let b = doc.add(point("a", 2.0)).unwrap();
    let revision = doc.revision();
    doc.mark_saved(revision);
    (doc, a, b)
}

#[test]
fn another_editors_objects_come_in_without_an_undo_step_or_an_unsaved_mark() {
    let (mut doc, a, _) = saved_two();
    let uid = doc.uid(a).unwrap();
    let fresh = Uuid::now_v7();
    let (undo_before, revision, generation) = (doc.can_undo(), doc.revision(), doc.generation());
    let mark = doc.change_mark();
    doc.apply_external(External {
        put: vec![(uid, point("a", 10.0)), (fresh, point("b", 20.0))],
        remove: vec![Uuid::now_v7()],
        meta: None,
    })
    .unwrap();
    // The known object keeps its slot; the new one takes the next slot.
    assert_eq!(x_of(&doc, a), 10.0);
    let added = doc.slot_of(fresh).unwrap();
    assert_eq!((added.0, x_of(&doc, added)), (3, 20.0));
    assert_eq!(doc.get(added).unwrap().base().id, 3);
    // Not an edit of this user's: still saved, same revision, nothing to undo for it.
    assert!(!doc.is_dirty());
    assert_eq!(doc.revision(), revision);
    // But what the drawing shows changed: readers that redraw key on the generation.
    assert!(doc.generation() > generation);
    assert_eq!(doc.can_undo(), undo_before);
    // Readers that follow the drawing see it like any change.
    assert_eq!(doc.changes_since(mark), Changes::Slots(&[a, added]));
}

fn moved(doc: &mut Document, slot: Slot, x: f64) {
    let mut e = doc.get(slot).unwrap().clone();
    if let Entity::Point(p) = &mut e {
        p.p.x = x;
    }
    assert!(doc.update(slot, e));
}

#[test]
fn the_undo_steps_of_the_objects_it_changes_are_dropped() {
    let (mut doc, a, b) = saved_two();
    moved(&mut doc, a, 5.0);
    moved(&mut doc, b, 6.0);
    let uid = doc.uid(a).unwrap();
    doc.apply_external(External {
        put: vec![(uid, point("a", 50.0))],
        ..External::default()
    })
    .unwrap();
    // Undo takes back this user's own edit of b; a keeps what came from outside.
    assert!(doc.undo().is_some());
    assert_eq!((x_of(&doc, a), x_of(&doc, b)), (50.0, 2.0));
    // What is left touches b only: its adding. Adding and moving a went with a's new state.
    assert!(doc.undo().is_some());
    assert!(doc.get(b).is_none() && doc.get(a).is_some());
    assert!(!doc.can_undo());
}

#[test]
fn an_object_deleted_here_and_brought_back_elsewhere_loses_its_undo() {
    let (mut doc, a, _) = saved_two();
    let uid = doc.uid(a).unwrap();
    doc.remove(&[a]);
    doc.apply_external(External {
        put: vec![(uid, point("a", 7.0))],
        ..External::default()
    })
    .unwrap();
    // It comes back under its id in a new slot; undoing the deletion would add it twice.
    let back = doc.slot_of(uid).unwrap();
    assert_ne!(back, a);
    assert_eq!(x_of(&doc, back), 7.0);
    // Undoing everything left takes back b's adding only; the object that came back stays.
    while doc.undo().is_some() {}
    assert_eq!(doc.slot_of(uid), Some(back));
    assert_eq!(doc.len(), 1);
}

#[test]
fn removed_objects_go_and_unknown_ones_are_skipped() {
    let (mut doc, a, b) = saved_two();
    let uid = doc.uid(b).unwrap();
    doc.apply_external(External {
        remove: vec![uid, Uuid::now_v7()],
        ..External::default()
    })
    .unwrap();
    assert!(doc.get(b).is_none() && doc.get(a).is_some());
    assert_eq!(doc.slot_of(uid), None);
    assert!(!doc.is_dirty());
}

#[test]
fn a_bad_change_or_an_open_edit_changes_nothing() {
    let (mut doc, a, _) = saved_two();
    let uid = doc.uid(a).unwrap();
    let twice = External {
        put: vec![(uid, point("a", 9.0))],
        remove: vec![uid],
        meta: None,
    };
    assert!(doc.apply_external(twice).unwrap_err().contains("iki kez"));
    let nil = External {
        put: vec![(Uuid::nil(), point("a", 9.0))],
        ..External::default()
    };
    assert!(doc.apply_external(nil).unwrap_err().contains("nil"));
    let group = doc.begin_group("Taşı");
    let later = External {
        put: vec![(uid, point("a", 9.0))],
        ..External::default()
    };
    assert!(
        doc.apply_external(later.clone())
            .unwrap_err()
            .contains("Açık bir düzenleme")
    );
    doc.end_group(group);
    assert_eq!(x_of(&doc, a), 1.0);
    doc.apply_external(later).unwrap();
    assert_eq!(x_of(&doc, a), 9.0);
}

#[test]
fn new_metadata_keeps_this_users_active_layer_when_it_can() {
    let (mut doc, _, _) = saved_two();
    let mut layers: Vec<LayerNode> = doc.layers().nodes().to_vec();
    layers[1].name = "Çizim (yeni ad)".into();
    doc.apply_external(External {
        meta: Some(ExternalMeta {
            name: Some("Ada 5".into()),
            layers: Some(layers.clone()),
            ..ExternalMeta::default()
        }),
        ..External::default()
    })
    .unwrap();
    assert_eq!(doc.name(), "Ada 5");
    assert_eq!(doc.layers().get("c").unwrap().name, "Çizim (yeni ad)");
    assert_eq!(doc.layers().active(), "b");
    assert!(!doc.is_dirty());
    // A tree without this user's active layer: the first layer becomes active.
    let only_c: Vec<LayerNode> = layers.into_iter().skip(1).collect();
    doc.apply_external(External {
        meta: Some(ExternalMeta {
            layers: Some(only_c),
            ..ExternalMeta::default()
        }),
        ..External::default()
    })
    .unwrap();
    assert_eq!(doc.layers().active(), "c");
}

#[test]
fn a_change_from_outside_on_an_unsaved_drawing_leaves_it_unsaved() {
    let (mut doc, a, _) = saved_two();
    doc.add(point("a", 3.0)).unwrap();
    assert!(doc.is_dirty());
    let uid = doc.uid(a).unwrap();
    doc.apply_external(External {
        put: vec![(uid, point("a", 4.0))],
        ..External::default()
    })
    .unwrap();
    assert!(doc.is_dirty());
    // Its own edit is still there to undo.
    assert!(doc.undo().is_some());
    assert_eq!(doc.len(), 2);
}

#[test]
fn a_file_of_the_same_drawing_is_unchanged_by_its_equal_copy() {
    let snapshot =
        DocumentSnapshotV1::from_json(include_str!("../../../../fixtures/document/v1/sample.json"))
            .unwrap();
    let mut doc = Document::from_snapshot(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let same: Vec<(Uuid, Entity)> = doc
        .entities()
        .map(|e| (doc.uid(Slot(e.base().id)).unwrap(), e.clone()))
        .collect();
    let mark = doc.change_mark();
    let generation = doc.generation();
    doc.apply_external(External {
        put: same,
        ..External::default()
    })
    .unwrap();
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.generation(), generation);
    // Nothing changed, so nothing is reported changed.
    assert_eq!(doc.changes_since(mark), Changes::Slots(&[]));
}

#[test]
fn every_edit_moves_the_generation_with_the_revision() {
    let (mut doc, a, _) = saved_two();
    let g = doc.generation();
    moved(&mut doc, a, 8.0);
    assert_eq!(doc.generation(), g + 1);
    doc.undo();
    doc.toggle_layer_visible("c");
    assert_eq!(doc.generation(), g + 3);
    // Folding a group or choosing the active layer is not a change of the drawing.
    doc.set_layer_expanded("g", false);
    doc.set_active_layer("a");
    assert_eq!(doc.generation(), g + 3);
}
