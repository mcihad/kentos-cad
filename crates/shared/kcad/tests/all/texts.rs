//! A text's alignment, width factor and mask in the project file
//! (docs/adr/0145, docs/specs/kcad-v2.md §6.1, §6.6, §6.9): document schema 7
//! is written only for a drawing whose text or attribute definition has one,
//! every other drawing keeps its schema and its bytes; they come back as
//! written; the errors name their places, the writer refuses what the reader
//! would; the typed columns carry them.

use kentos_kcad::contracts::{
    AttributeDefinition, BlockDefinition, BlockId, DocumentSnapshotV2, Entity, EntityBase,
    EntityId, TextAlign, TextEntity, Vec2,
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

fn text(align: Option<TextAlign>, width_factor: Option<f64>, mask: bool) -> Entity {
    Entity::Text(TextEntity {
        base: base(0),
        p: Vec2 { x: 3.0, y: 4.0 },
        text: "Ada 1284".into(),
        height: 2.5,
        rotation: 30.0,
        align,
        width_factor,
        mask,
        label_of: None,
        label_scale: None,
        paragraph: Default::default(),
        face: Default::default(),
        path: None,
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

/// A block with one attribute definition, and nothing placing it.
fn with_attribute(align: Option<TextAlign>, width_factor: Option<f64>) -> DocumentSnapshotV2 {
    let mut doc = drawing(vec![]);
    doc.blocks = vec![BlockDefinition {
        id: BlockId([7; 16]),
        name: "Rögar".into(),
        base: Vec2 { x: 0.0, y: 0.0 },
        entities: vec![],
        attributes: vec![AttributeDefinition {
            tag: "NO".into(),
            prompt: None,
            value: Some("R-?".into()),
            p: Vec2 { x: 0.8, y: 0.0 },
            height: 0.5,
            rotation: 0.0,
            align,
            width_factor,
        }],
        description: None,
    }];
    doc
}

#[test]
fn only_a_drawing_with_text_extras_is_schema_7_and_the_others_keep_their_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    // The drawings of before text extras stay as they were (the fixtures did not change).
    for (file, want) in [
        ("minimal.kcad", 2),
        ("line-weights.kcad", 3),
        ("elevations.kcad", 4),
        ("parts.kcad", 5),
        ("blocks.kcad", 6),
        ("texts.kcad", 7),
    ] {
        let data = read(file);
        assert_eq!(schema(&data), want, "{file}");
        let doc = kentos_kcad::decode(&data).expect("reads");
        assert!(kentos_kcad::encode(&doc).expect("writes") == data, "{file}");
    }
    assert_eq!(of(&drawing(vec![text(None, None, false)])), 2);
    for extra in [
        text(Some(TextAlign::MiddleCenter), None, false),
        text(None, Some(0.8), false),
        text(None, None, true),
    ] {
        assert_eq!(of(&drawing(vec![extra])), 7);
    }
    // A block's attribute definition, or a text inside a definition, is enough.
    assert_eq!(of(&with_attribute(None, None)), 6);
    assert_eq!(of(&with_attribute(Some(TextAlign::TopLeft), None)), 7);
    assert_eq!(of(&with_attribute(None, Some(1.25))), 7);
    let mut inside = with_attribute(None, None);
    inside.blocks[0].entities.push(text(None, None, true));
    assert_eq!(of(&inside), 7);
}

#[test]
fn every_alignment_width_factor_and_mask_comes_back_as_written() {
    let mut more: Vec<Entity> = TextAlign::ALL
        .iter()
        .map(|a| text(Some(*a), None, false))
        .collect();
    more.push(text(None, Some(f64::MIN_POSITIVE), true));
    more.push(text(Some(TextAlign::BottomRight), Some(100.0), true));
    let doc = drawing(more);
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    assert_eq!(
        json(&kentos_kcad::decode(&bytes).expect("reads")),
        json(&doc)
    );
    // The fixture drawing: texts and an attribute definition with them, bit for bit.
    let want = content("texts.json");
    let got = kentos_kcad::decode(&read("texts.kcad")).expect("reads");
    assert_eq!(json(&got), json(&want));
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |file: &str| kentos_kcad::decode(&read(file)).expect_err("refused");
    for (file, code, place, words) in [
        (
            "broken/text-align-in-schema-6.kcad",
            Code::UnknownField,
            "document/entities/0/text/align",
            "",
        ),
        (
            "broken/attribute-align-in-schema-6.kcad",
            Code::UnknownField,
            "document/blocks/0/attributes/0/align",
            "",
        ),
        (
            "broken/text-align-baseline-left.kcad",
            Code::BadValue,
            "document/entities/0/text/align",
            "“baselineLeft” bilinmiyor",
        ),
        (
            "broken/text-align-unknown.kcad",
            Code::BadValue,
            "document/entities/0/text/align",
            "“center” bilinmiyor",
        ),
        (
            "broken/text-width-factor-zero.kcad",
            Code::BadValue,
            "document/entities/0/text/widthFactor",
            "genişlik çarpanı 0",
        ),
        (
            "broken/text-width-factor-too-wide.kcad",
            Code::BadValue,
            "document/entities/0/text/widthFactor",
            "en çok 100",
        ),
        (
            "broken/text-width-factor-nan.kcad",
            Code::NonFinite,
            "document/entities/0/text/widthFactor",
            "",
        ),
        (
            "broken/text-mask-false.kcad",
            Code::BadValue,
            "document/entities/0/text/mask",
            "zemin false yazılmaz",
        ),
        (
            "broken/attribute-width-factor-negative.kcad",
            Code::BadValue,
            "document/blocks/0/attributes/0/widthFactor",
            "genişlik çarpanı -1",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
        assert!(e.message.contains(words), "{file}: {e}");
    }
}

#[test]
fn the_writer_refuses_a_width_factor_the_reader_would() {
    for w in [0.0, -0.5, 100.5] {
        let e =
            kentos_kcad::encode(&drawing(vec![text(None, Some(w), false)])).expect_err("refused");
        assert_eq!(e.code, Code::BadValue, "{w}: {e}");
        assert!(
            e.message.contains("document/entities/1/text/widthFactor"),
            "{w}: {e}"
        );
        let e = kentos_kcad::encode(&with_attribute(None, Some(w))).expect_err("refused");
        assert_eq!(e.code, Code::BadValue, "{w}: {e}");
        assert!(
            e.message
                .contains("document/blocks/0/attributes/0/widthFactor"),
            "{w}: {e}"
        );
    }
    let e = kentos_kcad::encode(&drawing(vec![text(None, Some(f64::NAN), false)]))
        .expect_err("refused");
    assert_eq!(e.code, Code::NonFinite, "{e}");
}

#[test]
fn the_typed_columns_carry_them() {
    let doc = content("texts.json");
    let (head, cols) = kentos_kcad::split(doc).expect("splits");
    let (bytes, back) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("texts.kcad"));
    assert_eq!(back, head);
    // Read back into columns: the same columns, every float bit for bit (NaN aside, there is none here).
    let doc = kentos_kcad::decode(&bytes).expect("reads");
    let (_, again) = kentos_kcad::split(doc).expect("splits");
    assert_eq!(again, cols);
}
