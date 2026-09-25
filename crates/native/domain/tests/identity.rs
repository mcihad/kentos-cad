//! A `.kcad` v1 file opened in the native document gets the persistent ids of
//! the deterministic migration (docs/adr/0014, TODOS.md DOM-04): the ids of
//! `fixtures/document/v1/identity`, which the independent Python reference
//! wrote and the browser reproduces (apps/web/src/io/identity.wasm.test.ts).
//! So the desktop and the web name the objects of the same file the same way.

use kentos_domain::contracts::{DocumentSnapshotV1, Entity, EntityBase, PointEntity, Vec2};
use kentos_domain::{Document, Slot, Uuid};
use serde_json::Value;

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/document/v1/identity/"
);

fn read(name: &str) -> String {
    std::fs::read_to_string(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn open(text: &str) -> Document {
    Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
}

fn uuid(value: &Value) -> Uuid {
    Uuid::parse_str(value.as_str().expect("text")).expect("a UUID")
}

#[test]
fn a_v1_file_opens_with_the_reference_ids() {
    let expected: Value = serde_json::from_str(&read("expected.json")).expect("expected.json");
    let cases = expected["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 3);
    for case in cases {
        for input in case["inputs"].as_array().expect("inputs") {
            let name = input.as_str().expect("file name");
            let doc = open(&read(name));
            let entities = case["entities"].as_array().expect("entities");
            assert_eq!(doc.len(), entities.len(), "{name}");
            for e in entities {
                let slot = Slot(u32::try_from(e["id"].as_u64().expect("id")).expect("u32"));
                let uid = uuid(&e["uid"]);
                assert_eq!(doc.uid(slot), Some(uid), "{name}: {slot:?}");
                assert_eq!(doc.slot_of(uid), Some(slot), "{name}: {uid}");
            }
        }
    }
}

#[test]
fn the_same_file_opens_with_the_same_ids_and_new_objects_get_v7() {
    let text = read("sample.compact.kcad");
    let (mut first, second) = (open(&text), open(&text));
    let ids = |doc: &Document| {
        doc.entities()
            .map(|e| doc.uid(Slot(e.base().id)))
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&first), ids(&second));
    assert!(
        ids(&first)
            .iter()
            .flatten()
            .all(|u| u.get_version_num() == 5)
    );
    let point = Entity::Point(PointEntity {
        base: EntityBase {
            id: 0,
            layer_id: "cizim".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
        },
        p: Vec2 { x: 1.0, y: 2.0 },
        z: None,
    });
    let slot = first.add(point).expect("a slot");
    assert_eq!(first.uid(slot).map(|u| u.get_version_num()), Some(7));
    // Written and opened again unchanged: the same drawing, the same ids.
    let mut again = open(&text);
    let saved = serde_json::to_string(&again.to_snapshot()).expect("written");
    assert_eq!(ids(&open(&saved)), ids(&second));
    // Edited, it is another snapshot: v1 stores no ids, so every object gets others (FILE-05).
    let edited = Entity::Point(PointEntity {
        base: EntityBase {
            id: 0,
            layer_id: "cizim".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
        },
        p: Vec2 { x: 3.0, y: 4.0 },
        z: None,
    });
    again.add(edited).expect("a slot");
    let reopened = open(&serde_json::to_string(&again.to_snapshot()).expect("written"));
    let before = ids(&second);
    assert!(
        reopened
            .entities()
            .all(|e| !before.contains(&reopened.uid(Slot(e.base().id))))
    );
}
