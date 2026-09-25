//! The persistent ids of v1 drawings (docs/adr/0014, TODOS.md DOM-04) against the
//! independent reference: `fixtures/document/v1/identity`, written by
//! `scripts/fixtures/v1_identity_reference.py` with Python's standard library.
//! The canonical text must match it byte for byte, and every input of a case
//! (the same drawing with other whitespace, key order and number spelling) must
//! give the case's ids. The browser checks the same fixture through the formats
//! WASM module (`apps/web/src/io/identity.wasm.test.ts`).

use kentos_contracts::{
    DocumentSnapshotV1, V1EntityIdentity, V1Identities, canonical_v1, uuid_text, v1_identities,
};
use serde::Deserialize;

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/document/v1/identity/"
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Expected {
    import_namespace: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    name: String,
    inputs: Vec<String>,
    canonical: String,
    source_sha256: String,
    namespace: String,
    project: String,
    entities: Vec<V1EntityIdentity>,
}

impl Case {
    fn identities(&self) -> V1Identities {
        V1Identities {
            source_sha256: self.source_sha256.clone(),
            namespace: self.namespace.clone(),
            project: self.project.clone(),
            entities: self.entities.clone(),
        }
    }
}

fn read(name: &str) -> String {
    std::fs::read_to_string(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn expected() -> Expected {
    serde_json::from_str(&read("expected.json")).expect("expected.json")
}

#[test]
fn every_input_gives_the_reference_ids() {
    let expected = expected();
    assert_eq!(
        expected.import_namespace,
        uuid_text(&kentos_contracts::KENTOS_V1_IMPORT)
    );
    assert_eq!(expected.cases.len(), 3);
    for case in &expected.cases {
        for input in &case.inputs {
            let got = v1_identities(&read(input)).unwrap_or_else(|e| panic!("{input}: {e}"));
            assert_eq!(got, case.identities(), "{}: {input}", case.name);
        }
    }
}

#[test]
fn the_canonical_text_is_the_reference_text() {
    for case in expected().cases {
        let mut snapshot = DocumentSnapshotV1::from_json(&read(&case.inputs[0])).expect("readable");
        let text = canonical_v1(&mut snapshot).expect("written");
        assert_eq!(
            String::from_utf8(text).expect("UTF-8"),
            read(&case.canonical),
            "{}",
            case.name
        );
    }
}

#[test]
fn another_drawing_gets_other_ids_and_every_id_is_a_v5() {
    let expected = expected();
    let [sample, edited, minimal] = &expected.cases[..] else {
        panic!("three cases")
    };
    // One millimetre moved: another snapshot, so no id is shared.
    assert_ne!(sample.namespace, edited.namespace);
    assert!(
        sample
            .entities
            .iter()
            .zip(&edited.entities)
            .all(|(a, b)| a.id == b.id && a.uid != b.uid)
    );
    // Local ids are taken as the file has them (not renumbered), in file order.
    assert_eq!(
        minimal.entities.iter().map(|e| e.id).collect::<Vec<_>>(),
        [9, 5]
    );
    for uid in [&sample.namespace, &sample.project]
        .into_iter()
        .chain(sample.entities.iter().map(|e| &e.uid))
    {
        let b = uid.as_bytes();
        assert_eq!(b.len(), 36, "{uid}");
        assert_eq!(b[14], b'5', "version 5: {uid}");
        assert!(
            matches!(b[19], b'8' | b'9' | b'a' | b'b'),
            "variant 10: {uid}"
        );
        assert_eq!(uid.to_lowercase(), *uid);
    }
}

#[test]
fn a_byte_order_mark_changes_nothing() {
    let expected = expected();
    let text = format!("\u{feff}{}", read("sample.compact.kcad"));
    assert_eq!(
        v1_identities(&text).expect("readable"),
        expected.cases[0].identities()
    );
}

#[test]
fn local_ids_must_be_unique_and_positive() {
    let mut v: serde_json::Value = serde_json::from_str(&read("sample.compact.kcad")).unwrap();
    v["entities"][4]["id"] = v["entities"][3]["id"].clone();
    let err = v1_identities(&v.to_string()).unwrap_err();
    assert!(err.contains("Nesne 5 (circle): yerel kimlik 4"), "{err}");
    v["entities"][4]["id"] = serde_json::Value::from(0);
    assert!(v1_identities(&v.to_string()).is_err());
    // What the contract cannot read says why instead of giving ids.
    assert!(
        v1_identities("{\"format\":\"kentos.document\",\"version\":2}")
            .unwrap_err()
            .contains("sürümü 2")
    );
}
