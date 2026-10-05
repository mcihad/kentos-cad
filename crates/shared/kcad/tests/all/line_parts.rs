//! Multi-part polylines and multi-point objects in the project file
//! (docs/adr/0174, docs/specs/kcad-v2.md §6.1, §6.6): document schema 17 is
//! written only for a drawing with a polyline or a point that has parts,
//! every other drawing keeps its schema and its bytes; the parts come back in
//! order, bit for bit, with their arcs and elevations; the writer refuses
//! what the reader would; the typed columns carry them.

use kentos_kcad::contracts::{
    AreaPart, DocumentSnapshotV2, Entity, EntityBase, EntityId, PathEntity, PointEntity, PointPart,
    RingGeometry, Vec2,
};
use kentos_kcad::{Code, Quiet};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says: the byte after the `version` key.
fn schema(file: &[u8]) -> u8 {
    let payload = &file[36..file.len() - 32];
    let key = b"\x67version";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("a version key")
        + key.len();
    payload[at]
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
    }
}

fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn line_part(x: f64, n: usize) -> AreaPart {
    AreaPart {
        pts: (0..n).map(|i| v(x + i as f64, 0.0)).collect(),
        bulges: None,
        holes: None,
        zs: None,
    }
}

fn polyline(parts: Option<Vec<AreaPart>>) -> Entity {
    Entity::Polyline(PathEntity {
        base: base(0),
        pts: vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0)],
        bulges: None,
        holes: None,
        zs: None,
        parts,
    })
}

fn point(parts: Option<Vec<PointPart>>) -> Entity {
    Entity::Point(PointEntity {
        base: base(0),
        p: v(0.0, 0.0),
        z: Some(100.0),
        parts,
    })
}

/// The minimal fixture's drawing (one point, schema 2) with `more` after its point.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x44u8; 16];
        uid[15] = n as u8;
        doc.uids.push(EntityId(uid));
        e.base_mut().id = n;
        doc.entities.push(e);
    }
    doc
}

#[test]
fn only_a_drawing_with_line_or_point_parts_is_schema_17() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("multi-part-lines.kcad");
    assert_eq!(schema(&data), 17);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&drawing(vec![polyline(None), point(None)])), 2);
    assert_eq!(
        of(&drawing(vec![polyline(Some(vec![line_part(5.0, 2)]))])),
        17
    );
    let parts = vec![PointPart {
        p: v(3.0, 4.0),
        z: None,
    }];
    assert_eq!(of(&drawing(vec![point(Some(parts))])), 17);
    // An empty list is a field too.
    assert_eq!(of(&drawing(vec![polyline(Some(Vec::new()))])), 17);
    // A block definition's polyline with parts is the drawing's schema too.
    let mut doc = content("blocks.json");
    if let Some(Entity::Polyline(p)) = doc.blocks[0]
        .entities
        .iter_mut()
        .find(|e| matches!(e, Entity::Polyline(_)))
    {
        p.parts = Some(vec![line_part(9.0, 3)]);
    } else {
        doc.blocks[0].entities.push({
            let mut e = polyline(Some(vec![line_part(9.0, 3)]));
            e.base_mut().id = 99;
            e
        });
    }
    assert_eq!(of(&doc), 17);
}

#[test]
fn the_parts_come_back_in_order_bit_for_bit() {
    let doc = content("multi-part-lines.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let Entity::Polyline(road) = &again.entities[0] else {
        panic!("a polyline")
    };
    let parts = road.parts.as_ref().expect("parts");
    assert_eq!(parts.len(), 2);
    assert_eq!(
        parts[1].zs.as_ref().expect("zs")[2].map(f64::to_bits),
        Some((-0.0f64).to_bits())
    );
    let Entity::Point(points) = &again.entities[1] else {
        panic!("a point")
    };
    assert_eq!(points.parts.as_ref().map(Vec::len), Some(2));
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |e: Entity| kentos_kcad::encode(&drawing(vec![e])).expect_err("refused");
    // A polyline's part has two vertices or more.
    let e = refused(polyline(Some(vec![line_part(5.0, 2), line_part(9.0, 1)])));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("polyline/parts/1"), "{e}");
    assert!(e.message.contains("1 köşesi var; en az iki olmalı"), "{e}");
    // And no holes.
    let mut holed = line_part(5.0, 3);
    holed.holes = Some(vec![RingGeometry {
        pts: vec![v(5.0, 0.0), v(6.0, 0.0), v(6.0, 1.0)],
        bulges: None,
        zs: None,
    }]);
    let e = refused(polyline(Some(vec![holed])));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("çoklu çizginin parçasının adası"), "{e}");
    // A part's elevations: one per vertex.
    let mut short = line_part(5.0, 3);
    short.zs = Some(vec![Some(1.0)]);
    let e = refused(polyline(Some(vec![short])));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("parts/0/zs"), "{e}");
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/polyline-parts-in-schema-16.kcad",
            Code::UnknownField,
            "polyline/parts",
        ),
        (
            "broken/point-parts-in-schema-16.kcad",
            Code::UnknownField,
            "point/parts",
        ),
        (
            "broken/polyline-part-one-point.kcad",
            Code::BadValue,
            "polyline/parts/0",
        ),
        (
            "broken/polyline-part-holes.kcad",
            Code::UnknownField,
            "polyline/parts/0/holes",
        ),
        (
            "broken/point-part-without-place.kcad",
            Code::MissingField,
            "point/parts/0",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}

#[test]
fn the_typed_columns_carry_line_and_point_parts() {
    let doc = content("multi-part-lines.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("multi-part-lines.kcad"));
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(
        serde_json::to_string(&entities).expect("serializes"),
        serde_json::to_string(&doc.entities).expect("serializes")
    );
}
