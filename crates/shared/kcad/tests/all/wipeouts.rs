//! An area's mask in the project file (docs/adr/0310 §1): document schema 54 is written only for a drawing that has
//! one, a block definition's too; it comes back as written, through the codec and the typed columns; the writer refuses
//! a mask on another kind; the reader's errors name their places.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity, Wipeout};
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
    let doc = kentos_kcad::decode(&read("wipeouts.kcad")).expect("reads");
    assert_eq!(doc, content("wipeouts.json"));
    let masks: Vec<Option<Wipeout>> = doc.entities.iter().map(|e| e.base().look.wipeout).collect();
    assert_eq!(
        masks,
        [
            None,
            None,
            Some(Wipeout { frame: true }),
            Some(Wipeout { frame: false }),
            None
        ]
    );
    assert_eq!(
        doc.blocks[0].entities[0].base().look.wipeout,
        Some(Wipeout { frame: false }),
        "a block's piece"
    );
}

#[test]
fn the_masks_come_back_through_the_codec_and_the_columns() {
    let mut doc = content("wipeouts.json");
    for frame in [true, false] {
        doc.entities[3].base_mut().look.wipeout = Some(Wipeout { frame });
        let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
        let back = kentos_kcad::decode(&bytes).expect("reads");
        assert_eq!(back, doc, "frame {frame}");
        let (head, cols) = kentos_kcad::split(back).expect("splits");
        let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
        assert!(again == bytes, "frame {frame}");
    }
}

#[test]
fn only_a_drawing_with_a_mask_is_schema_54_and_others_keep_their_bytes() {
    let data = read("wipeouts.kcad");
    assert_eq!(schema(&data), 54);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    for e in &mut doc.entities {
        e.base_mut().look.wipeout = None;
    }
    assert_eq!(
        schema(&kentos_kcad::encode_verified(&doc).expect("writes")),
        54,
        "the block's piece still masks"
    );
    doc.blocks[0].entities[0].base_mut().look.wipeout = None;
    assert!(schema(&kentos_kcad::encode_verified(&doc).expect("writes")) < 54);
    let file = read("arrays.kcad");
    let older = kentos_kcad::decode(&file).expect("reads");
    assert!(kentos_kcad::encode(&older).expect("writes") == file);
}

#[test]
fn the_writer_refuses_a_mask_on_another_kind() {
    let mut doc = content("wipeouts.json");
    let Entity::Line(line) = &mut doc.entities[1] else {
        panic!("a line")
    };
    line.base.look.wipeout = Some(Wipeout { frame: true });
    let err = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(err.code, Code::BadValue, "{err}");
    assert!(err.message.contains("yalnız alanda"), "{err}");
}

#[test]
fn the_readers_errors_name_their_places() {
    for (file, code, place) in [
        (
            "broken/wipeouts-in-schema-53.kcad",
            Code::UnknownField,
            "wipeout",
        ),
        (
            "broken/wipeout-on-a-line.kcad",
            Code::BadValue,
            "yalnız alanda",
        ),
        (
            "broken/wipeout-without-frame.kcad",
            Code::MissingField,
            "frame",
        ),
        (
            "broken/wipeout-frame-not-bool.kcad",
            Code::WrongType,
            "frame",
        ),
        (
            "broken/wipeout-unknown-field.kcad",
            Code::UnknownField,
            "fill",
        ),
        ("broken/schema-version-58.kcad", Code::SchemaVersion, ""),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
