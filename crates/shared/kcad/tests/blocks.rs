//! Blocks in the project file (docs/adr/0144, docs/specs/kcad-v2.md §6.1,
//! §6.9): document schema 6 is written only for a drawing with block
//! definitions, every other drawing keeps its schema and its bytes; the
//! definitions and their objects come back bit for bit, without persistent
//! ids; the reader's and the writer's errors name their places; the typed
//! columns carry inserts while the definitions travel in the drawing's JSON.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};
use kentos_kcad::{Code, Quiet};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

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

#[test]
fn schema_six_only_with_definitions() {
    let doc = content("blocks.json");
    assert_eq!(schema(&kentos_kcad::encode(&doc).expect("writes")), 6);
    // Without its blocks and inserts the same drawing is schema 2 again.
    let mut plain = doc.clone();
    plain.blocks.clear();
    let keep: Vec<bool> = plain
        .entities
        .iter()
        .map(|e| !matches!(e, Entity::Insert(_)))
        .collect();
    let mut kept = keep.iter();
    plain.entities.retain(|_| *kept.next().unwrap_or(&true));
    let mut kept = keep.iter();
    plain.uids.retain(|_| *kept.next().unwrap_or(&true));
    assert_eq!(schema(&kentos_kcad::encode(&plain).expect("writes")), 2);
    // Every other valid fixture keeps its schema and bytes (fixtures.rs); schema 6 reads its objects' slots
    // per definition: 1, 2, 3 … in the definition's order.
    let back = kentos_kcad::decode(&read("blocks.kcad")).expect("reads");
    for block in &back.blocks {
        let slots: Vec<u32> = block.entities.iter().map(|e| e.base().id).collect();
        assert_eq!(slots, (1..=block.entities.len() as u32).collect::<Vec<_>>());
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |file: &str| kentos_kcad::decode(&read(file)).expect_err("refused");
    let place = |file: &str, code: Code, path: &str| {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(path), "{file}: {e}");
    };
    place(
        "broken/blocks-in-schema-5.kcad",
        Code::UnknownField,
        "document/blocks",
    );
    place(
        "broken/insert-in-schema-5.kcad",
        Code::UnknownKind,
        "document/entities/0/insert",
    );
    place(
        "broken/block-object-uid.kcad",
        Code::UnknownField,
        "document/blocks/0/entities/0/circle/uid",
    );
    place(
        "broken/unknown-block.kcad",
        Code::UnknownBlock,
        "document/entities/0/insert/block",
    );
    place(
        "broken/unknown-block-inside.kcad",
        Code::UnknownBlock,
        "document/blocks/0/entities/0/insert/block",
    );
    place(
        "broken/duplicate-block-name.kcad",
        Code::DuplicateBlock,
        "document/blocks/1/name",
    );
    place(
        "broken/duplicate-block-id.kcad",
        Code::DuplicateBlock,
        "document/blocks/1/id",
    );
    place(
        "broken/block-cycle.kcad",
        Code::BlockCycle,
        "document/blocks/0",
    );
    place(
        "broken/block-too-deep.kcad",
        Code::BlockTooDeep,
        "document/blocks/0",
    );
    place(
        "broken/insert-scale-zero.kcad",
        Code::BadValue,
        "document/entities/0/insert/scale",
    );
    place(
        "broken/insert-mirror-false.kcad",
        Code::BadValue,
        "document/entities/0/insert/mirror",
    );
    place(
        "broken/duplicate-attribute-tag.kcad",
        Code::BadValue,
        "document/blocks/0/attributes/1/tag",
    );
    let e = refused("broken/duplicate-block-name.kcad");
    assert!(e.message.contains("“DİREK” adı 1. tanımda da var"), "{e}");
}

#[test]
fn the_writers_errors_name_their_places() {
    let doc = content("blocks.json");
    let refuse = |change: &dyn Fn(&mut DocumentSnapshotV2), code: Code, path: &str| {
        let mut d = doc.clone();
        change(&mut d);
        let e = kentos_kcad::encode(&d).expect_err("refused");
        assert_eq!(e.code, code, "{e}");
        assert!(e.message.contains(path), "{e}");
    };
    refuse(
        &|d| d.blocks[1].name = "AYDINLATMA DİREĞİ".into(),
        Code::DuplicateBlock,
        "document/blocks/1/name",
    );
    refuse(
        &|d| d.blocks[0].name = "RÖGAR".into(),
        Code::DuplicateBlock,
        "document/blocks/1/name",
    );
    refuse(
        &|d| {
            if let Entity::Insert(i) = &mut d.entities[2] {
                i.scale = -1.0;
            }
        },
        Code::BadValue,
        "document/entities/2/insert/scale",
    );
    refuse(
        &|d| {
            // Rögar holds Aydınlatma direği, which holds Rögar.
            let back = d.blocks[1].entities[0].clone();
            let mut insert = d.blocks[0].entities[1].clone();
            if let Entity::Insert(i) = &mut insert {
                i.block = d.blocks[0].id;
            }
            d.blocks[1].entities = vec![back, insert];
        },
        Code::BlockCycle,
        "document/blocks/0",
    );
    refuse(
        &|d| d.blocks.clear(),
        Code::UnknownBlock,
        "document/entities/0/insert/block",
    );
}

#[test]
fn inserts_cross_the_browsers_typed_boundary() {
    let doc = content("blocks.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    // Insert is kind 13; the definitions are in the head, not in the columns.
    assert_eq!(cols.kinds, [13, 13, 13, 1]);
    assert!(head.contains("\"blocks\":[{"), "{head}");
    let (bytes, head_back) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("blocks.kcad"));
    assert_eq!(head_back, head);
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(json(&back), json(&doc));
    // The mirror flag and the −0 rotation survive.
    let (_, cols) = kentos_kcad::split(back).expect("splits");
    let (entities, _) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    match (&entities[1], &entities[2]) {
        (Entity::Insert(mirrored), Entity::Insert(turned)) => {
            assert!(mirrored.mirror);
            assert_eq!(turned.rotation.to_bits(), (-0.0f64).to_bits());
        }
        other => panic!("{other:?}"),
    }
}
