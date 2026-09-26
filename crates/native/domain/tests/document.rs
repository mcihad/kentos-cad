//! What the shared fixtures cannot say, because the web has no counterpart yet
//! or does otherwise (docs/adr/0020 “Bilinçli farklar”):
//! persistent ids (docs/adr/0014), the ADR 0003 rules the web breaks for layer
//! styles, hatch islands, undo while busy, the `.kcad` v1 round trip, and
//! the ids new layers get.

use kentos_domain::contracts::{
    DocumentSnapshotV1, Entity, EntityBase, HatchEntity, HatchPattern, HatchPatternType,
    LayerStyle, PointEntity, Vec2,
};
use kentos_domain::{Document, NewLayer, Slot, labels};

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

fn sample() -> DocumentSnapshotV1 {
    DocumentSnapshotV1::from_json(SAMPLE).expect("the web's sample file reads")
}

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
        "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
    Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
}

fn base(layer: &str) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.into(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
    }
}

fn point(layer: &str, x: f64) -> Entity {
    Entity::Point(PointEntity {
        base: base(layer),
        p: Vec2 { x, y: 0.0 },
        z: None,
    })
}

fn moved(doc: &Document, slot: Slot, x: f64) -> Entity {
    let mut entity = doc.get(slot).expect("known").clone();
    if let Entity::Point(p) = &mut entity {
        p.p.x = x;
    }
    entity
}

#[test]
fn objects_get_a_v7_uid_that_edits_undo_and_redo_keep() {
    let mut doc = empty();
    let first = doc.add(point("a", 1.0)).expect("slot");
    let second = doc.add(point("a", 2.0)).expect("slot");
    let uid = doc.uid(first).expect("uid");
    assert_eq!(uid.get_version_num(), 7);
    assert_ne!(
        uid,
        doc.uid(second).expect("uid"),
        "every object its own id"
    );
    assert_eq!(doc.slot_of(uid), Some(first));

    // Changing an object keeps its id; undo and redo bring the same one back.
    assert!(doc.update(first, moved(&doc, first, 5.0)));
    assert_eq!(doc.uid(first), Some(uid));
    doc.remove(&[first]);
    assert_eq!(doc.slot_of(uid), None);
    assert_eq!(doc.undo().as_deref(), Some(labels::REMOVE));
    assert_eq!(doc.uid(first), Some(uid));
    assert_eq!(doc.slot_of(uid), Some(first));
    // Back past the change, the second add and the first add.
    for label in [labels::CHANGE, labels::ADD, labels::ADD] {
        assert_eq!(doc.undo().as_deref(), Some(label));
    }
    assert_eq!(doc.slot_of(uid), None, "the add is undone");
    doc.redo();
    assert_eq!(doc.uid(first), Some(uid), "redo adds the same object again");
    assert_eq!(doc.get(first).map(|e| e.base().id), Some(first.0));
}

#[test]
fn objects_read_from_a_v1_file_get_uids_and_keep_their_ids_as_slots() {
    let snapshot = sample();
    let doc = Document::from_snapshot(snapshot.clone()).expect("opens");
    let mut uids = std::collections::HashSet::new();
    for entity in &snapshot.entities {
        let slot = Slot(entity.base().id);
        assert_eq!(doc.get(slot), Some(entity));
        assert!(uids.insert(doc.uid(slot).expect("uid")));
    }
}

#[test]
fn a_v1_file_round_trips_unchanged_through_the_document() {
    let snapshot = sample();
    let doc = Document::from_snapshot(snapshot.clone()).expect("opens");
    assert_eq!(doc.to_snapshot(), snapshot);
    assert_eq!(
        serde_json::to_string(&doc.to_snapshot()).expect("writes"),
        serde_json::to_string(&snapshot).expect("writes")
    );
    // Edits and their undo leave the same file (a changed object keeps its place).
    let mut doc = doc;
    let first = Slot(snapshot.entities[0].base().id);
    let entity = doc.get(first).expect("first").clone();
    doc.update(first, entity);
    doc.undo();
    assert_eq!(doc.to_snapshot(), snapshot);
}

#[test]
fn a_file_the_document_cannot_hold_is_refused_with_the_reason() {
    let mut snapshot = sample();
    snapshot.entities[1] = snapshot.entities[0].clone();
    let error = Document::from_snapshot(snapshot).expect_err("repeated id");
    assert!(
        error.starts_with("Nesne 2 ") && error.contains("benzersiz"),
        "{error}"
    );

    let mut snapshot = sample();
    if let Entity::Point(p) = &mut snapshot.entities[0] {
        p.base.layer_id = "yok".into();
    }
    let error = Document::from_snapshot(snapshot).expect_err("unknown layer");
    assert!(error.contains("“yok” katmanı dosyada yok"), "{error}");

    let mut snapshot = sample();
    snapshot.layers.clear();
    snapshot.entities.clear();
    let error = Document::from_snapshot(snapshot).expect_err("no layer");
    assert!(error.contains("en az bir katman"), "{error}");
}

#[test]
fn the_active_layer_falls_back_to_the_first_layer_as_on_the_web() {
    let mut snapshot = sample();
    snapshot.active_layer = "layer-g".into();
    let doc = Document::from_snapshot(snapshot).expect("opens");
    assert_eq!(doc.layers().active(), "parsel");
}

/// ADR 0003: a failed transaction leaves the dirty flag and the revision as
/// they were. The web breaks this for layer styles: its layer store marks the
/// drawing edited on every style change, the revert included.
#[test]
fn a_failed_transaction_with_a_layer_style_leaves_the_drawing_clean() {
    let mut doc = empty();
    let revision = doc.revision();
    let red = LayerStyle {
        color: "#FF0000".into(),
        ..doc.layers().get("a").expect("a").style.clone()
    };
    let result: Result<(), &str> = doc.transact("Yarım", |doc| {
        doc.set_layer_style("a", red.clone(), labels::LAYER_STYLE);
        Err("kural ihlali")
    });
    assert_eq!(result, Err("kural ihlali"));
    assert_eq!(doc.layers().get("a").expect("a").style.color, "fg");
    assert!(!doc.is_dirty());
    assert_eq!(doc.revision(), revision);
    assert!(!doc.can_undo());

    // A cancelled group: the same.
    let group = doc.begin_group("Model");
    doc.set_layer_style("a", red, labels::LAYER_STYLE);
    assert!(
        !doc.is_dirty(),
        "inside a group nothing is an edit before it ends"
    );
    doc.cancel_group(group);
    assert_eq!(doc.layers().get("a").expect("a").style.color, "fg");
    assert!(!doc.is_dirty());
    assert_eq!(doc.revision(), revision);
}

/// Only a polyline drops holes on an update; the web drops a hatch's islands
/// too (`updateOp` deletes `holes` from everything but a polygon).
#[test]
fn a_hatch_keeps_its_islands_when_it_changes() {
    let mut doc = empty();
    let ring = vec![
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 10.0, y: 0.0 },
        Vec2 { x: 0.0, y: 10.0 },
    ];
    let island = vec![
        Vec2 { x: 1.0, y: 1.0 },
        Vec2 { x: 2.0, y: 1.0 },
        Vec2 { x: 1.0, y: 2.0 },
    ];
    let hatch = Entity::Hatch(HatchEntity {
        base: base("a"),
        ring,
        holes: Some(vec![island.clone()]),
        pattern: HatchPattern {
            kind: HatchPatternType::Solid,
            angle: 0.0,
            spacing: 1.0,
        },
    });
    let slot = doc.add(hatch).expect("slot");
    let mut changed = doc.get(slot).expect("hatch").clone();
    if let Entity::Hatch(h) = &mut changed {
        h.base.attrs.insert("N".into(), "1".into());
    }
    doc.update(slot, changed);
    let Some(Entity::Hatch(h)) = doc.get(slot) else {
        panic!("a hatch");
    };
    assert_eq!(h.holes, Some(vec![island]));
}

/// The web undoes the step before an open group (a processing model still
/// running), and the group's end then clears it from redo.
#[test]
fn undo_and_redo_wait_while_a_transaction_or_a_group_is_open() {
    let mut doc = empty();
    doc.add(point("a", 1.0)).expect("slot");
    let group = doc.begin_group("Model");
    doc.add(point("a", 2.0)).expect("slot");
    assert_eq!(doc.undo(), None);
    assert_eq!(doc.len(), 2);
    doc.end_group(group);
    assert_eq!(doc.undo().as_deref(), Some("Model"));
    assert_eq!(doc.undo().as_deref(), Some(labels::ADD));
    assert_eq!(doc.redo().as_deref(), Some(labels::ADD));

    let inside: Result<Option<String>, ()> = doc.transact("İşlem", |doc| Ok(doc.undo()));
    assert_eq!(inside, Ok(None));
    assert_eq!(doc.len(), 1);
}

#[test]
fn a_group_begun_inside_another_ends_nothing() {
    let mut doc = empty();
    let outer = doc.begin_group("Dış");
    let inner = doc.begin_group("İç");
    doc.add(point("a", 1.0)).expect("slot");
    doc.end_group(inner);
    assert!(doc.is_busy());
    assert!(!doc.can_undo());
    doc.end_group(outer);
    assert!(!doc.is_busy());
    assert_eq!(doc.undo().as_deref(), Some("Dış"));
}

#[test]
fn isolating_an_unknown_layer_changes_nothing() {
    let mut doc = empty();
    doc.isolate_layer("yok");
    assert!(doc.layers().nodes().iter().all(|node| node.visible));
    assert!(!doc.is_dirty());
}

#[test]
fn the_last_slot_is_given_and_then_adding_is_refused_whole() {
    let mut snapshot = sample();
    entity_base_mut(&mut snapshot.entities[0]).id = u32::MAX - 1;
    let mut doc = Document::from_snapshot(snapshot).expect("opens");
    let before = doc.len();
    assert_eq!(doc.add(point("parsel", 1.0)), Ok(Slot(u32::MAX)));
    let refused = doc.add_many(vec![point("parsel", 2.0)], labels::ADD);
    assert!(refused.is_err());
    assert_eq!(doc.len(), before + 1);
    assert!(
        refused
            .expect_err("refused")
            .to_string()
            .contains("tükendi")
    );
}

/// Readers that follow the document (the geometry store, docs/adr/0029)
/// learn every object an applied op touched: edits, undo, redo, a failed
/// transaction and a cancelled group; layer changes touch no object.
#[test]
fn changes_name_every_object_an_applied_op_touched() {
    use kentos_domain::Changes;
    let slots = |doc: &Document, mark| match doc.changes_since(mark) {
        Changes::Slots(s) => s.iter().map(|s| s.0).collect::<Vec<_>>(),
        Changes::All => panic!("the journal reaches back"),
    };
    let mut doc = empty();
    let start = doc.change_mark();
    let a = doc.add(point("a", 1.0)).expect("a slot");
    let b = doc.add(point("b", 2.0)).expect("a slot");
    assert_eq!(slots(&doc, start), [a.0, b.0]);
    let after_adds = doc.change_mark();
    assert!(doc.update(a, moved(&doc, a, 5.0)));
    assert_eq!(doc.remove(&[b]), 1);
    assert_eq!(slots(&doc, after_adds), [a.0, b.0]);
    let before_undo = doc.change_mark();
    doc.undo();
    doc.redo();
    assert_eq!(slots(&doc, before_undo), [b.0, b.0]);
    // A transaction that fails reverts what it did: both the op and its reversal are named.
    let before_failure = doc.change_mark();
    let failed: Result<(), &str> = doc.transact("Deneme", |doc| {
        doc.add(point("a", 9.0)).map_err(|_| "slots")?;
        Err("vazgeç")
    });
    assert!(failed.is_err());
    let named = slots(&doc, before_failure);
    assert_eq!(named.len(), 2);
    assert_eq!(named[0], named[1]);
    // Layer state is not an object.
    let before_layers = doc.change_mark();
    doc.toggle_layer_visible("a");
    doc.toggle_layer_locked("g");
    assert_eq!(slots(&doc, before_layers), Vec::<u32>::new());
    // A document read afresh has an empty journal: an old mark reads everything again.
    let fresh = empty();
    assert_eq!(fresh.changes_since(doc.change_mark()), Changes::All);
}

fn entity_base_mut(entity: &mut Entity) -> &mut EntityBase {
    match entity {
        Entity::Point(e) => &mut e.base,
        Entity::Line(e) => &mut e.base,
        Entity::Polyline(e) | Entity::Polygon(e) => &mut e.base,
        Entity::Circle(e) => &mut e.base,
        Entity::Arc(e) => &mut e.base,
        Entity::Ellipse(e) => &mut e.base,
        Entity::Spline(e) => &mut e.base,
        Entity::Xline(e) | Entity::Ray(e) => &mut e.base,
        Entity::Text(e) => &mut e.base,
        Entity::Dimension(e) => &mut e.base,
        Entity::Hatch(e) => &mut e.base,
    }
}

/// A layer without an id gets `layer-N` after the largest `layer-N` the file
/// has (the web's counter), never one a node has, and never the same twice.
/// A given id is kept unless a node has it already.
#[test]
fn new_layers_count_on_from_the_file_and_never_repeat_an_id() {
    let text = r#"{"format":"kentos.document","version":1,"name":"Deneme","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
        "layers":[{"id":"layer-3","name":"Üç","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]},
        {"id":"layer-12","name":"On iki","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"layer-3","entities":[],"styles":{"items":[],"categories":[]}}"#;
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads"))
        .expect("opens");
    assert_eq!(doc.add_layer(NewLayer::layer("Bir"), None), "layer-13");
    assert_eq!(doc.add_layer(NewLayer::group("İki"), None), "layer-14");
    // A given id is kept, and a larger layer-N moves the counter on.
    let forty = NewLayer {
        id: Some("layer-40".into()),
        ..NewLayer::layer("Kırk")
    };
    assert_eq!(doc.add_layer(forty, None), "layer-40");
    assert_eq!(doc.add_layer(NewLayer::layer("Sonraki"), None), "layer-41");
    // An id a node has already gets a new one; the old node keeps it.
    let same = NewLayer {
        id: Some("layer-12".into()),
        ..NewLayer::layer("Aynı")
    };
    assert_eq!(doc.add_layer(same, None), "layer-42");
    assert_eq!(
        doc.layers().get("layer-12").map(|n| n.name.as_str()),
        Some("On iki")
    );
    assert!(doc.is_dirty());
    assert!(!doc.can_undo());
}

/// A group is made empty and open; a new layer is not made active by itself
/// (the web's Yeni katman command does that).
#[test]
fn a_new_group_is_empty_and_the_active_layer_stays() {
    let mut doc = empty();
    let group = doc.add_layer(NewLayer::group("Yeni grup"), Some("a"));
    let node = doc.layers().get(&group).expect("added").clone();
    assert!(node.children.is_empty() && node.expanded);
    assert_eq!(
        doc.layers().parent(&group).map(|p| p.id.as_str()),
        Some("g")
    );
    assert_eq!(doc.layers().active(), "a");
    assert_eq!(doc.layers().unique_name("Yeni grup"), "Yeni grup 2");
}
