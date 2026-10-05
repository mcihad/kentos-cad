//! Texts that write an object's label in the project file (docs/adr/0175 §4,
//! docs/specs/kcad-v2.md §6.1, §6.6): document schema 18 is written only for
//! a drawing with a text that has `labelOf` and `labelScale`, every other
//! drawing keeps its schema and its bytes; the link comes back bit for bit;
//! the writer refuses what the reader would (one field without the other, a
//! scale of 0, a nil id, a block definition's text); the typed columns carry
//! it.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity, EntityBase, EntityId, TextEntity, Vec2};
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

fn text(label_of: Option<EntityId>, label_scale: Option<f64>) -> Entity {
    Entity::Text(TextEntity {
        base: EntityBase {
            id: 0,
            layer_id: "0".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        },
        p: Vec2 { x: 1.0, y: 2.0 },
        text: "101".into(),
        height: 2.5,
        rotation: 0.0,
        align: None,
        width_factor: None,
        mask: false,
        label_of,
        label_scale,
    })
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

/// The minimal drawing's point's persistent id: what a linked text names.
fn first(doc: &DocumentSnapshotV2) -> EntityId {
    doc.uids[0]
}

#[test]
fn only_a_drawing_with_a_linked_text_is_schema_18() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("linked-texts.kcad");
    assert_eq!(schema(&data), 18);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&drawing(vec![text(None, None)])), 2);
    let base = drawing(Vec::new());
    let linked = drawing(vec![text(Some(first(&base)), Some(1000.0))]);
    assert_eq!(of(&linked), 18);
}

#[test]
fn the_link_comes_back_bit_for_bit() {
    let doc = content("linked-texts.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let Entity::Text(parcel) = &again.entities[2] else {
        panic!("a text")
    };
    assert_eq!(parcel.label_of, Some(again.uids[0]));
    assert_eq!(parcel.label_scale, Some(1000.0));
    let Entity::Text(own) = &again.entities[4] else {
        panic!("a text")
    };
    assert_eq!((own.label_of, own.label_scale), (None, None));
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let base = drawing(Vec::new());
    let id = first(&base);
    let refused = |e: Entity| kentos_kcad::encode(&drawing(vec![e])).expect_err("refused");
    for (e, place) in [
        (text(Some(id), None), "text/labelScale"),
        (text(None, Some(1000.0)), "text/labelOf"),
        (text(Some(id), Some(0.0)), "text/labelScale"),
        (text(Some(id), Some(-500.0)), "text/labelScale"),
        (text(Some(EntityId([0; 16])), Some(1000.0)), "text/labelOf"),
    ] {
        let e = refused(e);
        assert_eq!(e.code, Code::BadValue, "{e}");
        assert!(e.message.contains(place), "{e}");
    }
    // Not finite: the float's own refusal.
    let e = refused(text(Some(id), Some(f64::NAN)));
    assert_eq!(e.code, Code::NonFinite, "{e}");
    // A block definition's text names no object: its objects have no persistent ids.
    let mut doc = content("blocks.json");
    let mut inside = text(Some(id), Some(1000.0));
    inside.base_mut().id = 99;
    doc.blocks[0].entities.push(inside);
    let e = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(e.code, Code::BadValue, "{e}");
    assert!(e.message.contains("blok tanımının yazısı"), "{e}");
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/text-label-of-in-schema-17.kcad",
            Code::UnknownField,
            "text/labelOf",
        ),
        (
            "broken/text-label-of-without-scale.kcad",
            Code::BadValue,
            "birlikte",
        ),
        (
            "broken/text-label-scale-without-of.kcad",
            Code::BadValue,
            "birlikte",
        ),
        (
            "broken/text-label-scale-zero.kcad",
            Code::BadValue,
            "text/labelScale",
        ),
        (
            "broken/text-label-scale-nan.kcad",
            Code::NonFinite,
            "text/labelScale",
        ),
        (
            "broken/text-label-of-short.kcad",
            Code::BadValue,
            "text/labelOf",
        ),
        (
            "broken/text-label-of-nil.kcad",
            Code::BadValue,
            "text/labelOf",
        ),
        (
            "broken/block-text-label-of.kcad",
            Code::UnknownField,
            "text/labelOf",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}

#[test]
fn the_typed_columns_carry_the_link() {
    let doc = content("linked-texts.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("linked-texts.kcad"));
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(
        serde_json::to_string(&entities).expect("serializes"),
        serde_json::to_string(&doc.entities).expect("serializes")
    );
}
