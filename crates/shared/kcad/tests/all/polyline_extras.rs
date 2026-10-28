//! A closed and a wide polyline in the project file (docs/adr/0258, docs/specs/kcad-v2.md §6.6): document schema 46
//! is written only for a drawing with such a polyline, in the drawing or a block definition; every other drawing keeps
//! its schema and its bytes. `closed` and `widths` come back as written, through the codec and the typed columns; the
//! writer refuses what the reader would; the reader's errors name their places.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity, PathEntity};
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

fn path_mut(doc: &mut DocumentSnapshotV2, i: usize) -> &mut PathEntity {
    match &mut doc.entities[i] {
        Entity::Polyline(p) | Entity::Polygon(p) => p,
        other => panic!("{i}: {other:?}"),
    }
}

#[test]
fn the_sample_reads_to_the_contracts_value() {
    let doc = kentos_kcad::decode(&read("polyline-extras.kcad")).expect("reads");
    assert_eq!(doc, content("polyline-extras.json"));
    let Entity::Polyline(closed) = &doc.entities[1] else {
        panic!("{:?}", doc.entities[1])
    };
    assert!(closed.closed);
    // The closing edge's bulge is the last of `bulges`.
    assert_eq!(closed.bulges.as_ref().map(Vec::len), Some(3));
    let Entity::Polyline(wide) = &doc.entities[2] else {
        panic!("{:?}", doc.entities[2])
    };
    assert_eq!(wide.widths, Some(vec![[0.0, 0.4], [0.4, 0.4], [0.0, 0.0]]));
    // A block definition's wide polyline too.
    let Entity::Polyline(column) = &doc.blocks[0].entities[0] else {
        panic!("{:?}", doc.blocks[0].entities[0])
    };
    assert!(column.closed && column.widths.is_some());
}

#[test]
fn closed_and_widths_come_back_through_the_codec_and_the_columns() {
    let mut doc = content("polyline-extras.json");
    for widths in [
        Some(vec![
            [0.1 + 0.2, 1e-9],
            [f64::MIN_POSITIVE, 1e300],
            [0.0, 0.0],
        ]),
        Some(vec![[0.0, 0.0], [-0.0, 0.25], [0.0, 0.0]]),
        None,
    ] {
        path_mut(&mut doc, 2).widths = widths.clone();
        let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
        let back = kentos_kcad::decode(&bytes).expect("reads");
        assert_eq!(back, doc, "{widths:?}");
        let (head, cols) = kentos_kcad::split(back).expect("splits");
        let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
        assert!(again == bytes, "{widths:?}");
    }
}

#[test]
fn only_a_drawing_with_a_closed_or_wide_polyline_is_schema_46_and_others_keep_their_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("polyline-extras.kcad");
    assert_eq!(schema(&data), 46);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    // Without them in the drawing, the block definition's still makes it 46.
    for i in 0..5 {
        let p = path_mut(&mut doc, i);
        p.closed = false;
        p.widths = None;
        if let Some(b) = p.bulges.as_mut() {
            b.truncate(p.pts.len() - 1);
        }
    }
    assert_eq!(of(&doc), 46);
    let Entity::Polyline(column) = &mut doc.blocks[0].entities[0] else {
        panic!("a polyline")
    };
    column.closed = false;
    column.widths = None;
    // An insert's and the corners' elevations: schema 6's.
    assert_eq!(of(&doc), 6);
    // The newest drawings before it keep their schemas and their bytes.
    for (name, n) in [("requests", 45), ("sheets", 44), ("measures", 39)] {
        let file = read(&format!("{name}.kcad"));
        assert_eq!(schema(&file), n, "{name}");
        let doc = kentos_kcad::decode(&file).expect("reads");
        assert!(kentos_kcad::encode(&doc).expect("writes") == file, "{name}");
    }
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |i: usize, f: &dyn Fn(&mut PathEntity)| {
        let mut doc = content("polyline-extras.json");
        f(path_mut(&mut doc, i));
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    for (err, words) in [
        (
            refused(2, &|p| p.widths = Some(vec![[1.0, 1.0]])),
            "Genişlik sayısı",
        ),
        (
            refused(2, &|p| {
                p.widths = Some(vec![[1.0, -1.0], [1.0, 1.0], [0.0, 0.0]])
            }),
            "geçersiz",
        ),
        (
            refused(2, &|p| p.widths = Some(vec![[1.0, 1.0]; 3])),
            "son genişlik",
        ),
        (
            refused(2, &|p| p.widths = Some(vec![[0.0, 0.0]; 3])),
            "sıfır",
        ),
        (
            refused(0, &|p| {
                p.parts = Some(vec![kentos_kcad::contracts::AreaPart {
                    pts: p.pts[..2].to_vec(),
                    bulges: None,
                    holes: None,
                    zs: None,
                    ms: None,
                }])
            }),
            "tek parçalı",
        ),
    ] {
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains("polyline"), "{err}");
        assert!(err.message.contains(words), "{err}");
    }
    // An area has neither.
    let mut doc = content("polyline-extras.json");
    let p = path_mut(&mut doc, 0).clone();
    doc.entities[0] = Entity::Polygon(p);
    let err = kentos_kcad::encode(&doc).expect_err("an area's closed");
    assert!(err.message.contains("Kapalı alanın"), "{err}");
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/polyline-closed-in-schema-45.kcad",
            Code::UnknownField,
            "polyline/closed",
        ),
        (
            "broken/polyline-widths-in-schema-45.kcad",
            Code::UnknownField,
            "polyline/widths",
        ),
        (
            "broken/polygon-closed.kcad",
            Code::UnknownField,
            "polygon/closed",
        ),
        (
            "broken/polygon-widths.kcad",
            Code::UnknownField,
            "polygon/widths",
        ),
        (
            "broken/polyline-closed-false.kcad",
            Code::BadValue,
            "false yazılmaz",
        ),
        (
            "broken/polyline-widths-count.kcad",
            Code::BadValue,
            "Genişlik sayısı",
        ),
        (
            "broken/polyline-widths-negative.kcad",
            Code::BadValue,
            "geçersiz",
        ),
        (
            "broken/polyline-widths-all-zero.kcad",
            Code::BadValue,
            "sıfır",
        ),
        (
            "broken/polyline-widths-open-last.kcad",
            Code::BadValue,
            "son genişlik",
        ),
        (
            "broken/polyline-widths-three.kcad",
            Code::BadValue,
            "2 sayı",
        ),
        ("broken/polyline-widths-nan.kcad", Code::NonFinite, "widths"),
        (
            "broken/polyline-closed-parts.kcad",
            Code::BadValue,
            "tek parçalı",
        ),
        ("broken/schema-version-127.kcad", Code::SchemaVersion, ""),
    ] {
        let err = refused(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
