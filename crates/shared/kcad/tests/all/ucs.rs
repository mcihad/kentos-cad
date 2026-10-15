//! The user coordinate systems in the project file (docs/adr/0261 §1, docs/specs/kcad-v2.md §6.4.11): document
//! schema 49 is written only for a drawing whose current UCS is not the world's or that keeps a named one; they
//! come back as written, through the codec and the typed columns; the writer refuses what the reader would; the
//! reader's errors name their places.

use kentos_kcad::contracts::{DocumentSnapshotV2, NamedUcs, Ucs, Vec2};
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
    let doc = kentos_kcad::decode(&read("ucs.kcad")).expect("reads");
    assert_eq!(doc, content("ucs.json"));
    let u = doc.settings.ucs.expect("a UCS");
    assert_eq!((u.origin.x, u.origin.y), (12.5, -4.0));
    assert_eq!(u.angle, std::f64::consts::PI / 6.0);
    let names: Vec<&str> = doc
        .settings
        .named_ucs
        .iter()
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(names, ["Bina", "Yol ekseni"]);
}

#[test]
fn the_ucs_comes_back_through_the_codec_and_the_columns() {
    let mut doc = content("ucs.json");
    for angle in [1e-300, 6.283185307179585, 3.0] {
        doc.settings.ucs = Some(Ucs {
            origin: Vec2 {
                x: -1.0e7,
                y: 1.0e7,
            },
            angle,
        });
        let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
        let back = kentos_kcad::decode(&bytes).expect("reads");
        assert_eq!(back, doc, "{angle}");
        let (head, cols) = kentos_kcad::split(back).expect("splits");
        let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
        assert!(again == bytes, "{angle}");
    }
}

#[test]
fn only_a_drawing_with_a_ucs_is_schema_49_and_others_keep_their_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("ucs.kcad");
    assert_eq!(schema(&data), 49);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    doc.settings.ucs = None;
    // The named ones still make it 49.
    assert_eq!(of(&doc), 49);
    doc.settings.named_ucs.clear();
    assert!(of(&doc) < 49);
    let file = read("look.kcad");
    assert_eq!(schema(&file), 48);
    let look = kentos_kcad::decode(&file).expect("reads");
    assert!(kentos_kcad::encode(&look).expect("writes") == file);
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |f: &dyn Fn(&mut DocumentSnapshotV2)| {
        let mut doc = content("ucs.json");
        f(&mut doc);
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    let named = |name: &str| NamedUcs {
        name: name.into(),
        origin: Vec2 { x: 1.0, y: 2.0 },
        angle: 0.5,
    };
    for (err, words) in [
        (refused(&|d| d.settings.ucs = Some(Ucs::WORLD)), "Dünya"),
        (
            refused(&|d| {
                d.settings.ucs = Some(Ucs {
                    origin: Vec2 { x: 0.0, y: 0.0 },
                    angle: 7.0,
                })
            }),
            "açısı",
        ),
        (
            refused(&|d| d.settings.named_ucs = vec![named("A"), named("A")]),
            "iki UCS",
        ),
        (
            refused(&|d| d.settings.named_ucs = vec![named(" A")]),
            "UCS adı",
        ),
    ] {
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains(words), "{err}");
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    for (file, code, place) in [
        ("broken/ucs-in-schema-48.kcad", Code::UnknownField, "ucs"),
        ("broken/ucs-world.kcad", Code::BadValue, "Dünya"),
        ("broken/ucs-angle-tau.kcad", Code::BadValue, "açısı"),
        ("broken/ucs-far.kcad", Code::BadValue, "10 000 km"),
        ("broken/named-ucs-empty.kcad", Code::BadValue, "boş"),
        ("broken/named-ucs-twice.kcad", Code::BadValue, "iki UCS"),
        ("broken/named-ucs-blank.kcad", Code::BadValue, "UCS adı"),
        (
            "broken/named-ucs-unknown-field.kcad",
            Code::UnknownField,
            "namedUcs/0/note",
        ),
        ("broken/schema-version-54.kcad", Code::SchemaVersion, ""),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
