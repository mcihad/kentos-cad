//! A data layer's edit session (docs/adr/9103 §2): new rows drawn, rows taken and changed, rows deleted, what the layer
//! would write, a save taking it out of the drawing, letting go, and what a saved file keeps.


use kentos_domain::contracts::{
    DocumentSnapshotV1, Entity, EntityBase, PointEntity, Vec2, codes,
};
use kentos_domain::{DataWritten, Document};

/// A drawing with a drawing layer `a` and a point data layer `p` (Parseller) over a GeoPackage table.
fn doc() -> Document {
    let text = r#"{"format":"kentos.document","version":1,"name":"Deneme","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
        "layers":[
        {"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]},
        {"id":"p","name":"Parseller","type":"layer","visible":true,"locked":false,"expanded":true,"geometry":"point",
        "data":{"driver":"gpkg","address":"/veri/kent.gpkg","table":"direk","key":"fid",
        "geometry":{"column":"geom","typeName":"POINT","srid":5256}},
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]},
        {"id":"r","name":"Salt","type":"layer","visible":true,"locked":false,"expanded":true,"geometry":"point",
        "data":{"driver":"gpkg","address":"/veri/kent.gpkg","table":"eski","key":"fid","readOnly":true,
        "geometry":{"column":"geom","typeName":"POINT","srid":5256}},
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
    Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
}

fn point(layer: &str, x: f64, no: &str) -> Entity {
    let mut attrs = std::collections::BTreeMap::new();
    attrs.insert("no".to_owned(), no.to_owned());
    Entity::Point(PointEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer.into(),
            color: None,
            attrs,
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

/// Writes `body` as one step the way a command does, its faults checked: the first fault's code.
fn step(doc: &mut Document, body: impl FnOnce(&mut Document)) -> Option<&'static str> {
    let mut code = None;
    let _ = doc.run_step(
        |doc| -> Result<(), ()> {
            body(doc);
            Ok(())
        },
        |doc, report, _| {
            let faults = doc.faults(report);
            code = faults.first().map(|f| f.code.unwrap_or("integrity"));
            if faults.is_empty() { Ok(None) } else { Err(()) }
        },
    );
    code
}

#[test]
fn only_a_data_layer_opens_for_editing_and_not_a_read_only_one() {
    let mut doc = doc();
    assert!(doc.start_editing("a").is_err(), "a drawing layer is always edited");
    let read_only = doc.start_editing("r").expect_err("read-only");
    assert!(read_only.0.contains("salt okunur"), "{read_only}");
    assert_eq!(doc.start_editing("p"), Ok(true));
    assert_eq!(doc.start_editing("p"), Ok(false), "open already");
    assert!(doc.editing("p"));
    assert_eq!(doc.edited_layers(), vec!["p".to_owned()]);
    assert!(!doc.is_dirty(), "opening is not an edit");
}

#[test]
fn a_data_layer_not_being_edited_takes_no_object() {
    let mut doc = doc();
    let code = step(&mut doc, |d| {
        d.add(point("p", 1.0, "1")).expect("added");
    });
    assert_eq!(code, Some(codes::DATA_LAYER_NOT_EDITING));
    assert_eq!(doc.count("p"), 0, "taken back whole");
    doc.start_editing("p").expect("opens");
    assert_eq!(step(&mut doc, |d| { d.add(point("p", 1.0, "1")).expect("added"); }), None);
    assert_eq!(doc.count("p"), 1);
}

#[test]
fn new_taken_changed_and_deleted_rows_are_what_the_layer_would_write() {
    let mut doc = doc();
    doc.start_editing("p").expect("opens");
    let taken = doc
        .take_data_rows("p", vec![(7, point("a", 10.0, "7")), (8, point("a", 20.0, "8"))])
        .expect("taken");
    let (seven, eight) = (taken[0].expect("7"), taken[1].expect("8"));
    assert!(!doc.is_dirty() && !doc.can_undo(), "taking is not an edit");
    assert!(!doc.data_modified("p"), "a row taken is not changed");
    assert_eq!(doc.taken_key("p", seven), Some(7));
    // Taken again: the same object.
    assert_eq!(doc.take_data_rows("p", vec![(7, point("p", 99.0, "7"))]).expect("again"), vec![Some(seven)]);

    // A new row drawn, row 7 moved, row 8 erased, row 9 deleted by its key.
    assert_eq!(step(&mut doc, |d| { d.add(point("p", 5.0, "yeni")).expect("added"); }), None);
    let slot7 = doc.slot_of(seven).expect("7 here");
    assert_eq!(step(&mut doc, |d| { assert!(d.update(slot7, point("p", 11.0, "7"))); }), None);
    let slot8 = doc.slot_of(eight).expect("8 here");
    assert_eq!(step(&mut doc, |d| { d.remove(&[slot8]); }), None);
    assert_eq!(doc.delete_data_rows("p", &[9, 9]), Ok(1));

    let changes = doc.data_changes("p");
    assert_eq!(changes.added.len(), 1);
    assert_eq!(changes.changed.len(), 1);
    assert_eq!(changes.changed[0].key, 7);
    assert_eq!(changes.deleted, vec![8, 9]);
    assert_eq!(doc.data_hidden("p").into_iter().collect::<Vec<_>>(), vec![7, 8, 9]);

    // Undo gives back the deleted key, then the erased row.
    doc.undo();
    assert_eq!(doc.data_changes("p").deleted, vec![8]);
    doc.undo();
    assert!(doc.data_changes("p").deleted.is_empty());
    doc.redo();
    doc.redo();
    assert_eq!(doc.data_changes("p").deleted, vec![8, 9]);
}

#[test]
fn a_save_takes_what_it_wrote_out_of_the_drawing_and_keeps_what_came_after() {
    let mut doc = doc();
    doc.start_editing("p").expect("opens");
    let seven = doc.take_data_rows("p", vec![(7, point("a", 10.0, "7"))]).expect("taken")[0].expect("7");
    assert_eq!(step(&mut doc, |d| { d.add(point("p", 5.0, "yeni")).expect("added"); }), None);
    assert_eq!(step(&mut doc, |d| { d.add(point("p", 6.0, "sonra")).expect("added"); }), None);
    let slot7 = doc.slot_of(seven).expect("7");
    assert_eq!(step(&mut doc, |d| { assert!(d.update(slot7, point("p", 11.0, "7"))); }), None);
    doc.delete_data_rows("p", &[9]).expect("deleted");
    let changes = doc.data_changes("p");
    let (first, second) = (changes.added[0].clone(), changes.added[1].clone());

    // The second new row changed while the save ran: it stays, taken against the row as written.
    let slot2 = doc.slot_of(second.0).expect("second");
    assert_eq!(step(&mut doc, |d| { assert!(d.update(slot2, point("p", 6.5, "sonra"))); }), None);
    doc.data_saved(
        "p",
        DataWritten {
            added: vec![(first.0, first.1.clone(), 101), (second.0, second.1.clone(), 102)],
            changed: vec![(seven, changes.changed[0].now.clone())],
            deleted: vec![9],
        },
    )
    .expect("saved");

    assert_eq!(doc.count("p"), 1, "only what changed since stays");
    let left = doc.data_changes("p");
    assert!(left.added.is_empty() && left.deleted.is_empty());
    assert_eq!(left.changed.len(), 1);
    assert_eq!(left.changed[0].key, 102, "the new row's key now");
    assert!(!doc.can_undo(), "a written row is not unwritten from here");
    assert!(doc.editing("p"), "saving keeps the layer open");
}

#[test]
fn letting_go_and_stopping() {
    let mut doc = doc();
    doc.start_editing("p").expect("opens");
    doc.take_data_rows("p", vec![(7, point("a", 10.0, "7"))]).expect("taken");
    assert_eq!(step(&mut doc, |d| { d.add(point("p", 5.0, "yeni")).expect("added"); }), None);
    doc.delete_data_rows("p", &[9]).expect("deleted");
    let refused = doc.stop_editing("p").expect_err("changes wait");
    assert!(refused.0.contains("kaydedilmemiş"), "{refused}");

    assert_eq!(doc.discard_data_edits("p"), Ok(true));
    assert_eq!(doc.count("p"), 0);
    assert!(doc.data_changes("p").is_empty() && doc.data_hidden("p").is_empty());
    assert!(!doc.can_undo() && !doc.can_redo(), "its steps are gone");
    assert!(doc.editing("p"), "letting go keeps the layer open");

    doc.take_data_rows("p", vec![(7, point("a", 10.0, "7"))]).expect("taken");
    assert_eq!(doc.stop_editing("p"), Ok(true), "an unchanged taken row is no change");
    assert_eq!(doc.count("p"), 0);
    assert!(!doc.editing("p"));
    assert!(doc.take_data_rows("p", vec![(7, point("a", 1.0, "7"))]).is_err(), "not being edited");
}

#[test]
fn a_saved_file_keeps_the_new_rows_and_leaves_the_taken_ones_to_their_table() {
    let mut doc = doc();
    doc.start_editing("p").expect("opens");
    doc.take_data_rows("p", vec![(7, point("a", 10.0, "7"))]).expect("taken");
    assert_eq!(step(&mut doc, |d| { d.add(point("p", 5.0, "yeni")).expect("added"); }), None);
    let file = doc.to_snapshot_v2();
    assert_eq!(file.entities.len(), 1, "the new row only");
    let again = Document::from_snapshot_v2(file).expect("reads back");
    assert!(again.editing("p"), "a data layer holding objects is being edited");
    assert_eq!(again.data_changes("p").added.len(), 1);
    assert!(!again.editing("a"));
    
}

/// What a saved file leaves out comes back beside it (docs/adr/9111 §3): the taken rows with their objects now and as they
/// came, an erased one too, and the keys deleted without being taken; a save then finds the same changes. Nothing of it
/// is an edit.
#[test]
fn the_rows_a_file_leaves_out_come_back_beside_it() {
    let mut doc = doc();
    doc.start_editing("p").expect("opens");
    let taken = doc
        .take_data_rows("p", vec![(7, point("a", 10.0, "7")), (8, point("a", 20.0, "8")), (6, point("a", 30.0, "6"))])
        .expect("taken");
    let (seven, eight) = (taken[0].expect("7"), taken[1].expect("8"));
    assert_eq!(step(&mut doc, |d| { d.add(point("p", 5.0, "yeni")).expect("added"); }), None);
    let slot7 = doc.slot_of(seven).expect("7 here");
    assert_eq!(step(&mut doc, |d| { assert!(d.update(slot7, point("p", 11.0, "7"))); }), None);
    let slot8 = doc.slot_of(eight).expect("8 here");
    assert_eq!(step(&mut doc, |d| { d.remove(&[slot8]); }), None);
    doc.delete_data_rows("p", &[9]).expect("deleted");

    // Row 6, taken and unchanged, waits on nothing: it is not among them.
    let pending = doc.pending_rows();
    assert_eq!(pending.len(), 1);
    let keys: Vec<(i64, bool)> = pending[0].taken.iter().map(|r| (r.key, r.now.is_some())).collect();
    assert_eq!(keys, vec![(7, true), (8, false)], "7 changed, 8 erased");
    assert_eq!(pending[0].deleted, vec![9]);
    let before = doc.data_changes("p");

    let mut again = Document::from_snapshot_v2(doc.to_snapshot_v2()).expect("reads back");
    assert_eq!(again.data_changes("p").changed.len(), 0, "the file alone has the new row only");
    assert!(again.restore_pending(pending.clone()).is_empty(), "everything came back");
    let after = again.data_changes("p");
    assert_eq!(after.added.len(), 1);
    assert_eq!(after.changed.len(), 1);
    assert_eq!(after.changed[0].key, 7);
    let (Entity::Point(was), Entity::Point(now)) = (&*before.changed[0].now, &*after.changed[0].now) else {
        panic!("points");
    };
    assert_eq!((was.p.x, now.p.x), (11.0, 11.0), "moved as it was");
    assert_eq!(after.deleted, vec![8, 9]);
    assert_eq!(again.taken_key("p", seven), Some(7), "under its own id");
    assert!(!again.is_dirty() && !again.can_undo(), "giving back is not an edit");
    assert_eq!(again.pending_rows().len(), 1);

    // Given back twice: refused row by row, nothing doubled.
    let said = again.restore_pending(pending);
    assert_eq!(said.len(), 2, "{said:?}");
    assert_eq!(again.data_changes("p").changed.len(), 1);
    // A layer one may not edit is said.
    let refused = again.restore_pending(vec![kentos_domain::PendingLayer {
        layer: "r".into(),
        taken: Vec::new(),
        deleted: vec![1],
    }]);
    assert_eq!(refused.len(), 1);
    assert!(refused[0].contains("salt okunur"), "{refused:?}");
}

/// A new row the file holds but that was written to its table after the file was saved leaves the drawing opened again
/// (docs/adr/9111 §3): written twice otherwise. A layer left with nothing to save stops being edited.
#[test]
fn what_was_written_after_the_file_was_saved_leaves_the_drawing_opened_again() {
    let mut doc = doc();
    doc.start_editing("p").expect("opens");
    assert_eq!(step(&mut doc, |d| { d.add(point("p", 5.0, "yeni")).expect("added"); }), None);
    let file = doc.to_snapshot_v2();
    let written = doc.data_changes("p").added.clone();
    assert!(doc.data_left().is_empty());
    doc.data_saved("p", DataWritten { added: vec![(written[0].0, written[0].1.clone(), 41)], ..DataWritten::default() })
        .expect("saved");
    assert_eq!(doc.data_left(), &[written[0].0], "it left the drawing");

    let mut again = Document::from_snapshot_v2(file).expect("reads back");
    assert!(again.editing("p") && again.count("p") == 1, "the file holds it as a new row");
    assert_eq!(again.drop_left(doc.data_left()), 1);
    assert_eq!(again.count("p"), 0);
    assert!(!again.editing("p"), "nothing left to save");
    assert!(!again.is_dirty() && !again.can_undo());
    assert_eq!(again.drop_left(doc.data_left()), 0, "gone already");

    // Letting go is kept the same way.
    let mut doc = self::doc();
    doc.start_editing("p").expect("opens");
    assert_eq!(step(&mut doc, |d| { d.add(point("p", 5.0, "yeni")).expect("added"); }), None);
    let uid = doc.data_changes("p").added[0].0;
    assert_eq!(doc.discard_data_edits("p"), Ok(true));
    assert_eq!(doc.data_left(), &[uid]);
}
