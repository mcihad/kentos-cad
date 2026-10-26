//! A drawing written from objects held apart, as the open document shares them (docs/adr/0654): the same bytes as
//! the drawing written whole, every valid fixture; the read-back check stops at the first object that does not come
//! back the same.

use std::sync::Arc;

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};
use kentos_kcad::{Code, Quiet};
use serde_json::Value;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn apart(doc: &DocumentSnapshotV2) -> (DocumentSnapshotV2, Vec<Arc<Entity>>) {
    let mut head = doc.clone();
    let objects = std::mem::take(&mut head.entities)
        .into_iter()
        .map(Arc::new)
        .collect();
    (head, objects)
}

#[test]
fn every_valid_fixture_gives_the_same_bytes_from_shared_objects() {
    let expected: Value = serde_json::from_slice(
        &std::fs::read(format!("{DIR}expected.json")).expect("expected.json"),
    )
    .expect("expected.json");
    let mut written = 0;
    for case in expected["files"].as_array().expect("files") {
        if case.get("error").is_some() {
            continue;
        }
        let file = case["file"].as_str().expect("file");
        let data = std::fs::read(format!("{DIR}{file}")).expect("file");
        let Ok(doc) = kentos_kcad::decode(&data) else {
            continue;
        };
        let whole = kentos_kcad::encode_verified(&doc).expect("whole");
        let (head, objects) = apart(&doc);
        let shared =
            kentos_kcad::encode_verified_parts_watched(&head, &objects, &mut Quiet).expect(file);
        assert_eq!(shared, whole, "{file}");
        written += 1;
    }
    assert!(written >= 40, "{written} drawings");
}

#[test]
fn a_head_with_objects_is_refused() {
    let data = std::fs::read(format!("{DIR}minimal.kcad")).expect("minimal");
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    let (_, objects) = apart(&doc);
    doc.entities = objects.iter().map(|e| (**e).clone()).collect();
    let refused = kentos_kcad::encode_verified_parts_watched(&doc, &objects, &mut Quiet)
        .expect_err("refused");
    assert_eq!(refused.code, Code::BadValue);
}

#[test]
fn ids_and_objects_that_do_not_match_are_refused() {
    let data = std::fs::read(format!("{DIR}minimal.kcad")).expect("minimal");
    let doc = kentos_kcad::decode(&data).expect("reads");
    let (mut head, objects) = apart(&doc);
    assert!(!objects.is_empty(), "the fixture has objects");
    head.uids.pop();
    let refused = kentos_kcad::encode_verified_parts_watched(&head, &objects, &mut Quiet)
        .expect_err("refused");
    assert_eq!(refused.code, Code::BadValue, "{}", refused.message);
}
