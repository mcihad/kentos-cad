//! Read-only candidate identity maps, policies, linked objects, limits and cooperative cancellation.
use kentos_commands::{BlocksFind, CommandResult, DocumentSnapshotV1};
use kentos_domain::{Document, Slot};
use kentos_native_application::blocks_find;
use serde_json::Value;

fn drawing() -> Document {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    Document::from_snapshot(DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap())
        .unwrap()
}

#[test]
fn limits_and_cancellation_keep_the_source_exact() {
    let doc = drawing();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(Slot(1)).unwrap().to_string();
    let input = BlocksFind {
        source: vec![uid.clone()],
        uids: Some(vec![uid; 50_001]),
        ..Default::default()
    };
    let CommandResult::Failed { error } = blocks_find::inspect(&doc, &input, || false) else {
        panic!("scope must refuse")
    };
    assert_eq!(error.code, "recognition_scope_limit");
    let input = BlocksFind {
        source: vec![doc.uid(Slot(1)).unwrap().to_string()],
        ..Default::default()
    };
    let mut checks = 0;
    let cancelled = blocks_find::inspect(&doc, &input, || {
        checks += 1;
        checks > 8
    });
    assert!(matches!(cancelled, CommandResult::Cancelled));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.can_undo());
}

#[test]
fn source_order_is_an_explicit_identity_map_and_duplicates_are_taken_once() {
    let doc = drawing();
    let uid = doc.uid(Slot(1)).unwrap().to_string();
    let input = BlocksFind {
        source: vec![uid.clone(), uid.clone()],
        ..Default::default()
    };
    let CommandResult::Completed { output, warnings } =
        blocks_find::inspect(&doc, &input, || false)
    else {
        panic!("query")
    };
    assert_eq!(output.source, [uid]);
    assert_eq!(output.matches.len(), 2);
    assert!(warnings.is_empty());
    assert_eq!(
        output.matches[1].members,
        [doc.uid(Slot(2)).unwrap().to_string()]
    );
    assert_eq!(output.matches[1].at.x, 10.0);
    assert_eq!(output.matches[1].scale, 1.0);
}

#[test]
fn dense_coincident_objects_reach_the_result_limit_without_exhausting_the_work_budget() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = Value::Array(
        (1..=10_000)
            .map(|id| {
                serde_json::json!({
                    "kind":"circle", "id":id, "layerId":"a", "attrs":{},
                    "c":{"x":0.0,"y":0.0}, "r":0.4
                })
            })
            .collect(),
    );
    let doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before = doc.to_snapshot_v2();
    let input = BlocksFind {
        source: vec![doc.uid(Slot(1)).unwrap().to_string()],
        ..Default::default()
    };
    let CommandResult::Completed { output, warnings } =
        blocks_find::inspect(&doc, &input, || false)
    else {
        panic!("query")
    };
    assert_eq!(output.matches.len(), 1000);
    assert!(output.truncated);
    assert!(
        output.comparisons < 3000,
        "{} comparisons",
        output.comparisons
    );
    assert!(warnings.iter().any(|w| w.code == "recognition_truncated"));
    let ids: std::collections::HashSet<_> = output.matches.iter().map(|m| &m.members[0]).collect();
    assert_eq!(ids.len(), 1000);
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.can_undo());
}
