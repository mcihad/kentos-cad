//! What the shared cases of `cad.entities.delete` (tests/fixtures.rs) do not
//! show: deletes inside an open transaction join it, one undo step for all,
//! and the undo brings the objects back with their ids, in their places.

use kentos_domain::contracts::{CommandResult, DocumentSnapshotV1, EntitiesDelete};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, delete};
use serde_json::Value;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.entities.delete.json");

fn drawing() -> Document {
    let fixture: Value = serde_json::from_str(CASES).expect("the cases read");
    let snapshot =
        DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).expect("the drawing reads");
    Document::from_snapshot(snapshot).expect("opens")
}

fn uid(doc: &Document, slot: u32) -> String {
    doc.uid(Slot(slot)).expect("an object").to_string()
}

#[test]
fn inside_a_transaction_the_delete_joins_it() {
    let mut doc = drawing();
    let before = doc.revision();
    let (a, b) = (uid(&doc, 4), uid(&doc, 9));
    let results = doc
        .transact("Model", |doc| {
            let mut cx = ExecutionContext::new(doc);
            let first = delete::execute(
                &mut cx,
                EntitiesDelete {
                    uids: vec![a.clone()],
                    expected_revision: None,
                },
            );
            let second = delete::execute(
                &mut cx,
                EntitiesDelete {
                    uids: vec![b.clone()],
                    expected_revision: None,
                },
            );
            Ok::<_, ()>([first, second])
        })
        .expect("the transaction commits");
    for result in results {
        let CommandResult::Completed { output, .. } = result else {
            panic!("deleted, not {result:?}");
        };
        // The revision moves when the transaction ends, not with each delete inside it.
        assert_eq!(output.revision, before.to_string());
    }
    assert_ne!(doc.revision(), before);
    assert_eq!(doc.len(), 4);
    assert_eq!(
        doc.undo().as_deref(),
        Some("Model"),
        "one step, the transaction's"
    );
    let ids: Vec<u32> = doc.entities().map(|e| e.base().id).collect();
    assert_eq!(ids, [4, 9, 12, 15, 20, 21], "back in their places");
    assert_eq!((uid(&doc, 4), uid(&doc, 9)), (a, b), "with their ids");
}
