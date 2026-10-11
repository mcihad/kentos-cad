//! Vertex measures (M) in the project file (docs/adr/0224 §1,
//! docs/specs/kcad-v2.md §6.1, §6.6): document schema 39 is written only for
//! a drawing with a line's `ma` or `mb` or a polyline's or a part's `ms`,
//! every other drawing keeps its schema and its bytes; the measures come back
//! bit for bit, `null` for a vertex without one; an area has none; the writer
//! refuses what the reader would; the typed columns carry them.

use kentos_kcad::contracts::{
    AreaPart, DocumentSnapshotV2, Entity, EntityBase, EntityId, LineEntity, PathEntity, Vec2,
};
use kentos_kcad::{Code, Quiet};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says: the unsigned integer after the
/// `version` key (one byte up to 23, past it the byte after CBOR's 0x18).
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

fn base(id: u32) -> EntityBase {
    EntityBase {
        id,
        layer_id: "0".into(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
    }
}

fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn line(ma: Option<f64>, mb: Option<f64>) -> Entity {
    Entity::Line(LineEntity {
        base: base(0),
        a: v(0.0, 0.0),
        b: v(10.0, 0.0),
        za: None,
        zb: None,
        ma,
        mb,
    })
}

fn path(ms: Option<Vec<Option<f64>>>, parts: Option<Vec<AreaPart>>) -> PathEntity {
    PathEntity {
        base: base(0),
        pts: vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0)],
        bulges: None,
        holes: None,
        zs: None,
        ms,
        parts,
    }
}

fn part(ms: Option<Vec<Option<f64>>>) -> AreaPart {
    AreaPart {
        pts: vec![v(5.0, 0.0), v(6.0, 0.0)],
        bulges: None,
        holes: None,
        zs: None,
        ms,
    }
}

/// The minimal fixture's drawing (one point, schema 2) with `more` after its point.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x4du8; 16];
        uid[15] = n as u8;
        doc.uids.push(EntityId(uid));
        e.base_mut().id = n;
        doc.entities.push(e);
    }
    doc
}

#[test]
fn only_a_drawing_with_a_measure_is_schema_39() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("measures.kcad");
    assert_eq!(schema(&data), 39);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    // Without a measure a drawing keeps its schema, byte for byte.
    assert_eq!(
        of(&drawing(vec![
            line(None, None),
            Entity::Polyline(path(None, None))
        ])),
        2
    );
    assert_eq!(of(&drawing(vec![line(None, Some(5.0))])), 39);
    // A list whose every vertex is without one is a field too, a part's as well.
    assert_eq!(
        of(&drawing(vec![Entity::Polyline(path(
            Some(vec![None; 3]),
            None
        ))])),
        39
    );
    assert_eq!(
        of(&drawing(vec![Entity::Polyline(path(
            None,
            Some(vec![part(Some(vec![Some(1.0), None]))])
        ))])),
        39
    );
    // A block definition's route is the drawing's schema too.
    let mut doc = content("blocks.json");
    doc.blocks[0].entities.push({
        let mut e = line(Some(0.0), Some(1.0));
        e.base_mut().id = 99;
        e
    });
    assert_eq!(of(&doc), 39);
}

#[test]
fn the_measures_come_back_bit_for_bit() {
    let doc = content("measures.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let Entity::Line(first) = &again.entities[0] else {
        panic!("a line")
    };
    assert_eq!((first.ma, first.mb), (Some(0.0), Some(250.0)));
    let Entity::Polyline(road) = &again.entities[2] else {
        panic!("a polyline")
    };
    assert_eq!(road.ms.as_ref().map(|m| m[2]), Some(None));
    let Entity::Polyline(parted) = &again.entities[3] else {
        panic!("a polyline")
    };
    assert_eq!(
        parted.parts.as_ref().and_then(|p| p[0].ms.clone()),
        Some(vec![Some(100.0), Some(200.0)])
    );
    // −0 stays −0 (the falling route's start).
    let Entity::Polyline(falling) = &again.entities[5] else {
        panic!("a polyline")
    };
    assert_eq!(
        falling.ms.as_ref().and_then(|m| m[0]).map(f64::to_bits),
        Some((-0.0f64).to_bits())
    );
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |e: Entity| kentos_kcad::encode(&drawing(vec![e])).expect_err("refused");
    // One measure per vertex.
    let e = refused(Entity::Polyline(path(Some(vec![Some(1.0)]), None)));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("polyline/ms"), "{e}");
    assert!(e.message.contains("1 ölçü var ama 3 köşe var"), "{e}");
    let e = refused(Entity::Polyline(path(
        None,
        Some(vec![part(Some(vec![None]))]),
    )));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("parts/0/ms"), "{e}");
    // An area has none, its parts neither.
    let e = refused(Entity::Polygon(path(Some(vec![None; 3]), None)));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("kapalı alanın ölçüsü"), "{e}");
    let e = refused(Entity::Polygon(path(
        None,
        Some(vec![AreaPart {
            pts: vec![v(5.0, 0.0), v(6.0, 0.0), v(6.0, 1.0)],
            ..part(Some(vec![None; 3]))
        }]),
    )));
    assert_eq!(e.code, Code::BadValue);
    // A non-finite measure is no number the file holds.
    let e = refused(line(Some(f64::NAN), None));
    assert_eq!(e.code, Code::NonFinite);
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/measures-in-schema-38.kcad",
            Code::UnknownField,
            "line/ma",
        ),
        (
            "broken/measures-in-polygon.kcad",
            Code::UnknownField,
            "polygon/ms",
        ),
        (
            "broken/measure-wrong-length.kcad",
            Code::BadValue,
            "polyline/ms",
        ),
        ("broken/line-measure-null.kcad", Code::WrongType, "line/m"),
        (
            "broken/part-measure-wrong-length.kcad",
            Code::BadValue,
            "parts/0/ms",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}

#[test]
fn the_typed_columns_carry_measures() {
    let doc = content("measures.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("measures.kcad"));
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(
        serde_json::to_string(&entities).expect("serializes"),
        serde_json::to_string(&doc.entities).expect("serializes")
    );
    // And the columns of a file read are the columns of the drawing that was written.
    let (_, again) =
        kentos_kcad::split(kentos_kcad::decode(&bytes).expect("reads")).expect("splits");
    let bits = |c: &kentos_kcad::Columns| c.floats.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&again), bits(&cols));
    assert_eq!(again.ints, cols.ints);
}
