//! What the shared cases of `cad.entities.create` (tests/fixtures.rs) do not
//! show: objects in a document out of slots (nothing is written, not even
//! the ones that fit), writes inside an open transaction, −0 carried over as
//! given, and the undo step's names.

use kentos_domain::contracts::{
    CommandResult, CreateOperation, DocumentSnapshotV1, EntitiesCreate, Entity, EntityGeometry,
    NewObject, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, create};
use serde_json::Value;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.entities.create.json");

/// The shared cases' drawing, with `change` applied to its setup first.
fn drawing(change: impl FnOnce(&mut Value)) -> Document {
    let fixture: Value = serde_json::from_str(CASES).expect("the cases read");
    let mut setup = fixture["setup"].clone();
    change(&mut setup);
    let snapshot = DocumentSnapshotV1::from_json(&setup.to_string()).expect("the drawing reads");
    Document::from_snapshot(snapshot).expect("opens")
}

fn at(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn points(n: usize) -> EntitiesCreate {
    EntitiesCreate {
        layer_id: "yapi".into(),
        objects: (0..n)
            .map(|i| NewObject {
                geometry: EntityGeometry::Point {
                    p: at(487000.0 + i as f64, 4420000.0),
                    z: None,
                },
                color: None,
                attrs: None,
                label: None,
            })
            .collect(),
        operation: Some(CreateOperation::Divide),
        expected_revision: None,
    }
}

#[test]
fn objects_in_a_document_out_of_slots_write_nothing_and_say_so() {
    // One slot is left: the second object would not get one.
    let mut doc = drawing(|setup| {
        setup["entities"][1]["id"] = (u32::MAX - 1).into();
    });
    let before = doc.revision();
    let result = create::execute(&mut ExecutionContext::new(&mut doc), points(2));
    let CommandResult::Failed { error } = result else {
        panic!("refused, not {result:?}");
    };
    assert_eq!(error.code, create::codes::SLOTS_EXHAUSTED);
    assert_eq!(doc.revision(), before);
    assert!(!doc.can_undo());
    assert_eq!(doc.len(), 2, "not even the one that fit");
    // The one that fits is written.
    let result = create::execute(&mut ExecutionContext::new(&mut doc), points(1));
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
}

#[test]
fn inside_a_transaction_the_writes_join_it() {
    let mut doc = drawing(|_| {});
    let before = doc.revision();
    let results = doc
        .transact("Model", |doc| {
            let mut cx = ExecutionContext::new(doc);
            Ok::<_, ()>([
                create::execute(&mut cx, points(2)),
                create::execute(&mut cx, points(3)),
            ])
        })
        .expect("the transaction commits");
    for result in results {
        let CommandResult::Completed { output, .. } = result else {
            panic!("written, not {result:?}");
        };
        // The revision moves when the transaction ends, not with each command inside it.
        assert_eq!(output.revision, before.to_string());
    }
    assert_eq!(doc.len(), 7);
    assert_eq!(
        doc.undo().as_deref(),
        Some("Model"),
        "one step, the transaction's"
    );
    assert_eq!(doc.len(), 2);
}

/// The geometry is written as given, bit for bit: −0 too, which JSON
/// cannot carry into the shared cases.
#[test]
fn the_geometry_is_written_as_given() {
    let mut doc = drawing(|_| {});
    let dir = at(-0.0, 1.0);
    let result = create::execute(
        &mut ExecutionContext::new(&mut doc),
        EntitiesCreate {
            layer_id: "yapi".into(),
            objects: vec![NewObject {
                geometry: EntityGeometry::Xline {
                    p: at(487020.000000001, -0.0),
                    dir,
                },
                color: None,
                attrs: None,
                label: None,
            }],
            operation: None,
            expected_revision: None,
        },
    );
    let CommandResult::Completed { output, .. } = result else {
        panic!("written, not {result:?}");
    };
    let Some(Entity::Xline(x)) = doc.get(Slot(output.ids[0])) else {
        panic!("the construction line");
    };
    assert!(x.dir.x == 0.0 && x.dir.x.is_sign_negative());
    assert!(x.p.y == 0.0 && x.p.y.is_sign_negative());
    assert_eq!(x.p.x.to_bits(), 487020.000000001_f64.to_bits());
}

/// Each operation names its undo step as the web's tool did; none is “Ekle”.
#[test]
fn the_undo_step_is_ekle_or_the_tools_name() {
    let names = [
        (None, "Ekle"),
        (Some(CreateOperation::Parallel), "Paralel çizgi"),
        (Some(CreateOperation::PerpendicularIn), "Dik in"),
        (Some(CreateOperation::PerpendicularOut), "Dik çık"),
        (Some(CreateOperation::Divide), "Böl"),
    ];
    for (operation, name) in names {
        assert_eq!(create::label(operation), name);
    }
}
