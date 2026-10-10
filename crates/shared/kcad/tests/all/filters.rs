//! Layer filters in the project file (docs/adr/0211 §2, docs/specs/kcad-v2.md
//! §6.5): document schema 35 is written only when a layer has a filter; every
//! other drawing keeps its schema and its bytes. Filters come back as they
//! were, their ids as 16-byte strings; the writer refuses what the reader
//! would; the reader's errors name their places.

use kentos_kcad::Code;
use kentos_kcad::contracts::{DocumentSnapshotV2, EntityId, LayerFilter, LayerNodeType};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says (one byte below 24, else the byte after 0x18).
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

fn filtered(filter: LayerFilter) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    doc.layers[0].filter = Some(filter);
    doc
}

fn condition(text: &str) -> LayerFilter {
    LayerFilter {
        expression: Some(text.into()),
        objects: Vec::new(),
    }
}

#[test]
fn only_a_drawing_with_a_filter_is_schema_35() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("filters.kcad");
    assert_eq!(schema(&data), 35);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    assert_eq!(of(&filtered(condition("Nitelik = 'Arsa'"))), 35);
    // The newest drawings before it keep their schemas.
    assert_eq!(of(&content("scenarios.json")), 34);
    assert_eq!(of(&content("networks.json")), 33);
}

#[test]
fn filters_come_back_as_they_were() {
    let doc = content("filters.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let parcels = again.layers[0].filter.as_ref().expect("a filter");
    assert_eq!(
        parcels.expression.as_deref(),
        Some("Nitelik = 'Arsa' ve $alan > 500")
    );
    assert_eq!(parcels.objects.len(), 2);
    assert!(
        again.layers[2]
            .filter
            .as_ref()
            .is_some_and(|f| f.expression.is_none())
    );
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |doc: DocumentSnapshotV2| kentos_kcad::encode(&doc).expect_err("refused");
    let one = EntityId::parse("0192f5a1-7777-7000-8000-000000000601").expect("an id");
    let mut on_group = filtered(condition("Kat > 2"));
    on_group.layers[0].kind = LayerNodeType::Group;
    for (doc, words) in [
        (
            filtered(LayerFilter::default()),
            "ne ifade ne nesne listesi",
        ),
        (filtered(condition(" Kat > 2")), "boşluk"),
        (
            filtered(condition(&"a".repeat(10_001))),
            "10000 karakterden uzun",
        ),
        (
            filtered(LayerFilter {
                expression: None,
                objects: vec![one, one],
            }),
            "iki kez",
        ),
        (on_group, "grup"),
    ] {
        let e = refused(doc);
        assert_eq!(e.code, Code::BadValue, "{e}");
        assert!(e.message.contains(words), "{words}: {e}");
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused =
        |name: &str| kentos_kcad::decode(&read(name)).expect_err(&format!("{name} is refused"));
    for (file, code, place) in [
        (
            "broken/filter-in-schema-34.kcad",
            Code::UnknownField,
            "layers/0/filter",
        ),
        (
            "broken/filter-on-group.kcad",
            Code::BadValue,
            "grubun süzgeci",
        ),
        (
            "broken/filter-on-service-layer.kcad",
            Code::BadValue,
            "servisten çizilir",
        ),
        (
            "broken/filter-empty.kcad",
            Code::BadValue,
            "ne ifade ne nesne listesi",
        ),
        (
            "broken/filter-expression-blank.kcad",
            Code::BadValue,
            "boşluk",
        ),
        (
            "broken/filter-expression-long.kcad",
            Code::BadValue,
            "karakterden uzun",
        ),
        (
            "broken/filter-objects-empty.kcad",
            Code::BadValue,
            "boş liste yazılmaz",
        ),
        (
            "broken/filter-objects-twice.kcad",
            Code::BadValue,
            "iki kez",
        ),
        (
            "broken/filter-object-short.kcad",
            Code::BadValue,
            "layers/0/filter/objects/0",
        ),
        (
            "broken/filter-object-nil.kcad",
            Code::BadValue,
            "boş (sıfır)",
        ),
        (
            "broken/filter-unknown-field.kcad",
            Code::UnknownField,
            "layers/0/filter/scope",
        ),
        ("broken/schema-version-37.kcad", Code::SchemaVersion, ""),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}
