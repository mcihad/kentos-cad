//! Leaders in the project file (docs/adr/0146, docs/specs/kcad-v2.md §6.1,
//! §6.6): document schema 8 is written only for a drawing that has a leader,
//! in it or in a block definition, and every other drawing keeps its schema
//! and its bytes; a leader's fields come back bit for bit; the writer and
//! the reader refuse the same leaders, naming their places; the typed
//! columns carry them.

use kentos_kcad::contracts::{
    DocumentSnapshotV2, Entity, EntityBase, EntityId, LeaderArrow, LeaderEntity, Vec2,
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

fn leader(pts: Vec<Vec2>) -> LeaderEntity {
    LeaderEntity {
        base: EntityBase {
            id: 0,
            layer_id: "0".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        },
        pts,
        text: Some("Mevcut bina".into()),
        height: 2.5,
        rotation: 0.0,
        arrow: None,
        mask: false,
    }
}

/// The minimal drawing with `more` objects after its own.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x46u8; 16];
        uid[15] = n as u8;
        doc.uids.push(EntityId(uid));
        e.base_mut().id = n;
        doc.entities.push(e);
    }
    doc
}

#[test]
fn a_leader_and_only_a_leader_makes_schema_8() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    assert_eq!(of(&drawing(vec![])), 2);
    let one = leader(vec![v(500000.0, 4400000.0), v(500008.0, 4400006.0)]);
    assert_eq!(of(&drawing(vec![Entity::Leader(one.clone())])), 8);
    // In a block definition only (its insert alone would be schema 6).
    let doc = content("leaders.json");
    let mut in_block = doc.clone();
    let kept: Vec<usize> = (0..in_block.entities.len())
        .filter(|&i| !matches!(in_block.entities[i], Entity::Leader(_)))
        .collect();
    in_block.entities = kept.iter().map(|&i| doc.entities[i].clone()).collect();
    in_block.uids = kept.iter().map(|&i| doc.uids[i]).collect();
    assert_eq!(of(&in_block), 8);
    // The fixture written by the independent Python writer is schema 8, and the codec writes its bytes.
    let data = read("leaders.kcad");
    assert_eq!(schema(&data), 8);
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    // The older drawings keep their schemas and bytes.
    for (file, want) in [
        ("minimal.kcad", 2),
        ("blocks.kcad", 6),
        ("texts.kcad", 7),
    ] {
        let data = read(file);
        assert_eq!(schema(&data), want, "{file}");
        let doc = kentos_kcad::decode(&data).expect("reads");
        assert!(kentos_kcad::encode(&doc).expect("writes") == data, "{file}");
    }
}

#[test]
fn every_field_of_a_leader_comes_back() {
    let doc = content("leaders.json");
    let back = kentos_kcad::decode(&kentos_kcad::encode(&doc).expect("writes")).expect("reads");
    let leaders: Vec<&LeaderEntity> = back
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Leader(l) => Some(l),
            _ => None,
        })
        .collect();
    assert_eq!(
        leaders.iter().map(|l| l.arrow).collect::<Vec<_>>(),
        [
            None,
            Some(LeaderArrow::Open),
            Some(LeaderArrow::Dot),
            Some(LeaderArrow::None)
        ]
    );
    assert_eq!(
        leaders.iter().map(|l| l.text.as_deref()).collect::<Vec<_>>(),
        [
            Some("Mevcut bina"),
            Some("Ø150 PVC"),
            Some("Ada 101 Parsel 5"),
            None
        ]
    );
    assert_eq!(
        leaders.iter().map(|l| l.mask).collect::<Vec<_>>(),
        [false, true, false, false]
    );
    assert_eq!(leaders[2].rotation, 30.0);
    assert_eq!(leaders[1].pts.len(), 3);
    let Entity::Leader(inside) = &back.blocks[0].entities[1] else {
        panic!("{:?}", back.blocks[0].entities)
    };
    assert_eq!(inside.text.as_deref(), Some("V"));
}

#[test]
fn the_writer_refuses_what_a_reader_would_refuse_with_the_place() {
    let refuse = |e: LeaderEntity, code: Code, place: &str| {
        let err = kentos_kcad::encode(&drawing(vec![Entity::Leader(e)])).expect_err("refused");
        assert_eq!(err.code, code, "{err}");
        assert!(err.message.contains(place), "{place}: {err}");
    };
    let two = vec![v(500000.0, 4400000.0), v(500008.0, 4400006.0)];
    refuse(
        leader(vec![v(500000.0, 4400000.0)]),
        Code::BadValue,
        "entities/1/leader/pts",
    );
    refuse(
        LeaderEntity {
            height: 0.0,
            ..leader(two.clone())
        },
        Code::BadValue,
        "entities/1/leader/height",
    );
    refuse(
        LeaderEntity {
            height: f64::NAN,
            ..leader(two.clone())
        },
        Code::NonFinite,
        "entities/1/leader/height",
    );
    refuse(
        LeaderEntity {
            text: Some(String::new()),
            ..leader(two)
        },
        Code::BadValue,
        "entities/1/leader/text",
    );
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |file: &str| kentos_kcad::decode(&read(file)).expect_err("refused");
    let e = refused("broken/leader-in-schema-7.kcad");
    assert_eq!(e.code, Code::UnknownKind);
    assert!(e.message.contains("“leader”"), "{e}");
    for (file, place) in [
        ("broken/leader-one-vertex.kcad", "document/entities/0/leader/pts"),
        ("broken/leader-zero-height.kcad", "document/entities/0/leader/height"),
        ("broken/leader-empty-note.kcad", "document/entities/0/leader/text"),
        ("broken/leader-filled-arrow.kcad", "document/entities/0/leader/arrow"),
        ("broken/leader-mask-false.kcad", "document/entities/0/leader/mask"),
    ] {
        let e = refused(file);
        assert_eq!(e.code, Code::BadValue, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}

#[test]
fn the_typed_columns_carry_leaders() {
    let doc = content("leaders.json");
    let (head, cols) = kentos_kcad::split(doc).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("leaders.kcad"));
}
