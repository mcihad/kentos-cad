//! A layer's freeze and a layer state's freeze and printing in the project file (docs/adr/0311 §1, §3): document
//! schema 55 is written only for a drawing that has a frozen node or a state keeping them; they come back as written,
//! through the codec and the typed columns; the writer refuses `frozen: false` and a state node's path; the reader's
//! errors name their places.

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
    let doc = kentos_kcad::decode(&read("frozen.kcad")).expect("reads");
    assert_eq!(doc, content("frozen.json"));
    let frozen: Vec<Option<bool>> = doc.layers.iter().map(|n| n.frozen).collect();
    assert_eq!(frozen, [None, Some(true), None, Some(true)]);
    let state = &doc.settings.layer_states[0];
    assert_eq!(state.nodes[1].frozen, Some(true));
    assert_eq!(state.nodes[2].plot, Some(false));
    let older = &doc.settings.layer_states[1];
    assert_eq!(
        (older.nodes[0].frozen, older.nodes[0].plot),
        (None, None),
        "a state saved before keeps neither"
    );
}

#[test]
fn the_freezes_come_back_through_the_codec_and_the_columns() {
    let mut doc = content("frozen.json");
    for frozen in [Some(true), None] {
        doc.layers[0].frozen = frozen;
        let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
        let back = kentos_kcad::decode(&bytes).expect("reads");
        assert_eq!(back, doc, "{frozen:?}");
        let (head, cols) = kentos_kcad::split(back).expect("splits");
        let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
        assert!(again == bytes, "{frozen:?}");
    }
}

#[test]
fn only_a_drawing_with_a_freeze_is_schema_55_and_others_keep_their_bytes() {
    let data = read("frozen.kcad");
    assert_eq!(schema(&data), 55);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    doc.layers[1].frozen = None;
    doc.layers[3].frozen = None;
    assert_eq!(
        schema(&kentos_kcad::encode_verified(&doc).expect("writes")),
        55,
        "a state still keeps freezes"
    );
    for n in doc
        .settings
        .layer_states
        .iter_mut()
        .flat_map(|s| &mut s.nodes)
    {
        n.frozen = None;
        n.plot = None;
    }
    assert!(schema(&kentos_kcad::encode_verified(&doc).expect("writes")) < 55);
    let file = read("wipeouts.kcad");
    let older = kentos_kcad::decode(&file).expect("reads");
    assert!(kentos_kcad::encode(&older).expect("writes") == file);
}

#[test]
fn the_writer_refuses_frozen_false_and_a_path() {
    let mut doc = content("frozen.json");
    doc.layers[0].frozen = Some(false);
    let err = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(err.code, Code::BadValue, "{err}");
    assert!(err.message.contains("yalnız true"), "{err}");
    let mut doc = content("frozen.json");
    doc.settings.layer_states[0].nodes[0].path = Some("A-DUVAR".into());
    let err = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(err.code, Code::BadValue, "{err}");
    assert!(err.message.contains("yol"), "{err}");
}

#[test]
fn the_readers_errors_name_their_places() {
    for (file, code, place) in [
        (
            "broken/frozen-in-schema-54.kcad",
            Code::UnknownField,
            "frozen",
        ),
        ("broken/frozen-false.kcad", Code::BadValue, "yalnız true"),
        (
            "broken/layer-states-frozen-in-schema-54.kcad",
            Code::UnknownField,
            "frozen",
        ),
        ("broken/layer-states-path.kcad", Code::UnknownField, "path"),
        ("broken/schema-version-120.kcad", Code::SchemaVersion, ""),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
