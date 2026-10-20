//! Inches, feet and the length style in the project file (docs/adr/0262 §4): document schema 50 is written only
//! for an inch or foot drawing or one with a style; it comes back as written; the writer refuses what the reader
//! would; the reader's errors name their places.

use kentos_kcad::Code;
use kentos_kcad::contracts::{DocumentSnapshotV2, DrawingUnit, LengthStyle};

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
fn the_sample_reads_and_is_schema_50() {
    let data = read("imperial.kcad");
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert_eq!(doc, content("imperial.json"));
    assert_eq!(doc.settings.drawing_unit, Some(DrawingUnit::In));
    assert_eq!(doc.settings.length_style, Some(LengthStyle::Architectural));
    assert_eq!(schema(&data), 50);
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
}

#[test]
fn feet_without_a_style_are_schema_50_and_metres_keep_their_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let mut doc = content("imperial.json");
    doc.settings.drawing_unit = Some(DrawingUnit::Ft);
    doc.settings.length_style = None;
    assert_eq!(of(&doc), 50);
    let back =
        kentos_kcad::decode(&kentos_kcad::encode_verified(&doc).expect("writes")).expect("reads");
    assert_eq!(back, doc);
    doc.settings.drawing_unit = Some(DrawingUnit::Mm);
    assert!(of(&doc) < 50);
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    for (unit, style) in [
        (Some(DrawingUnit::Mm), LengthStyle::Fractional),
        (Some(DrawingUnit::In), LengthStyle::Decimal),
        (None, LengthStyle::Architectural),
    ] {
        let mut doc = content("imperial.json");
        doc.settings.drawing_unit = unit;
        doc.settings.length_style = Some(style);
        let err = kentos_kcad::encode(&doc).expect_err("refused");
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains("ayak ve inç"), "{err}");
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    for (file, code, place) in [
        (
            "broken/drawing-unit-in-in-schema-49.kcad",
            Code::BadValue,
            "drawingUnit",
        ),
        (
            "broken/length-style-in-schema-49.kcad",
            Code::UnknownField,
            "lengthStyle",
        ),
        (
            "broken/length-style-decimal.kcad",
            Code::BadValue,
            "lengthStyle",
        ),
        (
            "broken/length-style-metric.kcad",
            Code::BadValue,
            "lengthStyle",
        ),
        ("broken/schema-version-91.kcad", Code::SchemaVersion, ""),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
