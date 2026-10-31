//! The named views in the project file (docs/adr/0285 §1, docs/specs/kcad-v2.md §6.4.12): document schema 52 is
//! written only for a project that keeps a named view; they come back as written, through the codec and the typed
//! columns; the writer refuses what the reader would; the reader's errors name their places.

use kentos_kcad::contracts::{DocumentSnapshotV2, NamedView, Vec2};
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

fn view(name: &str) -> NamedView {
    NamedView {
        id: String::new(),
        name: name.into(),
        center: Vec2 { x: 1.0, y: 2.0 },
        width: 30.0,
        height: 20.0,
        twist: 0.5,
    }
}

#[test]
fn the_sample_reads_to_the_contracts_value() {
    let doc = kentos_kcad::decode(&read("views.kcad")).expect("reads");
    assert_eq!(doc, content("views.json"));
    let names: Vec<&str> = doc
        .settings
        .named_views
        .iter()
        .map(|v| v.name.as_str())
        .collect();
    assert_eq!(names, ["Bina girişi", "Bütün çizim", "Yol ekseni"]);
    assert_eq!(
        doc.settings.named_views[0].twist,
        std::f64::consts::PI / 6.0
    );
}

#[test]
fn the_views_come_back_through_the_codec_and_the_columns() {
    let mut doc = content("views.json");
    for (twist, size) in [(1e-300, 2.0e7), (6.283185307179585, 1e-9), (3.0, 0.125)] {
        doc.settings.named_views[1] = NamedView {
            id: String::new(),
            name: "Uç".into(),
            center: Vec2 {
                x: -1.0e7,
                y: 1.0e7,
            },
            width: size,
            height: size,
            twist,
        };
        let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
        let back = kentos_kcad::decode(&bytes).expect("reads");
        assert_eq!(back, doc, "{twist}");
        let (head, cols) = kentos_kcad::split(back).expect("splits");
        let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
        assert!(again == bytes, "{twist}");
    }
}

#[test]
fn only_a_project_with_a_named_view_is_schema_52_and_others_keep_their_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("views.kcad");
    assert_eq!(schema(&data), 52);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    doc.settings.named_views.truncate(1);
    assert_eq!(of(&doc), 52);
    doc.settings.named_views.clear();
    assert!(of(&doc) < 52);
    let file = read("subref-extras.kcad");
    assert_eq!(schema(&file), 51);
    let older = kentos_kcad::decode(&file).expect("reads");
    assert!(kentos_kcad::encode(&older).expect("writes") == file);
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |f: &dyn Fn(&mut DocumentSnapshotV2)| {
        let mut doc = content("views.json");
        f(&mut doc);
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    for (err, words) in [
        (
            refused(&|d| d.settings.named_views = vec![view("A"), view("A")]),
            "iki görünüm",
        ),
        (
            refused(&|d| d.settings.named_views = vec![view(" A")]),
            "görünüm adı",
        ),
        (
            refused(&|d| d.settings.named_views[0].width = 0.0),
            "eni ve boyu",
        ),
        (
            refused(&|d| d.settings.named_views[0].twist = 7.0),
            "dönüşü",
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
            "broken/views-in-schema-51.kcad",
            Code::UnknownField,
            "namedViews",
        ),
        ("broken/named-views-empty.kcad", Code::BadValue, "boş"),
        (
            "broken/named-views-twice.kcad",
            Code::BadValue,
            "iki görünüm",
        ),
        (
            "broken/named-views-blank.kcad",
            Code::BadValue,
            "görünüm adı",
        ),
        (
            "broken/named-views-width-zero.kcad",
            Code::BadValue,
            "eni ve boyu",
        ),
        (
            "broken/named-views-twist-tau.kcad",
            Code::BadValue,
            "dönüşü",
        ),
        ("broken/named-views-far.kcad", Code::BadValue, "10 000 km"),
        (
            "broken/named-views-unknown-field.kcad",
            Code::UnknownField,
            "namedViews/0/note",
        ),
        (
            "broken/named-views-no-twist.kcad",
            Code::MissingField,
            "twist",
        ),
        ("broken/schema-version-140.kcad", Code::SchemaVersion, ""),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
