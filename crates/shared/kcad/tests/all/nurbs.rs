//! A NURBS curve in the project file (docs/adr/0259, docs/specs/kcad-v2.md §6.6): document schema 47 is written only
//! for a drawing with one, in the drawing or a block definition; every other drawing keeps its schema and its bytes.
//! The curve comes back as written, through the codec and the typed columns; the writer refuses what the reader
//! would; the reader's errors name their places.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity, NurbsEntity, Vec2};
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

fn curve_mut(doc: &mut DocumentSnapshotV2, i: usize) -> &mut NurbsEntity {
    match &mut doc.entities[i] {
        Entity::Nurbs(n) => n,
        other => panic!("{i}: {other:?}"),
    }
}

#[test]
fn the_sample_reads_to_the_contracts_value() {
    let doc = kentos_kcad::decode(&read("nurbs.kcad")).expect("reads");
    assert_eq!(doc, content("nurbs.json"));
    let Entity::Nurbs(arc) = &doc.entities[0] else {
        panic!("{:?}", doc.entities[0])
    };
    assert_eq!(arc.weights.as_ref().map(Vec::len), Some(3));
    assert_eq!(arc.tangents, Some([Some(Vec2 { x: 0.0, y: 1.0 }), None]));
    let Entity::Nurbs(ring) = &doc.entities[2] else {
        panic!("{:?}", doc.entities[2])
    };
    assert!(ring.closed && ring.periodic);
    assert!(matches!(&doc.blocks[0].entities[0], Entity::Nurbs(_)));
}

#[test]
fn the_curve_comes_back_through_the_codec_and_the_columns() {
    let mut doc = content("nurbs.json");
    for weights in [
        Some(vec![1.0, 0.1 + 0.2, f64::MIN_POSITIVE]),
        Some(vec![1e300, 1.0, 1.0]),
        None,
    ] {
        let c = curve_mut(&mut doc, 0);
        c.weights = weights.clone();
        c.tangents = Some([None, Some(Vec2 { x: -0.6, y: 0.8 })]);
        c.fit_tolerance = Some(0.0);
        let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
        let back = kentos_kcad::decode(&bytes).expect("reads");
        assert_eq!(back, doc, "{weights:?}");
        let (head, cols) = kentos_kcad::split(back).expect("splits");
        let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
        assert!(again == bytes, "{weights:?}");
    }
}

#[test]
fn only_a_drawing_with_a_nurbs_curve_is_schema_47_and_others_keep_their_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("nurbs.kcad");
    assert_eq!(schema(&data), 47);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    // Without them in the drawing, the block definition's still makes it 47.
    // The insert alone stays, with its persistent id.
    doc.entities.drain(..4);
    doc.uids.drain(..4);
    assert_eq!(of(&doc), 47);
    doc.blocks[0].entities.clear();
    assert!(of(&doc) < 47);
    // The newest drawings before it keep their schemas and their bytes.
    for (name, n) in [("polyline-extras", 46), ("requests", 45), ("sheets", 44)] {
        let file = read(&format!("{name}.kcad"));
        assert_eq!(schema(&file), n, "{name}");
        let doc = kentos_kcad::decode(&file).expect("reads");
        assert!(kentos_kcad::encode(&doc).expect("writes") == file, "{name}");
    }
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |f: &dyn Fn(&mut NurbsEntity)| {
        let mut doc = content("nurbs.json");
        f(curve_mut(&mut doc, 1));
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    for (err, words) in [
        (refused(&|c| c.degree = 12), "Derece"),
        (
            refused(&|c| c.knots.pop().map(|_| ()).unwrap_or(())),
            "Düğüm sayısı",
        ),
        (refused(&|c| c.knots.swap(4, 5)), "azalmamalı"),
        (
            refused(&|c| c.weights = Some(vec![1.0; 5])),
            "Ağırlık sayısı",
        ),
        (
            refused(&|c| c.weights = Some(vec![-1.0; 6])),
            "sıfırdan büyük",
        ),
        (refused(&|c| c.fit_tolerance = Some(-1.0)), "tolerans"),
    ] {
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains("nurbs"), "{err}");
        assert!(err.message.contains(words), "{err}");
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        ("broken/nurbs-in-schema-46.kcad", Code::UnknownKind, "nurbs"),
        (
            "broken/nurbs-closed-false.kcad",
            Code::BadValue,
            "nurbs/closed",
        ),
        (
            "broken/nurbs-degree-12.kcad",
            Code::BadValue,
            "nurbs/degree",
        ),
        (
            "broken/nurbs-knot-count.kcad",
            Code::BadValue,
            "Düğüm sayısı",
        ),
        (
            "broken/nurbs-knots-decreasing.kcad",
            Code::BadValue,
            "azalmamalı",
        ),
        ("broken/nurbs-inner-knot.kcad", Code::BadValue, "İç düğüm"),
        (
            "broken/nurbs-weight-zero.kcad",
            Code::BadValue,
            "Ağırlıklar",
        ),
        (
            "broken/nurbs-weight-count.kcad",
            Code::BadValue,
            "Ağırlık sayısı",
        ),
        (
            "broken/nurbs-knot-nan.kcad",
            Code::NonFinite,
            "nurbs/knots/3",
        ),
        (
            "broken/nurbs-tangents-three.kcad",
            Code::BadValue,
            "nurbs/tangents",
        ),
        ("broken/schema-version-92.kcad", Code::SchemaVersion, ""),
    ] {
        let err = refused(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
