//! Vertex elevations in the project file (docs/adr/0142, docs/specs/kcad-v2.md
//! §6.1, §6.6): document schema 4 is written only for a drawing that has one
//! and every other drawing keeps its schema and its bytes; every elevation
//! comes back bit for bit, a vertex without one stays without (not 0); the
//! errors name their places; the typed columns carry them, a vertex without
//! one as NaN.

use kentos_kcad::contracts::{
    DocumentSnapshotV2, Entity, EntityBase, EntityId, LineEntity, PathEntity, RingGeometry, Vec2,
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
        label_pins: Vec::new(),
    }
}

fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn triangle() -> Vec<Vec2> {
    vec![v(0.0, 0.0), v(4.0, 0.0), v(0.0, 3.0)]
}

fn line(id: u32, za: Option<f64>, zb: Option<f64>) -> Entity {
    Entity::Line(LineEntity {
        base: base(id),
        a: v(1.0, 2.0),
        b: v(3.0, 4.0),
        za,
        zb,
    })
}

fn path(id: u32, polygon: bool, zs: Option<Vec<Option<f64>>>) -> Entity {
    let p = PathEntity {
        base: base(id),
        pts: triangle(),
        bulges: None,
        holes: None,
        zs,
        parts: None,
    };
    if polygon {
        Entity::Polygon(p)
    } else {
        Entity::Polyline(p)
    }
}

/// The minimal fixture's drawing (one point, schema 2) with `more` after its point.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x22u8; 16];
        uid[15] = n as u8;
        doc.uids.push(EntityId(uid));
        e.base_mut().id = n;
        doc.entities.push(e);
    }
    doc
}

#[test]
fn every_valid_fixture_is_in_the_oldest_schema_that_holds_it_and_keeps_its_bytes() {
    // The drawings of before elevations stay schema 2 or 3, byte for byte (the fixtures were
    // written by the independent Python writer before this step and did not change).
    for (file, want) in [
        ("minimal.kcad", 2),
        ("drawing.kcad", 2),
        ("migrated.kcad", 2),
        ("exchange/web-edited.kcad", 2),
        ("exchange/desktop-edited.kcad", 2),
        ("line-weights.kcad", 3),
        ("elevations.kcad", 4),
    ] {
        let data = read(file);
        assert_eq!(schema(&data), want, "{file}");
        let doc = kentos_kcad::decode(&data).expect("reads");
        assert!(kentos_kcad::encode(&doc).expect("writes") == data, "{file}");
    }
}

#[test]
fn what_a_drawing_has_decides_its_schema() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    assert_eq!(of(&drawing(vec![])), 2);
    // Objects that may have elevations but have none stay as they were.
    assert_eq!(
        of(&drawing(vec![
            line(0, None, None),
            path(0, false, None),
            path(0, true, None)
        ])),
        2
    );

    // An object's own line weight: schema 3.
    let mut weighed = drawing(vec![]);
    weighed.entities[0].base_mut().line_weight = Some(0.35);
    assert_eq!(of(&weighed), 3);

    // Any vertex elevation: schema 4, one end of a line or one vertex of a path is enough.
    assert_eq!(of(&drawing(vec![line(0, Some(10.0), None)])), 4);
    assert_eq!(of(&drawing(vec![line(0, None, Some(-2.5))])), 4);
    assert_eq!(
        of(&drawing(vec![path(
            0,
            false,
            Some(vec![None, Some(1.0), None])
        )])),
        4
    );
    // A hole's elevations alone.
    let mut holed = path(0, true, None);
    if let Entity::Polygon(p) = &mut holed {
        p.holes = Some(vec![
            RingGeometry {
                pts: triangle(),
                bulges: None,
                zs: None,
            },
            RingGeometry {
                pts: triangle(),
                bulges: None,
                zs: Some(vec![Some(1.0), Some(2.0), Some(3.0)]),
            },
        ]);
    }
    assert_eq!(of(&drawing(vec![holed])), 4);
    // Schema 4 holds the line weights as schema 3 does.
    let mut both = drawing(vec![line(0, Some(10.0), Some(11.0))]);
    both.entities[0].base_mut().line_weight = Some(0.35);
    assert_eq!(of(&both), 4);
    let back = kentos_kcad::decode(&kentos_kcad::encode(&both).expect("writes")).expect("reads");
    assert_eq!(back.entities[0].base().line_weight, Some(0.35));
    // A hole without elevations is no elevation.
    let mut plain = path(0, true, None);
    if let Entity::Polygon(p) = &mut plain {
        p.holes = Some(vec![RingGeometry {
            pts: triangle(),
            bulges: None,
            zs: None,
        }]);
    }
    assert_eq!(of(&drawing(vec![plain])), 2);
}

#[test]
fn every_elevation_comes_back_bit_for_bit_and_a_vertex_without_one_stays_without() {
    let mut holed = path(0, true, Some(vec![Some(50.0), None, Some(f64::MAX)]));
    if let Entity::Polygon(p) = &mut holed {
        p.bulges = Some(vec![0.0, 0.25]);
        p.holes = Some(vec![
            RingGeometry {
                pts: triangle(),
                bulges: Some(vec![0.1]),
                zs: Some(vec![None, Some(-0.0), Some(5e-324)]),
            },
            RingGeometry {
                pts: triangle(),
                bulges: None,
                zs: None,
            },
        ]);
    }
    let doc = drawing(vec![
        line(0, Some(-0.0), Some(-25.125)),
        line(0, None, Some(1e-300)),
        path(0, false, Some(vec![Some(101.5), None, Some(-0.0)])),
        // No vertex has one, and the list is still a list: it stays.
        path(0, false, Some(vec![None, None, None])),
        holed,
    ]);
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(json(&back), json(&doc));
    assert!(kentos_kcad::encode(&back).expect("writes") == bytes);

    let Entity::Line(l) = &back.entities[1] else {
        panic!("a line")
    };
    assert_eq!(l.za.map(f64::to_bits), Some((-0.0f64).to_bits()));
    assert_eq!(l.zb, Some(-25.125));
    let Entity::Line(l) = &back.entities[2] else {
        panic!("a line")
    };
    assert_eq!((l.za, l.zb), (None, Some(1e-300)));
    let Entity::Polyline(p) = &back.entities[3] else {
        panic!("a polyline")
    };
    let zs = p.zs.as_ref().expect("elevations");
    assert_eq!(zs.len(), 3);
    assert_eq!(zs[0], Some(101.5));
    assert_eq!(zs[1], None, "a vertex without an elevation is not 0");
    assert_eq!(zs[2].map(f64::to_bits), Some((-0.0f64).to_bits()));
    let Entity::Polyline(p) = &back.entities[4] else {
        panic!("a polyline")
    };
    assert_eq!(p.zs, Some(vec![None, None, None]));
    let Entity::Polygon(p) = &back.entities[5] else {
        panic!("a polygon")
    };
    let holes = p.holes.as_ref().expect("holes");
    assert_eq!(
        holes[0]
            .zs
            .as_ref()
            .map(|z| z.iter().map(|z| z.map(f64::to_bits)).collect::<Vec<_>>()),
        Some(vec![
            None,
            Some((-0.0f64).to_bits()),
            Some(5e-324f64.to_bits())
        ])
    );
    assert_eq!(holes[1].zs, None);
}

#[test]
fn the_writer_refuses_what_a_reader_would_refuse_with_the_place() {
    let refuse = |doc: DocumentSnapshotV2, code: Code, place: &str| {
        let e = kentos_kcad::encode(&doc).expect_err("refused");
        assert_eq!(e.code, code, "{e}");
        assert!(e.message.contains(place), "{place}: {e}");
    };
    // One elevation per vertex.
    refuse(
        drawing(vec![path(0, false, Some(vec![Some(1.0), Some(2.0)]))]),
        Code::BadValue,
        "entities/1/polyline/zs",
    );
    refuse(
        drawing(vec![path(0, true, Some(vec![Some(1.0); 4]))]),
        Code::BadValue,
        "entities/1/polygon/zs",
    );
    let mut holed = path(0, true, None);
    if let Entity::Polygon(p) = &mut holed {
        p.holes = Some(vec![RingGeometry {
            pts: triangle(),
            bulges: None,
            zs: Some(vec![Some(1.0), None]),
        }]);
    }
    refuse(
        drawing(vec![holed]),
        Code::BadValue,
        "entities/1/polygon/holes/0/zs",
    );
    // A number the file cannot hold.
    refuse(
        drawing(vec![line(0, Some(f64::NAN), None)]),
        Code::NonFinite,
        "entities/1/line/za",
    );
    refuse(
        drawing(vec![line(0, None, Some(f64::NEG_INFINITY))]),
        Code::NonFinite,
        "entities/1/line/zb",
    );
    refuse(
        drawing(vec![path(
            0,
            false,
            Some(vec![Some(1.0), Some(f64::INFINITY), None]),
        )]),
        Code::NonFinite,
        "entities/1/polyline/zs/1",
    );
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |file: &str| kentos_kcad::decode(&read(file)).expect_err("refused");
    let e = refused("broken/elevation-wrong-length.kcad");
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("document/entities/0/polyline/zs"), "{e}");
    assert!(e.message.contains("2 kot var ama 3 köşe var"), "{e}");
    let e = refused("broken/hole-elevation-wrong-length.kcad");
    assert_eq!(e.code, Code::BadValue);
    assert!(
        e.message.contains("document/entities/0/polygon/holes/0/zs"),
        "{e}"
    );
    let e = refused("broken/elevation-nan.kcad");
    assert_eq!(e.code, Code::NonFinite);
    assert!(
        e.message.contains("document/entities/0/polyline/zs/1"),
        "{e}"
    );
    let e = refused("broken/elevation-infinity.kcad");
    assert_eq!(e.code, Code::NonFinite);
    assert!(e.message.contains("document/entities/0/line/zb"), "{e}");
    let e = refused("broken/elevation-int.kcad");
    assert_eq!(e.code, Code::WrongType);
    assert!(
        e.message.contains("document/entities/0/polyline/zs/1"),
        "{e}"
    );
    // An end without an elevation has no key: `null` there is a wrong type, not "none".
    let e = refused("broken/line-elevation-null.kcad");
    assert_eq!(e.code, Code::WrongType);
    assert!(e.message.contains("document/entities/0/line/za"), "{e}");
    // In an older schema the fields are unknown ones, whichever the kind.
    let e = refused("broken/elevation-in-schema-2.kcad");
    assert_eq!(e.code, Code::UnknownField);
    assert!(e.message.contains("document/entities/0/line/za"), "{e}");
    let e = refused("broken/elevation-in-schema-3.kcad");
    assert_eq!(e.code, Code::UnknownField);
    assert!(e.message.contains("document/entities/0/polyline/zs"), "{e}");
    // The newest schema this codec knows is 38 (multidimensional rasters, docs/adr/0243).
    let e = refused("broken/schema-version-39.kcad");
    assert_eq!(e.code, Code::SchemaVersion);
    assert!(
        e.message
            .contains("desteklenen: 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16"),
        "{e}"
    );
}

#[test]
fn the_typed_columns_carry_elevations_a_missing_one_as_nan() {
    let doc = content("elevations.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    // The fixture has two vertices without an elevation: one in a polyline, one in a hole.
    assert_eq!(cols.floats.iter().filter(|x| x.is_nan()).count(), 2);
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("elevations.kcad"));
    // And the columns of a file read are the columns of the drawing that was written.
    let (_, again) =
        kentos_kcad::split(kentos_kcad::decode(&bytes).expect("reads")).expect("splits");
    let bits = |c: &kentos_kcad::Columns| c.floats.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&again), bits(&cols));
    assert_eq!(again.ints, cols.ints);

    // Whichever NaN a page's engine wrote for "none", it reads as none and the check does not mind.
    let mut other = cols.clone();
    for x in other.floats.iter_mut().filter(|x| x.is_nan()) {
        *x = f64::from_bits(0xfff8_0000_0000_0001);
    }
    let (bytes, _) = kentos_kcad::encode_columns(&head, &other, &mut Quiet).expect("writes");
    assert!(bytes == read("elevations.kcad"));
}

#[test]
fn columns_with_an_elevation_list_of_another_length_reach_the_encoder_with_their_place() {
    let doc = content("elevations.json");
    let refuse = |change: &dyn Fn(&mut DocumentSnapshotV2), code: Code, place: &str| {
        let mut bad = doc.clone();
        change(&mut bad);
        let (head, cols) = kentos_kcad::split(bad).expect("splits");
        let e = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect_err("refused");
        assert_eq!(e.code, code, "{e}");
        assert!(e.message.contains(place), "{place}: {e}");
    };
    // The list is carried with its own length: the columns hold together, and the file's rule refuses it.
    refuse(
        &|d| {
            if let Entity::Polyline(p) = &mut d.entities[3] {
                p.zs.as_mut().expect("elevations").pop();
            }
        },
        Code::BadValue,
        "entities/3/polyline/zs",
    );
    refuse(
        &|d| {
            if let Entity::Polygon(p) = &mut d.entities[5] {
                let holes = p.holes.as_mut().expect("holes");
                holes[0].zs.as_mut().expect("elevations").push(Some(1.0));
            }
        },
        Code::BadValue,
        "entities/5/polygon/holes/0/zs",
    );
    // An infinity is no vertex without an elevation: the encoder's own refusal.
    refuse(
        &|d| {
            if let Entity::Polyline(p) = &mut d.entities[3] {
                p.zs.as_mut().expect("elevations")[0] = Some(f64::INFINITY);
            }
        },
        Code::NonFinite,
        "entities/3/polyline/zs/0",
    );
}

#[test]
fn elevation_columns_that_do_not_hold_together_are_refused() {
    let doc = content("elevations.json");
    let (_, cols) = kentos_kcad::split(doc).expect("splits");
    let refused = |cols: &kentos_kcad::Columns| {
        kentos_kcad::columns::unpack(cols)
            .map(|_| ())
            .expect_err("refused")
    };
    // Numbers gone: the lists run past the end.
    let mut short = cols.clone();
    short.floats.truncate(short.floats.len() - 3);
    assert_eq!(refused(&short).code, Code::BadColumns);
    // A list longer than the numbers left.
    let mut long = cols.clone();
    let at = long.ints.len() - 1;
    long.ints[at] = 1_000;
    assert_eq!(refused(&long).code, Code::BadColumns);
    // A flag no kind has: a line's third optional field.
    let mut flags = cols;
    // The first object is a line: its flags are the second int after the layer table (count, layer, flags).
    flags.ints[2] |= 1 << 10;
    assert_eq!(refused(&flags).code, Code::BadColumns);
}
