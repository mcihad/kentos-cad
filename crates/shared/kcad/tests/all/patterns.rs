//! Line types with patterns in the project file (docs/adr/0313 §1): document schema 56 is written only for a drawing
//! that has a line type, a line type scale or a pattern named by a layer, a state or an object; they come back as
//! written, through the codec and the typed columns; the writer refuses a scale of 1 and two types of one name; the
//! reader's errors name their places.

use kentos_kcad::contracts::DocumentSnapshotV2;
use kentos_kcad::{Code, Quiet};

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
fn the_sample_reads_to_the_contracts_value() {
    let doc = kentos_kcad::decode(&read("patterns.kcad")).expect("reads");
    assert_eq!(doc, content("patterns.json"));
    let s = &doc.settings;
    assert_eq!(s.line_types.len(), 3);
    assert_eq!(s.line_type_scale, Some(2.5));
    assert!(s.line_types[0].elements[2].text.is_some());
    assert!(
        s.line_types[1].elements[2]
            .shape
            .as_ref()
            .is_some_and(|x| x.place.absolute)
    );
    assert_eq!(doc.layers[0].style.line_pattern.as_deref(), Some("GAZ"));
    assert_eq!(
        doc.entities[1].base().look.line_pattern.as_deref(),
        Some("KESIK-İNCE")
    );
    assert_eq!(
        doc.blocks[0].entities[0]
            .base()
            .look
            .line_pattern
            .as_deref(),
        Some("SINIR")
    );
}

#[test]
fn the_patterns_come_back_through_the_codec_and_the_columns() {
    let doc = content("patterns.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(back, doc);
    let (head, cols) = kentos_kcad::split(back).expect("splits");
    let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(again == bytes);
}

#[test]
fn only_a_drawing_with_a_pattern_is_schema_56_and_others_keep_their_bytes() {
    let data = read("patterns.kcad");
    assert_eq!(schema(&data), 56);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    // Each in turn is enough for schema 56.
    let none = |doc: &mut DocumentSnapshotV2| {
        doc.settings.line_types.clear();
        doc.settings.line_type_scale = None;
        doc.layers[0].style.line_pattern = None;
        doc.settings.layer_states.clear();
        doc.entities[1].base_mut().look.line_pattern = None;
        doc.blocks[0].entities[0].base_mut().look.line_pattern = None;
    };
    none(&mut doc);
    assert!(schema(&kentos_kcad::encode_verified(&doc).expect("writes")) < 56);
    let mut one = doc.clone();
    one.settings.line_type_scale = Some(3.0);
    assert_eq!(
        schema(&kentos_kcad::encode_verified(&one).expect("writes")),
        56
    );
    let mut one = doc.clone();
    one.blocks[0].entities[0].base_mut().look.line_pattern = Some("X".into());
    assert_eq!(
        schema(&kentos_kcad::encode_verified(&one).expect("writes")),
        56
    );
    let file = read("frozen.kcad");
    let older = kentos_kcad::decode(&file).expect("reads");
    assert!(kentos_kcad::encode(&older).expect("writes") == file);
}

#[test]
fn the_writer_refuses_a_scale_of_one_and_a_name_twice() {
    let mut doc = content("patterns.json");
    doc.settings.line_type_scale = Some(1.0);
    let err = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(err.code, Code::BadValue, "{err}");
    let mut doc = content("patterns.json");
    let mut twin = doc.settings.line_types[0].clone();
    twin.name = "gaz".into();
    doc.settings.line_types.push(twin);
    let err = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(err.code, Code::BadValue, "{err}");
    assert!(err.message.contains("iki kez"), "{err}");
}

#[test]
fn the_readers_errors_name_their_places() {
    for (file, code, place) in [
        (
            "broken/patterns-in-schema-55.kcad",
            Code::UnknownField,
            "linePattern",
        ),
        (
            "broken/line-type-scale-in-schema-55.kcad",
            Code::UnknownField,
            "lineTypeScale",
        ),
        (
            "broken/line-type-scale-one.kcad",
            Code::BadValue,
            "1 yazılmaz",
        ),
        ("broken/line-types-empty.kcad", Code::BadValue, "boş liste"),
        (
            "broken/line-types-same-name.kcad",
            Code::BadValue,
            "iki kez",
        ),
        ("broken/schema-version-106.kcad", Code::SchemaVersion, ""),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
