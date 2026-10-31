//! The associative arrays in the project file (docs/adr/0300 §1, docs/specs/kcad-v2.md §6.14): document schema 53 is
//! written only for a drawing that has one; they come back as written, through the codec and the typed columns; the
//! writer refuses what the reader would; the reader's errors name their places.

use kentos_kcad::contracts::{ArrayItem, ArrayLayout, BlockId, DocumentSnapshotV2};
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
    let doc = kentos_kcad::decode(&read("arrays.kcad")).expect("reads");
    assert_eq!(doc, content("arrays.json"));
    assert_eq!(doc.arrays.len(), 3);
    assert_eq!(
        doc.arrays[0].items[2],
        ArrayItem::default(),
        "a removed item"
    );
    assert!(doc.arrays[0].items[3].kept, "a kept item");
    assert!(doc.arrays[1].mirror);
    assert!(matches!(
        doc.arrays[2].layout,
        ArrayLayout::Path {
            spacing: Some(25.0),
            ..
        }
    ));
}

#[test]
fn the_arrays_come_back_through_the_codec_and_the_columns() {
    let mut doc = content("arrays.json");
    for layout in [
        ArrayLayout::Line {
            count: 4,
            dx: 1e-9,
            dy: -2.5e6,
            fit: true,
        },
        ArrayLayout::Grid {
            rows: 2,
            cols: 2,
            dx: 3.0,
            dy: -4.0,
        },
    ] {
        doc.arrays[0].layout = layout.clone();
        let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
        let back = kentos_kcad::decode(&bytes).expect("reads");
        assert_eq!(back, doc, "{layout:?}");
        let (head, cols) = kentos_kcad::split(back).expect("splits");
        let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
        assert!(again == bytes, "{layout:?}");
    }
}

#[test]
fn only_a_drawing_with_an_array_is_schema_53_and_others_keep_their_bytes() {
    let data = read("arrays.kcad");
    assert_eq!(schema(&data), 53);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    doc.arrays.clear();
    assert!(schema(&kentos_kcad::encode_verified(&doc).expect("writes")) < 53);
    let file = read("views.kcad");
    let older = kentos_kcad::decode(&file).expect("reads");
    assert!(kentos_kcad::encode(&older).expect("writes") == file);
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |f: &dyn Fn(&mut DocumentSnapshotV2)| {
        let mut doc = content("arrays.json");
        f(&mut doc);
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    for (err, words) in [
        (
            refused(&|d| d.arrays[0].block = BlockId([9; 16])),
            "bloğu çizimde",
        ),
        (
            refused(&|d| {
                d.arrays[0].items.pop();
            }),
            "her yerinin",
        ),
        (refused(&|d| d.arrays[1].scale = 0.0), "ölçeği"),
        (refused(&|d| d.arrays[1].id = d.arrays[0].id), "iki kez"),
        (
            refused(&|d| d.arrays[0].items[2].uid = d.arrays[0].items[0].uid),
            "iki öğede",
        ),
    ] {
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains(words), "{err}");
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    for (file, code, place) in [
        (
            "broken/arrays-in-schema-52.kcad",
            Code::UnknownField,
            "arrays",
        ),
        (
            "broken/arrays-block-unknown.kcad",
            Code::BadValue,
            "arrays/0",
        ),
        (
            "broken/arrays-items-short.kcad",
            Code::BadValue,
            "her yerinin",
        ),
        (
            "broken/arrays-item-twice.kcad",
            Code::BadValue,
            "arrays/0/items/2",
        ),
        (
            "broken/arrays-item-missing.kcad",
            Code::BadValue,
            "arrays/0/items/2/uid",
        ),
        ("broken/arrays-scale-zero.kcad", Code::BadValue, "ölçeği"),
        ("broken/schema-version-138.kcad", Code::SchemaVersion, ""),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
