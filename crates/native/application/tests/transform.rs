//! What the shared cases of `cad.entities.transform` (tests/fixtures.rs) do
//! not show: −0 kept through a transform (JSON cannot write it), copies in a
//! document out of slots, and transforms inside an open transaction.

use kentos_domain::contracts::{
    CommandResult, DocumentSnapshotV1, EntitiesTransform, Entity, Transform, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, transform};
use serde_json::Value;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.entities.transform.json");

/// The shared cases' drawing, with `edit` applied to its setup first.
fn drawing(edit: impl FnOnce(&mut Value)) -> Document {
    let fixture: Value = serde_json::from_str(CASES).expect("the cases read");
    let mut setup = fixture["setup"].clone();
    edit(&mut setup);
    let snapshot = DocumentSnapshotV1::from_json(&setup.to_string()).expect("the drawing reads");
    Document::from_snapshot(snapshot).expect("opens")
}

fn uid(doc: &Document, slot: u32) -> String {
    doc.uid(Slot(slot)).expect("an object").to_string()
}

fn input(uids: Vec<String>, transform: Transform, copy: bool) -> EntitiesTransform {
    EntitiesTransform {
        uids,
        transform,
        copy: copy.then_some(true),
        expected_revision: None,
    }
}

/// A mirror turns a zero bulge into −0 (the core negates every bulge) and
/// the document keeps it; a move then carries the −0 bulges over as they
/// are. The web's handler keeps them too, packed; JSON would write 0.
#[test]
fn negative_zero_stays_negative_zero() {
    let mut doc = drawing(|setup| {
        // The closed area's bulges with zeros.
        setup["entities"][1]["bulges"] = serde_json::json!([0.0, 0.5, 0.0]);
    });
    let mirror = Transform::Mirror {
        a: Vec2 {
            x: 487025.0,
            y: 4420000.0,
        },
        b: Vec2 {
            x: 487025.0,
            y: 4420010.0,
        },
    };
    let area = uid(&doc, 9);
    let negative_zeros = |doc: &Document| {
        let Some(Entity::Polygon(p)) = doc.get(Slot(9)) else {
            panic!("the closed area");
        };
        let bulges = p.bulges.clone().expect("bulges");
        assert_eq!(bulges, [0.0, -0.5, 0.0]);
        bulges[0].is_sign_negative() && bulges[2].is_sign_negative()
    };
    let result = transform::execute(
        &mut ExecutionContext::new(&mut doc),
        input(vec![area.clone()], mirror, false),
    );
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    assert!(negative_zeros(&doc), "mirrored");
    let result = transform::execute(
        &mut ExecutionContext::new(&mut doc),
        input(vec![area], Transform::Move { dx: 1.0, dy: 2.0 }, false),
    );
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    assert!(negative_zeros(&doc), "moved");
}

#[test]
fn copies_in_a_document_out_of_slots_take_nothing_and_say_so() {
    let mut doc = drawing(|setup| {
        let last = setup["entities"].as_array().map_or(0, Vec::len) - 1;
        setup["entities"][last]["id"] = u32::MAX.into();
    });
    let before = doc.revision();
    let line = uid(&doc, 10);
    let result = transform::execute(
        &mut ExecutionContext::new(&mut doc),
        input(vec![line], Transform::Move { dx: 0.0, dy: 20.0 }, true),
    );
    let CommandResult::Failed { error } = result else {
        panic!("refused, not {result:?}");
    };
    assert_eq!(error.code, transform::codes::SLOTS_EXHAUSTED);
    assert_eq!(doc.revision(), before);
    assert!(!doc.can_undo());
}

#[test]
fn inside_a_transaction_the_transforms_join_it() {
    let mut doc = drawing(|_| {});
    let before = doc.revision();
    let (line, circle) = (uid(&doc, 10), uid(&doc, 13));
    let results = doc
        .transact("Model", |doc| {
            let mut cx = ExecutionContext::new(doc);
            let moved = transform::execute(
                &mut cx,
                input(
                    vec![line.clone()],
                    Transform::Move { dx: 1.0, dy: 2.0 },
                    false,
                ),
            );
            let copied = transform::execute(
                &mut cx,
                input(
                    vec![circle.clone()],
                    Transform::Scale {
                        center: Vec2 {
                            x: 487030.0,
                            y: 4420010.0,
                        },
                        factor: 2.0,
                    },
                    true,
                ),
            );
            Ok::<_, ()>([moved, copied])
        })
        .expect("the transaction commits");
    for result in results {
        let CommandResult::Completed { output, .. } = result else {
            panic!("written, not {result:?}");
        };
        // The revision moves when the transaction ends, not with each command inside it.
        assert_eq!(output.revision, before.to_string());
    }
    assert_ne!(doc.revision(), before);
    assert_eq!(
        doc.undo().as_deref(),
        Some("Model"),
        "one step, the transaction's"
    );
    assert_eq!(doc.len(), 13, "the copy is gone");
    let Some(Entity::Line(l)) = doc.get(Slot(10)) else {
        panic!("the line");
    };
    assert_eq!((l.a.x, l.a.y), (487010.0, 4420010.0), "back in place");
}

/// Each tool's name is its undo step's.
#[test]
fn the_undo_step_is_the_tools_name() {
    let c = Vec2 { x: 0.0, y: 0.0 };
    let m = Transform::Move { dx: 1.0, dy: 0.0 };
    assert_eq!(transform::label(&m, false), "Taşı");
    assert_eq!(transform::label(&m, true), "Kopyala");
    let r = Transform::Rotate {
        center: c,
        angle: 1.0,
    };
    assert_eq!(transform::label(&r, true), "Döndür");
    let s = Transform::Scale {
        center: c,
        factor: 2.0,
    };
    assert_eq!(transform::label(&s, false), "Ölçekle");
    let a = Transform::Mirror {
        a: c,
        b: Vec2 { x: 1.0, y: 0.0 },
    };
    assert_eq!(transform::label(&a, true), "Aynala");
}
