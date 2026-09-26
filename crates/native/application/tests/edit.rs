//! What the shared cases of `cad.entities.edit` (tests/fixtures.rs) do not
//! show: new pieces in a document out of slots (nothing is written, not even
//! the part that fits), edits inside an open transaction, −0 carried over as
//! given, and a core shape turned into the command's geometry and back.

use kentos_domain::contracts::{
    CommandResult, DocumentSnapshotV1, EditOperation, EntitiesEdit, Entity, EntityEdit,
    EntityGeometry, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::geometry::{edit_geometry, entity_of, shape};
use kentos_native_application::{ExecutionContext, edit};
use serde_json::Value;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.entities.edit.json");

/// The shared cases' drawing, with `change` applied to its setup first.
fn drawing(change: impl FnOnce(&mut Value)) -> Document {
    let fixture: Value = serde_json::from_str(CASES).expect("the cases read");
    let mut setup = fixture["setup"].clone();
    change(&mut setup);
    let snapshot = DocumentSnapshotV1::from_json(&setup.to_string()).expect("the drawing reads");
    Document::from_snapshot(snapshot).expect("opens")
}

fn uid(doc: &Document, slot: u32) -> String {
    doc.uid(Slot(slot)).expect("an object").to_string()
}

fn line(ax: f64, ay: f64, bx: f64, by: f64) -> EntityGeometry {
    EntityGeometry::Line {
        a: Vec2 { x: ax, y: ay },
        b: Vec2 { x: bx, y: by },
    }
}

fn trim(doc: &Document) -> EntitiesEdit {
    let target = uid(doc, 1);
    EntitiesEdit {
        operation: EditOperation::Trim,
        changes: vec![
            EntityEdit::Replace {
                uid: target.clone(),
                geometry: line(487000.0, 4420000.0, 487008.0, 4420000.0),
                keep_data: None,
            },
            EntityEdit::Add {
                from: target,
                geometry: line(487012.0, 4420000.0, 487020.0, 4420000.0),
                keep_data: None,
            },
        ],
        expected_revision: None,
    }
}

#[test]
fn pieces_in_a_document_out_of_slots_write_nothing_and_say_so() {
    let mut doc = drawing(|setup| {
        let last = setup["entities"].as_array().map_or(0, Vec::len) - 1;
        setup["entities"][last]["id"] = u32::MAX.into();
    });
    let before = doc.revision();
    let original = doc.get(Slot(1)).cloned();
    let input = trim(&doc);
    let result = edit::execute(&mut ExecutionContext::new(&mut doc), input);
    let CommandResult::Failed { error } = result else {
        panic!("refused, not {result:?}");
    };
    assert_eq!(error.code, edit::codes::SLOTS_EXHAUSTED);
    assert_eq!(doc.revision(), before);
    assert!(!doc.can_undo());
    assert_eq!(
        doc.get(Slot(1)).cloned(),
        original,
        "the part that fit is taken back"
    );
}

#[test]
fn inside_a_transaction_the_edits_join_it() {
    let mut doc = drawing(|_| {});
    let before = doc.revision();
    let first = trim(&doc);
    let arc = uid(&doc, 4);
    let results = doc
        .transact("Model", |doc| {
            let mut cx = ExecutionContext::new(doc);
            let trimmed = edit::execute(&mut cx, first);
            let lengthened = edit::execute(
                &mut cx,
                EntitiesEdit {
                    operation: EditOperation::Lengthen,
                    changes: vec![EntityEdit::Update {
                        uid: arc,
                        geometry: EntityGeometry::Arc {
                            c: Vec2 {
                                x: 487060.0,
                                y: 4420000.0,
                            },
                            r: 5.0,
                            a0: 0.0,
                            a1: 2.0,
                        },
                    }],
                    expected_revision: None,
                },
            );
            Ok::<_, ()>([trimmed, lengthened])
        })
        .expect("the transaction commits");
    for result in results {
        let CommandResult::Completed { output, .. } = result else {
            panic!("written, not {result:?}");
        };
        // The revision moves when the transaction ends, not with each command inside it.
        assert_eq!(output.revision, before.to_string());
    }
    assert_eq!(doc.len(), 9);
    assert_eq!(
        doc.undo().as_deref(),
        Some("Model"),
        "one step, the transaction's"
    );
    assert_eq!(doc.len(), 8, "the piece is gone");
}

/// The geometry is written as given, bit for bit: −0 too, which JSON
/// cannot carry into the shared cases.
#[test]
fn the_geometry_is_written_as_given() {
    let mut doc = drawing(|_| {});
    let target = uid(&doc, 1);
    let given = line(-0.0, 4420000.0, 487020.000000001, -0.0);
    let result = edit::execute(
        &mut ExecutionContext::new(&mut doc),
        EntitiesEdit {
            operation: EditOperation::Extend,
            changes: vec![EntityEdit::Update {
                uid: target,
                geometry: given,
            }],
            expected_revision: None,
        },
    );
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    let Some(Entity::Line(l)) = doc.get(Slot(1)) else {
        panic!("the line");
    };
    assert!(l.a.x == 0.0 && l.a.x.is_sign_negative());
    assert!(l.b.y == 0.0 && l.b.y.is_sign_negative());
    assert_eq!(l.b.x.to_bits(), 487020.000000001_f64.to_bits());
}

/// What a tool computed with the core becomes the command's geometry, and
/// the object written from it is the same shape again; a hatch and a
/// dimension too, since Esnet writes them (docs/adr/0047, part 2).
#[test]
fn a_core_shape_goes_to_the_command_and_back() {
    let doc = drawing(|_| {});
    let more = [
        r#"{"kind":"hatch","id":1,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"pattern":{"type":"solid","angle":0,"spacing":1}}"#,
        r#"{"kind":"dimension","id":2,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":2,"height":0.5,"style":"angular","c":{"x":-1,"y":1}}"#,
    ]
    .map(|text| serde_json::from_str::<Entity>(text).expect("an object"));
    for e in doc.entities().chain(more.iter()) {
        let s = shape(e);
        let geometry = edit_geometry(s.clone()).expect("an edit geometry");
        let back = entity_of(&geometry, e.base().clone());
        assert_eq!(shape(&back), s, "{}", e.kind());
    }
}

/// Each operation names its undo step as the web's tool did.
#[test]
fn the_undo_step_is_the_tools_name() {
    let names = [
        (EditOperation::Offset, "Ötele"),
        (EditOperation::Trim, "Buda"),
        (EditOperation::Extend, "Uzat"),
        (EditOperation::Fillet, "Köşe yuvarla"),
        (EditOperation::Chamfer, "Pah"),
        (EditOperation::Break, "Kır"),
        (EditOperation::Join, "Birleştir"),
        (EditOperation::Explode, "Patlat"),
        (EditOperation::Lengthen, "Uzat-kısalt"),
        (EditOperation::VertexAdd, "Köşe ekle"),
        (EditOperation::VertexRemove, "Köşe sil"),
        (EditOperation::Stretch, "Esnet"),
    ];
    for (operation, name) in names {
        assert_eq!(edit::label(operation), name);
    }
}
