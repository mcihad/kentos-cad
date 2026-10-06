//! Hatch patterns, gradients and ties in the project file (docs/adr/0186,
//! docs/specs/kcad-v2.md §6.1, §6.6): document schema 23 is written only for
//! a drawing with a hatch of families or a gradient (or a field of theirs)
//! or a hatch that follows its objects, in the drawing or a block
//! definition; every other drawing keeps its schema and its bytes. The
//! patterns come back bit for bit; the writer refuses what the reader would;
//! the reader's errors name their places; the typed columns carry them.

use kentos_kcad::contracts::{
    DocumentSnapshotV2, Entity, EntityBase, EntityId, GradientShape, HatchAssoc, HatchEntity,
    HatchGradient, HatchPattern, HatchPatternType, PatternLine, Vec2,
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

fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn hatch(pattern: HatchPattern, assoc: Option<HatchAssoc>) -> Entity {
    Entity::Hatch(HatchEntity {
        base: EntityBase {
            id: 0,
            layer_id: "0".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        },
        ring: vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0)],
        holes: None,
        pattern,
        assoc,
    })
}

fn ansi31() -> HatchPattern {
    HatchPattern {
        name: Some("ANSI31".into()),
        scale: Some(0.5),
        lines: Some(vec![PatternLine {
            angle: 45.0,
            origin: [0.0, 0.0],
            offset: [0.0, 3.175],
            dashes: Vec::new(),
        }]),
        ..HatchPattern::user(HatchPatternType::Pattern, 0.0, 1.0)
    }
}

fn gradient() -> HatchPattern {
    HatchPattern {
        gradient: Some(HatchGradient {
            shape: GradientShape::Spherical,
            inverted: false,
            color2: "#FFFFFF".into(),
        }),
        ..HatchPattern::user(HatchPatternType::Gradient, 0.0, 1.0)
    }
}

/// The minimal fixture's drawing (one point, schema 2) with `more` after its point.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x55u8; 16];
        uid[15] = n as u8;
        doc.uids.push(EntityId(uid));
        e.base_mut().id = n;
        doc.entities.push(e);
    }
    doc
}

#[test]
fn only_a_drawing_with_a_pattern_a_gradient_or_a_tie_is_schema_23() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("hatches.kcad");
    assert_eq!(schema(&data), 23);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    let lines = HatchPattern::user(HatchPatternType::Lines, 45.0, 2.0);
    assert_eq!(of(&drawing(vec![hatch(lines.clone(), None)])), 2);
    assert_eq!(of(&drawing(vec![hatch(ansi31(), None)])), 23);
    assert_eq!(of(&drawing(vec![hatch(gradient(), None)])), 23);
    // Lines that follow their objects: the tie is schema 23's.
    let mut doc = drawing(vec![hatch(lines, None)]);
    let tie = HatchAssoc {
        outer: doc.uids[0],
        islands: Vec::new(),
        cutouts: Vec::new(),
        seed: v(1.0, 1.0),
    };
    if let Entity::Hatch(h) = &mut doc.entities[1] {
        h.assoc = Some(tie);
    }
    assert_eq!(of(&doc), 23);
    // A block definition's pattern is the drawing's schema too.
    let mut doc = content("blocks.json");
    let mut e = hatch(ansi31(), None);
    e.base_mut().id = doc.blocks[0].entities.len() as u32 + 1;
    doc.blocks[0].entities.push(e);
    assert_eq!(of(&doc), 23);
}

#[test]
fn patterns_and_ties_come_back_bit_for_bit() {
    let doc = content("hatches.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let Entity::Hatch(tied) = &again.entities[3] else {
        panic!("a hatch")
    };
    let a = tied.assoc.as_ref().expect("its tie");
    assert_eq!(
        (a.outer, a.islands.as_slice(), a.cutouts.as_slice()),
        (doc.uids[0], &doc.uids[1..2], &doc.uids[2..3])
    );
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |e: Entity| kentos_kcad::encode(&drawing(vec![e])).expect_err("refused");
    // A pattern has families, each its lines apart.
    let mut bare = ansi31();
    bare.lines = Some(Vec::new());
    let e = refused(hatch(bare, None));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("hatch/pattern"), "{e}");
    assert!(e.message.contains("0 çizgi ailesi var"), "{e}");
    // A gradient's second colour is #RRGGBB.
    let mut named = gradient();
    named.gradient.as_mut().expect("gradient").color2 = "beyaz".into();
    let e = refused(hatch(named, None));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("#RRGGBB"), "{e}");
    // A tie names each object once.
    let id = EntityId([0x55; 16]);
    let twice = HatchAssoc {
        outer: id,
        islands: vec![id],
        cutouts: Vec::new(),
        seed: v(1.0, 1.0),
    };
    let e = refused(hatch(ansi31(), Some(twice)));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("hatch/assoc"), "{e}");
    // A block definition's objects have no persistent ids for a tie to name.
    let mut doc = content("blocks.json");
    let tie = HatchAssoc {
        outer: id,
        islands: Vec::new(),
        cutouts: Vec::new(),
        seed: v(1.0, 1.0),
    };
    let mut e = hatch(ansi31(), Some(tie));
    e.base_mut().id = doc.blocks[0].entities.len() as u32 + 1;
    doc.blocks[0].entities.push(e);
    let e = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(e.code, Code::BadValue);
    assert!(
        e.message
            .contains("blok tanımının taraması nesnelere bağlı olamaz"),
        "{e}"
    );
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/hatch-name-in-schema-22.kcad",
            Code::UnknownField,
            "hatch/pattern/name",
        ),
        (
            "broken/hatch-assoc-in-block.kcad",
            Code::UnknownField,
            "hatch/assoc",
        ),
        (
            "broken/hatch-pattern-dashes-empty.kcad",
            Code::BadValue,
            "hatch/pattern/lines/0/dashes",
        ),
        (
            "broken/hatch-gradient-inverted-false.kcad",
            Code::BadValue,
            "hatch/pattern/gradient/inverted",
        ),
        (
            "broken/hatch-assoc-islands-empty.kcad",
            Code::BadValue,
            "hatch/assoc/islands",
        ),
        (
            "broken/hatch-pattern-lines-apart-zero.kcad",
            Code::BadValue,
            "hatch/pattern",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}

#[test]
fn the_typed_columns_carry_patterns_gradients_and_ties() {
    let doc = content("hatches.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("hatches.kcad"));
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(
        serde_json::to_string(&entities).expect("serializes"),
        serde_json::to_string(&doc.entities).expect("serializes")
    );
}
