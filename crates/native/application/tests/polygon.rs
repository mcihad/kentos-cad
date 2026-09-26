//! What only the desktop's handler of `cad.polygon.create` can meet, beside
//! the shared cases (tests/fixtures.rs): a document out of slots, and a
//! command inside an open transaction.

use kentos_domain::Document;
use kentos_domain::contracts::{
    CommandResult, DocumentSnapshotV1, PolygonCreate, PolygonCreated, Vec2,
};
use kentos_native_application::{ExecutionContext, polygon};
use serde_json::Value;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.polygon.create.json");

/// The shared cases' drawing, with its second object's id changed to `last_id`.
fn drawing(last_id: u32) -> Document {
    let fixture: Value = serde_json::from_str(CASES).expect("the cases read");
    let mut setup = fixture["setup"].clone();
    setup["entities"][1]["id"] = last_id.into();
    let snapshot = DocumentSnapshotV1::from_json(&setup.to_string()).expect("the drawing reads");
    Document::from_snapshot(snapshot).expect("opens")
}

fn triangle() -> PolygonCreate {
    PolygonCreate {
        layer_id: "yapi".into(),
        pts: vec![
            Vec2 {
                x: 487010.0,
                y: 4420010.0,
            },
            Vec2 {
                x: 487030.0,
                y: 4420010.0,
            },
            Vec2 {
                x: 487030.0,
                y: 4420022.5,
            },
        ],
        bulges: None,
        holes: None,
        color: None,
        attrs: None,
        expected_revision: None,
    }
}

#[test]
fn a_document_out_of_slots_takes_nothing_and_says_so() {
    let mut doc = drawing(u32::MAX);
    let before = doc.revision();
    let result = polygon::execute(&mut ExecutionContext::new(&mut doc), triangle());
    let CommandResult::Failed { error } = result else {
        panic!("refused, not {result:?}");
    };
    assert_eq!(error.code, polygon::codes::SLOTS_EXHAUSTED);
    assert_eq!(
        error.message,
        "Çizimdeki nesne kimlikleri tükendi (en büyük kimlik 4294967295); yeni nesne eklenemedi."
    );
    assert_eq!(error.path, None);
    assert_eq!(doc.len(), 2);
    assert_eq!(doc.revision(), before);
    assert!(!doc.can_undo() && !doc.is_dirty());
}

#[test]
fn inside_a_transaction_the_write_joins_it() {
    let mut doc = drawing(9);
    let before = doc.revision();
    let written = doc
        .transact("Model", |doc| {
            let mut cx = ExecutionContext::new(doc);
            let first = polygon::execute(&mut cx, triangle());
            let second = polygon::execute(&mut cx, triangle());
            Ok::<_, ()>((first, second))
        })
        .expect("the transaction commits");
    for (result, id) in [written.0, written.1].into_iter().zip([10, 11]) {
        let CommandResult::Completed {
            output: PolygonCreated {
                id: got, revision, ..
            },
            ..
        } = result
        else {
            panic!("written, not {result:?}");
        };
        assert_eq!(got, id);
        // The revision moves when the transaction ends, not with each write inside it.
        assert_eq!(revision, before.to_string());
    }
    assert_ne!(doc.revision(), before);
    assert_eq!(doc.len(), 4);
    assert_eq!(
        doc.undo().as_deref(),
        Some("Model"),
        "one step, the transaction's"
    );
    assert_eq!(doc.len(), 2);
}
