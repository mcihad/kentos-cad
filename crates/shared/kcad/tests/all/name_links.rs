//! A point's name in the project file (docs/adr/9314, docs/specs/kcad-v2.md): a text linked to a point without a
//! scale. Document schema 257 is written only for a drawing that has one; the link comes back as written, the typed
//! columns carry it, and schema 256 refuses it.

use kentos_kcad::Code;
use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says: the unsigned integer after the `version` key, one or two bytes long.
fn schema(file: &[u8]) -> u32 {
    let payload = &file[36..file.len() - 32];
    let key = b"\x67version";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("a version key")
        + key.len();
    match payload[at] {
        0x18 => u32::from(payload[at + 1]),
        0x19 => u32::from(u16::from_be_bytes([payload[at + 1], payload[at + 2]])),
        small => u32::from(small),
    }
}

fn json(value: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

#[test]
fn a_point_s_name_is_schema_257_and_comes_back() {
    let doc = content("name-links.json");
    let named = doc
        .entities
        .iter()
        .filter(|e| matches!(e, Entity::Text(t) if t.label_of.is_some() && t.label_scale.is_none()))
        .count();
    assert_eq!(named, 1, "the sample's point's name");
    let written = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&written), 257);
    assert!(written == read("name-links.kcad"), "the reference's bytes");
    let back = kentos_kcad::decode(&written).expect("reads");
    assert_eq!(json(&back), json(&doc));
}

#[test]
fn the_typed_columns_carry_a_point_s_name() {
    let doc = content("name-links.json");
    let (_, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(json(&entities), json(&doc.entities));
}

#[test]
fn schema_256_refuses_a_point_s_name() {
    let e =
        kentos_kcad::decode(&read("broken/name-links-in-schema-256.kcad")).expect_err("refused");
    assert_eq!(e.code, Code::BadValue, "{e}");
    // The drawing without one keeps its schema: the linked texts' sample is still schema 18's.
    let linked = kentos_kcad::encode(&content("linked-texts.json")).expect("writes");
    assert!(linked == read("linked-texts.kcad"));
}
