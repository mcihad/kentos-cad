//! The byte-level fixtures (fixtures/kcad/v2), read as their hand-written
//! expected.json says (docs/specs/kcad-v2.md): the independent Python writer
//! (scripts/fixtures/kcad_v2_reference.py) made every file, so the codec is
//! held to an outside implementation, not to its own output.
//!
//! - every file sniffs as expected, and a broken one is refused with its code;
//! - a valid file reads to its drawing (`*.json`, the contract's JSON form),
//!   every float64 bit for bit, and the writer gives back the same bytes;
//! - the web app's v1 sample, migrated here (derived ids, project id, source
//!   record; docs/adr/0014), is `migrated.kcad` byte for byte.

use kentos_kcad::contracts::{DocumentSnapshotV1, DocumentSnapshotV2, migrate_v1};
use serde_json::Value;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// A drawing as JSON text: float64 in its shortest round-trip form, so -0.0 and 0.0 differ.
fn json(doc: &DocumentSnapshotV2) -> String {
    serde_json::to_string(doc).expect("serializes")
}

#[test]
fn every_fixture_reads_as_expected_json_says() {
    let expected: Value = serde_json::from_slice(&read("expected.json")).expect("expected.json");
    let cases = expected["files"].as_array().expect("files");
    assert!(cases.len() >= 50, "{} cases", cases.len());
    let (mut valid, mut broken) = (0, 0);
    for case in cases {
        let file = case["file"].as_str().expect("file");
        let data = read(file);
        assert_eq!(
            kentos_kcad::sniff(&data).as_str(),
            case["sniff"].as_str().expect("sniff"),
            "{file}: sniff"
        );
        match (kentos_kcad::decode(&data), case.get("error")) {
            (Err(e), Some(code)) => {
                assert_eq!(e.code.as_str(), code.as_str().expect("code"), "{file}: {e}");
                assert!(!e.message.is_empty());
                broken += 1;
            }
            (Err(e), None) => panic!("{file}: {} ({e})", e.code.as_str()),
            (Ok(_), Some(code)) => panic!("{file}: read, but {code} was expected"),
            (Ok(doc), None) => {
                // A drawing written by hand, or a file the apps wrote (only its object count here).
                let want = match case["content"].as_str() {
                    Some(name) => content(name),
                    None => {
                        let count = case["entities"].as_u64().expect("entities");
                        assert_eq!(doc.entities.len() as u64, count, "{file}");
                        doc.clone()
                    }
                };
                assert_eq!(json(&doc), json(&want), "{file}: the drawing");
                if case.get("rewrite") != Some(&Value::Bool(false)) {
                    let written = kentos_kcad::encode(&want).expect("writes");
                    assert!(written == data, "{file}: the writer's bytes differ");
                    assert_eq!(kentos_kcad::encode_verified(&want).expect("verifies"), data);
                }
                valid += 1;
            }
        }
    }
    assert_eq!((valid, broken), (6, cases.len() - 6));
}

#[test]
fn the_web_apps_v1_sample_migrates_to_the_reference_bytes() {
    let text = std::fs::read_to_string(format!("{DIR}../../document/v1/sample.json"))
        .expect("the v1 sample");
    let v2 = migrate_v1(DocumentSnapshotV1::from_json(&text).expect("reads")).expect("migrates");
    assert!(kentos_kcad::encode_verified(&v2).expect("writes") == read("migrated.kcad"));
    let source = v2.migrated_from.as_ref().expect("a source record");
    assert_eq!(
        source.source_sha256,
        "6c4eb805e94f39189c1bbaa922c6082553b990df75fd6c68ef00461b45fcea2b"
    );
    assert_eq!(
        v2.project_id.map(|p| p.to_text()).as_deref(),
        Some("bbe66b3a-6735-5aad-a407-a59712663053")
    );
}

#[test]
fn the_minimal_file_is_the_one_the_specification_shows() {
    // docs/specs/kcad-v2.md §10: the header's fields at their offsets.
    let data = read("minimal.kcad");
    assert_eq!(data.len(), 490);
    assert_eq!(&data[..9], b"\x89KCAD\r\n\x1a\n");
    assert_eq!(&data[9..16], &[2, 0, 0, 36, 0, 1, 0]);
    assert_eq!(&data[16..24], &422u64.to_le_bytes());
    assert_eq!(&data[24..32], &422u64.to_le_bytes());
    assert_eq!(&data[32..36], &[0, 0, 0, 0]);
    let (header, doc) = kentos_kcad::read(&data).expect("reads");
    assert_eq!(
        (header.major, header.minor, header.payload_length),
        (2, 0, 422)
    );
    assert_eq!(doc.entities.len(), 1);
    assert_eq!(
        doc.uids[0].to_text(),
        "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e10"
    );
    assert_eq!(
        doc.entities[0].base().id,
        1,
        "slots are given in file order"
    );
}
