//! The native document and the v2 snapshot (docs/specs/kcad-v2.md, ADR 0014
//! slice 4): a v1 file opens with its derived ids, project id and source
//! record, which is exactly the drawing the independent Python writer made of
//! it (fixtures/kcad/v2/migrated.json); a v2 snapshot opens with every id it
//! carries, and writing it again gives the same ids, also after edits.

use kentos_domain::contracts::{
    DocumentSnapshotV1, DocumentSnapshotV2, Entity, EntityBase, EntityId, PointEntity, Vec2,
};
use kentos_domain::{Document, Slot, Uuid};
use serde_json::Value;

const V1: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/document/v1/sample.json"
);
const MIGRATED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/kcad/v2/migrated.json"
);

fn v1() -> Document {
    let text = std::fs::read_to_string(V1).expect("the v1 sample");
    Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("reads")).expect("opens")
}

/// A snapshot as JSON with the objects' slots left out (a v2 file does not keep them).
fn without_slots(doc: &DocumentSnapshotV2) -> Value {
    let mut v = serde_json::to_value(doc).expect("serializes");
    for e in v["entities"].as_array_mut().expect("entities") {
        e.as_object_mut().expect("object").remove("id");
    }
    v
}

fn point(x: f64) -> Entity {
    Entity::Point(PointEntity {
        base: EntityBase {
            id: 0,
            layer_id: "cizim".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
        },
        p: Vec2 { x, y: 4_420_190.0 },
        z: None,
    })
}

#[test]
fn a_v1_file_becomes_the_reference_migration() {
    let doc = v1();
    let want: DocumentSnapshotV2 =
        serde_json::from_str(&std::fs::read_to_string(MIGRATED).expect("migrated.json"))
            .expect("the reference drawing");
    assert_eq!(without_slots(&doc.to_snapshot_v2()), without_slots(&want));
    assert_eq!(
        doc.project_id().map(|p| p.to_string()).as_deref(),
        Some("bbe66b3a-6735-5aad-a407-a59712663053")
    );
    assert_eq!(
        doc.migrated_from().map(|s| s.source_sha256.as_str()),
        Some("6c4eb805e94f39189c1bbaa922c6082553b990df75fd6c68ef00461b45fcea2b")
    );
}

#[test]
fn a_v2_snapshot_keeps_every_id_through_edits_and_saves() {
    let mut doc = v1();
    let before: Vec<(Slot, Uuid)> = doc
        .entities()
        .map(|e| Slot(e.base().id))
        .map(|s| (s, doc.uid(s).expect("an id")))
        .collect();
    // An edit keeps the id, a new object gets a v7, a removed one is gone.
    let (moved, _) = before[1];
    let mut line = doc.get(moved).cloned().expect("the line");
    if let Entity::Line(l) = &mut line {
        l.b.x += 1.5;
    }
    assert!(doc.update(moved, line));
    let added = doc.add(point(486_530.0)).expect("a slot");
    let new_uid = doc.uid(added).expect("an id");
    assert_eq!(new_uid.get_version_num(), 7);
    let (gone, gone_uid) = before[12];
    assert_eq!(doc.remove(&[gone]), 1);

    let saved = doc.to_snapshot_v2();
    let reopened = Document::from_snapshot_v2(saved.clone()).expect("opens");
    assert!(!reopened.is_dirty());
    assert_eq!(reopened.project_id(), doc.project_id());
    assert_eq!(reopened.migrated_from(), doc.migrated_from());
    let ids: Vec<Uuid> = reopened
        .entities()
        .map(|e| reopened.uid(Slot(e.base().id)).expect("an id"))
        .collect();
    let mut want: Vec<Uuid> = before[..12].iter().map(|&(_, u)| u).collect();
    want.push(new_uid);
    assert_eq!(ids, want);
    assert!(reopened.slot_of(gone_uid).is_none());
    // Written again, the same drawing.
    assert_eq!(
        without_slots(&reopened.to_snapshot_v2()),
        without_slots(&saved)
    );
}

#[test]
fn a_v2_snapshot_with_bad_ids_is_refused() {
    let good = v1().to_snapshot_v2();
    let refused = |change: &dyn Fn(&mut DocumentSnapshotV2)| {
        let mut s = good.clone();
        change(&mut s);
        Document::from_snapshot_v2(s).expect_err("refused")
    };
    assert!(refused(&|s| s.uids[3] = s.uids[2]).contains("Nesne 4 (polygon) › kalıcı kimlik"));
    assert!(refused(&|s| s.uids[0] = EntityId([0; 16])).contains("Nesne 1"));
    assert!(refused(&|s| {
        s.uids.pop();
    })
    .contains("13 nesne ama 12 kimlik"));
    assert!(refused(&|s| {
        if let Entity::Point(p) = &mut s.entities[0] {
            p.base.layer_id = "yok".into();
        }
    })
    .contains("“yok” katmanı dosyada yok"));
}
