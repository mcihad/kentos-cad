//! The new dimensions in the project file (docs/adr/0147, docs/specs/kcad-v2.md
//! §6.1, §6.6): document schema 9 is written only for a drawing with one of
//! the new kinds, a dimension's mask or a slope's elevations, in it or in a
//! block definition, and every other drawing keeps its schema and its bytes;
//! their fields come back bit for bit; the writer and the reader refuse the
//! same dimensions, naming their places; the typed columns carry them.

use kentos_kcad::contracts::{
    DimensionEntity, DimensionStyle, DocumentSnapshotV2, Entity, EntityBase, EntityId, Vec2,
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

/// An aligned dimension ten metres long, nothing optional.
fn dimension() -> DimensionEntity {
    DimensionEntity {
        base: EntityBase {
            id: 0,
            layer_id: "0".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        },
        a: v(500000.0, 4400000.0),
        b: v(500010.0, 4400000.0),
        offset: 2.0,
        height: 2.5,
        text: None,
        style: None,
        angle: None,
        c: None,
        mask: false,
        za: None,
        zb: None,
        look: Default::default(),
    }
}

/// The minimal drawing with `more` objects after its own.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x47u8; 16];
        uid[15] = n as u8;
        doc.uids.push(EntityId(uid));
        e.base_mut().id = n;
        doc.entities.push(e);
    }
    doc
}

#[test]
fn the_new_kinds_a_mask_and_elevations_and_only_they_make_schema_9() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let with = |d: DimensionEntity| of(&drawing(vec![Entity::Dimension(d)]));
    // The five older kinds stay schema 2.
    for style in [
        None,
        Some(DimensionStyle::Linear),
        Some(DimensionStyle::Radius),
    ] {
        assert_eq!(
            with(DimensionEntity {
                style,
                ..dimension()
            }),
            2
        );
    }
    assert_eq!(
        with(DimensionEntity {
            style: Some(DimensionStyle::Ordinate),
            angle: Some(90.0),
            ..dimension()
        }),
        9
    );
    assert_eq!(
        with(DimensionEntity {
            mask: true,
            ..dimension()
        }),
        9
    );
    assert_eq!(
        with(DimensionEntity {
            style: Some(DimensionStyle::Slope),
            za: Some(105.25),
            zb: Some(104.75),
            ..dimension()
        }),
        9
    );
    // In a block definition only.
    let doc = content("dimensions.json");
    let mut in_block = doc.clone();
    in_block.entities.clear();
    in_block.uids.clear();
    assert_eq!(of(&in_block), 9);
    // The fixture written by the independent Python writer is schema 9, and the codec writes its bytes.
    let data = read("dimensions.kcad");
    assert_eq!(schema(&data), 9);
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    // The older drawings keep their schemas and bytes.
    for (file, want) in [
        ("minimal.kcad", 2),
        ("drawing.kcad", 2),
        ("texts.kcad", 7),
        ("leaders.kcad", 8),
    ] {
        let data = read(file);
        assert_eq!(schema(&data), want, "{file}");
        let doc = kentos_kcad::decode(&data).expect("reads");
        assert!(kentos_kcad::encode(&doc).expect("writes") == data, "{file}");
    }
}

#[test]
fn every_field_of_the_new_dimensions_comes_back() {
    let doc = content("dimensions.json");
    let back = kentos_kcad::decode(&kentos_kcad::encode(&doc).expect("writes")).expect("reads");
    assert_eq!(back.entities, doc.entities);
    let dims: Vec<&DimensionEntity> = back
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some(d),
            _ => None,
        })
        .collect();
    assert_eq!(
        dims.iter().map(|d| d.style).collect::<Vec<_>>(),
        [
            Some(DimensionStyle::Ordinate),
            Some(DimensionStyle::Ordinate),
            Some(DimensionStyle::ArcLength),
            Some(DimensionStyle::Jogged),
            Some(DimensionStyle::Azimuth),
            Some(DimensionStyle::Slope),
            None
        ]
    );
    assert_eq!(
        dims.iter().map(|d| d.mask).collect::<Vec<_>>(),
        [false, true, false, false, false, true, true]
    );
    assert_eq!((dims[5].za, dims[5].zb), (Some(105.25), Some(104.75)));
    assert_eq!(dims[3].c, Some(v(500110.0, 4400030.0)));
    let Entity::Dimension(inside) = &back.blocks[0].entities[0] else {
        panic!("{:?}", back.blocks[0].entities)
    };
    assert_eq!(
        (inside.style, inside.za, inside.zb),
        (Some(DimensionStyle::Slope), Some(12.0), Some(11.8))
    );
}

#[test]
fn the_writer_refuses_what_a_reader_would_refuse_with_the_place() {
    let refuse = |d: DimensionEntity, code: Code, place: &str| {
        let err = kentos_kcad::encode(&drawing(vec![Entity::Dimension(d)])).expect_err("refused");
        assert_eq!(err.code, code, "{err}");
        assert!(err.message.contains(place), "{place}: {err}");
    };
    refuse(
        DimensionEntity {
            style: Some(DimensionStyle::ArcLength),
            ..dimension()
        },
        Code::MissingField,
        "entities/1/dimension/c",
    );
    refuse(
        DimensionEntity {
            style: Some(DimensionStyle::Slope),
            za: Some(105.0),
            ..dimension()
        },
        Code::MissingField,
        "entities/1/dimension/zb",
    );
    refuse(
        DimensionEntity {
            za: Some(105.0),
            ..dimension()
        },
        Code::BadValue,
        "entities/1/dimension/za",
    );
    refuse(
        DimensionEntity {
            style: Some(DimensionStyle::Ordinate),
            angle: Some(45.0),
            ..dimension()
        },
        Code::BadValue,
        "entities/1/dimension/angle",
    );
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |file: &str| kentos_kcad::decode(&read(file)).expect_err("refused");
    for (file, code, place) in [
        (
            "broken/dimension-ordinate-in-schema-8.kcad",
            Code::BadValue,
            "document/entities/0/dimension/style",
        ),
        (
            "broken/dimension-mask-in-schema-8.kcad",
            Code::UnknownField,
            "document/entities/0/dimension/mask",
        ),
        (
            "broken/dimension-arc-length-without-centre.kcad",
            Code::MissingField,
            "document/entities/0/dimension",
        ),
        (
            "broken/dimension-jogged-without-centre.kcad",
            Code::MissingField,
            "document/entities/0/dimension",
        ),
        (
            "broken/dimension-slope-without-elevations.kcad",
            Code::MissingField,
            "document/entities/0/dimension",
        ),
        (
            "broken/dimension-elevations-off-slope.kcad",
            Code::BadValue,
            "document/entities/0/dimension/za",
        ),
        (
            "broken/dimension-ordinate-axis.kcad",
            Code::BadValue,
            "document/entities/0/dimension/angle",
        ),
        (
            "broken/dimension-mask-false.kcad",
            Code::BadValue,
            "document/entities/0/dimension/mask",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}

#[test]
fn the_typed_columns_carry_the_new_dimensions() {
    let doc = content("dimensions.json");
    let (head, cols) = kentos_kcad::split(doc).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("dimensions.kcad"));
}
