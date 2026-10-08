//! Multi-part areas in the project file (docs/adr/0143, docs/specs/kcad-v2.md
//! §6.1, §6.6): document schema 5 is written only for a drawing with an area
//! that has parts, every other drawing keeps its schema and its bytes; the
//! parts come back in order, bit for bit, with their arcs, holes and
//! elevations; the errors name their places; the typed columns carry them.

use kentos_kcad::contracts::{
    AreaPart, DocumentSnapshotV2, Entity, EntityBase, EntityId, PathEntity, RingGeometry, Vec2,
};
use kentos_kcad::{Code, Quiet};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// A drawing as JSON text: float64 in its shortest round-trip form, so -0.0 and 0.0 differ.
fn json(doc: &DocumentSnapshotV2) -> String {
    serde_json::to_string(doc).expect("serializes")
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

fn square(x: f64) -> Vec<Vec2> {
    vec![v(x, 0.0), v(x + 1.0, 0.0), v(x + 1.0, 1.0), v(x, 1.0)]
}

fn part(x: f64) -> AreaPart {
    AreaPart {
        pts: square(x),
        bulges: None,
        holes: None,
        zs: None,
    }
}

fn area(parts: Option<Vec<AreaPart>>) -> Entity {
    Entity::Polygon(PathEntity {
        base: base(0),
        pts: square(0.0),
        bulges: None,
        holes: None,
        zs: None,
        parts,
    })
}

/// The minimal fixture's drawing (one point, schema 2) with `more` after its point.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x33u8; 16];
        uid[15] = n as u8;
        doc.uids.push(EntityId(uid));
        e.base_mut().id = n;
        doc.entities.push(e);
    }
    doc
}

#[test]
fn only_a_drawing_with_parts_is_schema_5_and_the_others_keep_their_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    // The drawings of before parts stay as they were (the fixtures did not change).
    for (file, want) in [
        ("minimal.kcad", 2),
        ("drawing.kcad", 2),
        ("line-weights.kcad", 3),
        ("elevations.kcad", 4),
        ("parts.kcad", 5),
    ] {
        let data = read(file);
        assert_eq!(schema(&data), want, "{file}");
        let doc = kentos_kcad::decode(&data).expect("reads");
        assert!(kentos_kcad::encode(&doc).expect("writes") == data, "{file}");
    }
    assert_eq!(of(&drawing(vec![area(None)])), 2);
    assert_eq!(of(&drawing(vec![area(Some(vec![part(5.0)]))])), 5);
    // An empty list is a field too: written as given, and it comes back.
    let empty = drawing(vec![area(Some(vec![]))]);
    let bytes = kentos_kcad::encode_verified(&empty).expect("writes");
    assert_eq!(schema(&bytes), 5);
    assert_eq!(
        json(&kentos_kcad::decode(&bytes).expect("reads")),
        json(&empty)
    );
    // Schema 5 holds what 3 and 4 hold: a line weight and elevations before the area.
    let mut all = drawing(vec![area(Some(vec![part(5.0)]))]);
    all.entities[0].base_mut().line_weight = Some(0.35);
    if let Entity::Polygon(p) = &mut all.entities[1] {
        p.zs = Some(vec![Some(1.0), None, Some(2.0), Some(3.0)]);
    }
    let bytes = kentos_kcad::encode_verified(&all).expect("writes");
    assert_eq!(schema(&bytes), 5);
    assert_eq!(
        json(&kentos_kcad::decode(&bytes).expect("reads")),
        json(&all)
    );
}

#[test]
fn parts_come_back_in_order_with_their_arcs_holes_and_elevations() {
    let doc = kentos_kcad::decode(&read("parts.kcad")).expect("reads");
    assert_eq!(json(&doc), json(&content("parts.json")));
    let Entity::Polygon(p) = &doc.entities[0] else {
        panic!("{:?}", doc.entities[0])
    };
    let parts = p.parts.as_ref().expect("parts");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].bulges.as_deref(), Some(&[0.0, 0.5, 0.0, 0.0][..]));
    let zs = parts[0].zs.as_ref().expect("elevations");
    assert_eq!(zs[1], None, "a vertex without an elevation stays without");
    assert!(
        zs[3].is_some_and(|z| z == 0.0 && z.is_sign_negative()),
        "-0 stays -0"
    );
    let hole = &parts[1].holes.as_ref().expect("a hole")[0];
    assert_eq!(hole.bulges.as_deref(), Some(&[0.25][..]));
    assert_eq!(hole.zs, Some(vec![Some(99.0), Some(99.5), None]));
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |file: &str| kentos_kcad::decode(&read(file)).expect_err("refused");
    // Parts are schema 5's: in schema 4 an unknown field, and never a polyline's.
    let e = refused("broken/parts-in-schema-4.kcad");
    assert_eq!(e.code, Code::UnknownField);
    assert!(
        e.message.contains("document/entities/0/polygon/parts"),
        "{e}"
    );
    let e = refused("broken/parts-on-polyline.kcad");
    assert_eq!(e.code, Code::UnknownField);
    assert!(
        e.message.contains("document/entities/0/polyline/parts"),
        "{e}"
    );
    // A part's elevations are one per vertex, as the area's own.
    let e = refused("broken/part-elevation-wrong-length.kcad");
    assert_eq!(e.code, Code::BadValue);
    assert!(
        e.message.contains("document/entities/0/polygon/parts/0/zs"),
        "{e}"
    );
    assert!(e.message.contains("2 kot var ama 4 köşe var"), "{e}");
    let e = refused("broken/part-without-points.kcad");
    assert_eq!(e.code, Code::MissingField);
    assert!(
        e.message.contains("document/entities/0/polygon/parts/0"),
        "{e}"
    );
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |e: Entity| kentos_kcad::encode(&drawing(vec![e])).expect_err("refused");
    // A polyline's part has no holes (docs/adr/0174).
    let mut holed_line = part(5.0);
    holed_line.holes = Some(vec![RingGeometry {
        pts: square(5.25),
        bulges: None,
        zs: None,
    }]);
    let Entity::Polygon(mut path) = area(Some(vec![holed_line])) else {
        unreachable!()
    };
    path.base = base(0);
    let e = refused(Entity::Polyline(path));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("polyline/parts/0"), "{e}");
    assert!(e.message.contains("çoklu çizginin parçasının adası"), "{e}");
    // A part's elevations: one per vertex.
    let mut short = part(5.0);
    short.zs = Some(vec![Some(1.0)]);
    let e = refused(area(Some(vec![short])));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("parts/0/zs"), "{e}");
    // A part's hole too.
    let mut holed = part(5.0);
    holed.holes = Some(vec![RingGeometry {
        pts: square(5.25),
        bulges: None,
        zs: Some(vec![None]),
    }]);
    let e = refused(area(Some(vec![holed])));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("parts/0/holes/0/zs"), "{e}");
}

#[test]
fn the_typed_columns_carry_parts() {
    let doc = content("parts.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("parts.kcad"));
    // And the columns of a file read are the columns of the drawing that was written.
    let (_, again) =
        kentos_kcad::split(kentos_kcad::decode(&bytes).expect("reads")).expect("splits");
    let bits = |c: &kentos_kcad::Columns| c.floats.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&again), bits(&cols));
    assert_eq!(again.ints, cols.ints);
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(
        serde_json::to_string(&entities).expect("serializes"),
        serde_json::to_string(&doc.entities).expect("serializes")
    );
    // The first object's ints: the layer table's size, then its layer, flags and attribute count;
    // its 4 vertices, 1 hole (flags 0, 4 vertices); 2 parts, the first with bulges and
    // elevations (flags 3).
    assert_eq!(cols.ints[4..10], [4, 1, 0, 4, 2, 3]);
    // A part's flags beyond bulges, elevations and holes are no columns this layout knows.
    let mut broken = cols.clone();
    broken.ints[9] = 8;
    let e = kentos_kcad::columns::unpack(&broken).expect_err("refused");
    assert!(e.message.contains("parçanın bayrakları 0x8"), "{e}");
}
