//! A dimension's references in the project file (docs/adr/0249 §4, docs/specs/kcad-v2.md §6.6): document schema 43
//! is written only for a drawing with a dimension that has one; every other drawing keeps its schema and its bytes.
//! The references come back as written, through the codec and the typed columns; the writer refuses what the reader
//! would; the reader's errors name their places.

use kentos_kcad::contracts::{
    DimensionRefs, DocumentSnapshotV2, Entity, EntityId, SubPlace, SubRef,
};
use kentos_kcad::{Code, Quiet};

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

fn refs_of(doc: &DocumentSnapshotV2, i: usize) -> Option<&DimensionRefs> {
    match &doc.entities[i] {
        Entity::Dimension(d) => d.refs.as_deref(),
        other => panic!("{i}: {other:?}"),
    }
}

fn refs_mut(doc: &mut DocumentSnapshotV2, i: usize) -> &mut Option<Box<DimensionRefs>> {
    match &mut doc.entities[i] {
        Entity::Dimension(d) => &mut d.refs,
        other => panic!("{i}: {other:?}"),
    }
}

const PARCEL: EntityId = EntityId([
    0x01, 0x92, 0xf5, 0xa0, 0x7c, 0x3e, 0x7d, 0x4a, 0x9b, 0x1e, 0x4c, 0x2f, 0x8a, 0x6d, 0x3e, 0x21,
]);

#[test]
fn the_sample_reads_to_the_contracts_value() {
    let doc = kentos_kcad::decode(&read("dimension-refs.kcad")).expect("reads");
    assert_eq!(doc, content("dimension-refs.json"));
    let aligned = refs_of(&doc, 1).expect("refs");
    assert_eq!(
        aligned.a,
        Some(SubRef {
            uid: PARCEL,
            place: SubPlace::Vertex {
                part: 0,
                ring: 0,
                index: 0
            },
            path: Vec::new(),
            broken: None,
        })
    );
    let b = aligned.b.as_ref().expect("b");
    assert_eq!(b.broken.as_deref(), Some("vertex_gone"));
    assert!(aligned.c.is_none());
    let linear = refs_of(&doc, 2).expect("refs");
    assert!(linear.a.is_none() && linear.c.is_none());
    assert_eq!(
        linear.b.as_ref().map(|r| r.place),
        Some(SubPlace::Mid {
            part: 0,
            ring: 0,
            index: 2
        })
    );
}

#[test]
fn every_place_comes_back_through_the_codec_and_the_columns() {
    let mut doc = content("dimension-refs.json");
    let places = [
        SubPlace::Node { part: 3 },
        SubPlace::Start,
        SubPlace::End,
        SubPlace::Center,
        SubPlace::Vertex {
            part: 1,
            ring: 2,
            index: 10_000_000,
        },
        SubPlace::Mid {
            part: 0,
            ring: 0,
            index: 0,
        },
    ];
    for (i, place) in places.into_iter().enumerate() {
        let r = |broken: Option<&str>| SubRef {
            uid: PARCEL,
            place,
            path: Vec::new(),
            broken: broken.map(str::to_owned),
        };
        *refs_mut(&mut doc, 2) = Some(Box::new(DimensionRefs {
            a: Some(r(None)),
            b: (i % 2 == 0).then(|| r(Some("ambiguous"))),
            c: Some(r(Some("object_gone"))),
            rule: [
                None,
                Some(kentos_contracts::DimensionRule::Curve),
                Some(kentos_contracts::DimensionRule::Edges),
            ][i % 3],
        }));
        let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
        let back = kentos_kcad::decode(&bytes).expect("reads");
        assert_eq!(back, doc, "{place:?}");
        let (head, cols) = kentos_kcad::split(back).expect("splits");
        let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
        assert!(again == bytes, "{place:?}");
    }
}

#[test]
fn only_a_drawing_with_references_is_schema_43_and_others_keep_their_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("dimension-refs.kcad");
    assert_eq!(schema(&data), 43);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    // Without references the same drawing (an aligned and a linear dimension) is schema 2's.
    *refs_mut(&mut doc, 1) = None;
    *refs_mut(&mut doc, 2) = None;
    assert_eq!(of(&doc), 2);
    // The newest drawings before it keep their schemas and their bytes.
    for (name, n) in [("addresses", 42), ("compose", 41), ("dimensions", 9)] {
        let file = read(&format!("{name}.kcad"));
        assert_eq!(schema(&file), n, "{name}");
        let doc = kentos_kcad::decode(&file).expect("reads");
        assert!(kentos_kcad::encode(&doc).expect("writes") == file, "{name}");
    }
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |f: &dyn Fn(&mut DimensionRefs)| {
        let mut doc = content("dimension-refs.json");
        f(refs_mut(&mut doc, 1).as_mut().expect("refs"));
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    for (err, words) in [
        (refused(&|r| *r = DimensionRefs::default()), "boş"),
        (
            refused(&|r| r.a.as_mut().expect("a").uid = EntityId([0; 16])),
            "kimliği boş",
        ),
        (
            refused(&|r| r.a.as_mut().expect("a").broken = Some("lost".into())),
            "bilinmiyor",
        ),
        (
            refused(&|r| {
                r.a.as_mut().expect("a").place = SubPlace::Vertex {
                    part: 0,
                    ring: 0,
                    index: 10_000_001,
                }
            }),
            "çok büyük",
        ),
    ] {
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains("dimension/refs"), "{err}");
        assert!(err.message.contains(words), "{err}");
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/dimension-refs-in-schema-42.kcad",
            Code::UnknownField,
            "dimension/refs",
        ),
        (
            "broken/dimension-refs-in-block.kcad",
            Code::UnknownField,
            "dimension/refs",
        ),
        (
            "broken/dimension-refs-unknown-field.kcad",
            Code::UnknownField,
            "refs/d",
        ),
        ("broken/dimension-refs-empty.kcad", Code::BadValue, "boş"),
        (
            "broken/dimension-refs-zero-uid.kcad",
            Code::BadValue,
            "refs/a/uid",
        ),
        (
            "broken/dimension-refs-unknown-kind.kcad",
            Code::BadValue,
            "place/kind",
        ),
        (
            "broken/dimension-refs-unknown-broken.kcad",
            Code::BadValue,
            "lost",
        ),
        ("broken/schema-version-152.kcad", Code::SchemaVersion, ""),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}
