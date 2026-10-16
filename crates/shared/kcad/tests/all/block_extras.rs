//! An insert's own y factor and a block's unit in the project file (docs/adr/0315 §3, §4): document schema 57 is
//! written only for a drawing with an insert whose y factor is not its x one, of the drawing or of a definition, or
//! a block with a unit; they come back as written, through the codec and the typed columns; the reader refuses a y
//! factor equal to the x one, one not positive, an unknown unit, and either in schema 56.

use kentos_kcad::contracts::{BlockUnits, DocumentSnapshotV2, Entity};
use kentos_kcad::{Code, Quiet};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says.
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

fn y_scale(e: &Entity) -> Option<f64> {
    match e {
        Entity::Insert(i) => i.y_scale,
        _ => None,
    }
}

#[test]
fn the_sample_reads_to_the_contracts_value() {
    let doc = kentos_kcad::decode(&read("block-extras.kcad")).expect("reads");
    assert_eq!(doc, content("block-extras.json"));
    assert_eq!(y_scale(&doc.entities[0]), Some(1.5));
    assert_eq!(y_scale(&doc.entities[2]), None);
    assert_eq!(y_scale(&doc.blocks[2].entities[0]), Some(1.0));
    assert_eq!(
        doc.blocks.iter().map(|b| b.units).collect::<Vec<_>>(),
        [
            Some(BlockUnits::Mm),
            Some(BlockUnits::Unitless),
            Some(BlockUnits::M)
        ]
    );
    assert_eq!(schema(&read("block-extras.kcad")), 57);
}

#[test]
fn they_come_back_through_the_codec_and_the_columns() {
    let doc = content("block-extras.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(back, doc);
    let (head, cols) = kentos_kcad::split(back).expect("splits");
    let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(again == bytes);
}

#[test]
fn only_a_stretched_insert_or_a_unit_is_schema_57() {
    let mut doc = content("block-extras.json");
    let plain = |doc: &mut DocumentSnapshotV2| {
        for e in doc.entities.iter_mut().chain(doc.blocks.iter_mut().flat_map(|b| b.entities.iter_mut())) {
            if let Entity::Insert(i) = e {
                i.y_scale = None;
            }
        }
        for b in &mut doc.blocks {
            b.units = None;
        }
    };
    plain(&mut doc);
    assert!(schema(&kentos_kcad::encode_verified(&doc).expect("writes")) < 57);
    // Each in turn is enough.
    let mut one = doc.clone();
    one.blocks[1].units = Some(BlockUnits::Unitless);
    assert_eq!(schema(&kentos_kcad::encode_verified(&one).expect("writes")), 57);
    let mut one = doc.clone();
    if let Entity::Insert(i) = &mut one.blocks[2].entities[0] {
        i.y_scale = Some(3.0);
    }
    assert_eq!(schema(&kentos_kcad::encode_verified(&one).expect("writes")), 57);
    // A y factor equal to the x one is refused: one way to write an insert, as the reader takes it.
    let mut one = doc.clone();
    if let Entity::Insert(i) = &mut one.entities[0] {
        i.y_scale = Some(i.scale);
    }
    let err = kentos_kcad::encode(&one).expect_err("refused");
    assert_eq!(err.code, Code::BadValue, "{err}");
}

#[test]
fn the_readers_errors_name_their_places() {
    for (file, code, place) in [
        ("broken/block-extras-in-schema-56.kcad", Code::UnknownField, "units"),
        ("broken/insert-y-scale-in-schema-56.kcad", Code::UnknownField, "yScale"),
        ("broken/block-units-in-schema-56.kcad", Code::UnknownField, "units"),
        ("broken/insert-y-scale-equal.kcad", Code::BadValue, "eşitse yazılmaz"),
        ("broken/insert-y-scale-zero.kcad", Code::BadValue, "pozitif"),
        ("broken/block-units-unknown.kcad", Code::BadValue, "blok birimi"),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(
            err.message.contains(place) || err.to_string().contains(place),
            "{file}: {err}"
        );
    }
}
