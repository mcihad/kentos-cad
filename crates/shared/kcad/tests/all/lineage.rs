//! What the split and merged objects came from in the project file (docs/adr/0598 §1, docs/specs/kcad-v2.md §5):
//! document schema 101 is written only for a project with a record; the records come back in their order; the
//! writer refuses what the reader would; the reader's errors name their places.

use kentos_kcad::Code;
use kentos_kcad::contracts::DocumentSnapshotV2;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says.
fn schema(file: &[u8]) -> u8 {
    let payload = &file[36..file.len() - 32];
    let key = b"\x67version";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("a version key")
        + key.len();
    if payload[at] == 0x18 {
        payload[at + 1]
    } else {
        payload[at]
    }
}

#[test]
fn lineage_is_schema_101_and_comes_back_in_its_order() {
    let doc = content("lineage.json");
    let written = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&written), 101);
    assert!(written == read("lineage.kcad"), "the reference's bytes");
    let back = kentos_kcad::decode(&written).expect("reads");
    assert_eq!(back.settings.lineage, doc.settings.lineage);
    let mut plain = doc.clone();
    plain.settings.lineage.clear();
    assert!(schema(&kentos_kcad::encode(&plain).expect("writes")) < 101);
    let mut twice = doc.clone();
    twice.settings.lineage.push(doc.settings.lineage[0].clone());
    assert!(
        kentos_kcad::encode(&twice).is_err(),
        "a record twice is written"
    );
    for (name, code) in [
        ("broken/lineage-in-schema-100.kcad", Code::UnknownField),
        ("broken/lineage-unknown-field.kcad", Code::UnknownField),
        ("broken/lineage-empty.kcad", Code::BadValue),
        ("broken/lineage-own-parent.kcad", Code::BadValue),
        ("broken/lineage-parent-twice.kcad", Code::BadValue),
        ("broken/lineage-no-parents.kcad", Code::BadValue),
        ("broken/lineage-twice.kcad", Code::BadValue),
        ("broken/lineage-operation.kcad", Code::BadValue),
        ("broken/lineage-step-space.kcad", Code::BadValue),
    ] {
        let e = kentos_kcad::decode(&read(name)).expect_err(name);
        assert_eq!(e.code, code, "{name}: {e}");
    }
}
