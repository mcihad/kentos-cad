//! Pictures in the project file (docs/adr/0192, docs/specs/kcad-v2.md §6.1,
//! §6.6): document schema 24 is written only for a drawing with a picture;
//! every other drawing keeps its schema and its bytes. A picture is the
//! drawing's only; it comes back bit for bit; the writer refuses what the
//! reader would; the reader's errors name their places; the typed columns
//! carry it.

use kentos_kcad::contracts::{
    DocumentSnapshotV2, Entity, EntityBase, ImageEntity, ImageFields, Vec2,
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

fn picture(fields: impl FnOnce(&mut ImageFields)) -> Entity {
    let mut image = ImageFields {
        p: Vec2 { x: 0.0, y: 0.0 },
        width: 8.0,
        height: 6.0,
        rotation: 0.0,
        mirror: false,
        asset: Some("resim-0011223344556677".into()),
        file: None,
        clip: None,
        opacity: None,
    };
    fields(&mut image);
    Entity::Image(ImageEntity {
        base: EntityBase {
            id: 0,
            layer_id: "0".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
        },
        image,
    })
}

/// The minimal fixture's drawing (one point, schema 2) with `more` after its point.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x66u8; 16];
        uid[15] = n as u8;
        doc.uids.push(kentos_kcad::contracts::EntityId(uid));
        e.base_mut().id = n;
        doc.entities.push(e);
    }
    doc
}

#[test]
fn only_a_drawing_with_a_picture_is_schema_24() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("images.kcad");
    assert_eq!(schema(&data), 24);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    assert_eq!(of(&drawing(vec![picture(|_| {})])), 24);
    // Linked, clipped, mirrored and see-through, as well.
    let linked = picture(|i| {
        i.asset = None;
        i.file = Some("foto/saha.jpg".into());
        i.mirror = true;
        i.clip = Some(vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 1.0, y: 0.0 },
            Vec2 { x: 0.5, y: 1.0 },
        ]);
        i.opacity = Some(0.5);
    });
    assert_eq!(of(&drawing(vec![linked])), 24);
}

#[test]
fn pictures_come_back_bit_for_bit() {
    let doc = content("images.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let Entity::Image(linked) = &again.entities[1] else {
        panic!("a picture")
    };
    assert!(linked.image.mirror);
    assert_eq!(linked.image.file.as_deref(), Some("foto/saha.jpg"));
    assert_eq!(linked.image.opacity, Some(0.6));
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |e: Entity| kentos_kcad::encode(&drawing(vec![e])).expect_err("refused");
    for (e, place, words) in [
        (
            picture(|i| i.width = 0.0),
            "image/width",
            "genişliği ve yüksekliği",
        ),
        (
            picture(|i| i.file = Some("logo.png".into())),
            "image/asset",
            "ikisi birden değil",
        ),
        (picture(|i| i.asset = None), "image/asset", "kaynağı yok"),
        (
            picture(|i| {
                i.clip = Some(vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 1.0, y: 1.0 }]);
            }),
            "image/clip",
            "kırpma sınırı",
        ),
        (
            picture(|i| i.opacity = Some(0.05)),
            "image/opacity",
            "donukluğu",
        ),
    ] {
        let err = refused(e);
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains(place), "{err}");
        assert!(err.message.contains(words), "{err}");
    }
    // A number that is not finite is the float's own refusal.
    let err = refused(picture(|i| i.rotation = f64::NAN));
    assert_eq!(err.code, Code::NonFinite, "{err}");
    // A block definition holds no picture.
    let mut doc = content("blocks.json");
    let mut e = picture(|_| {});
    e.base_mut().id = doc.blocks[0].entities.len() as u32 + 1;
    doc.blocks[0].entities.push(e);
    let err = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(err.code, Code::BadValue);
    assert!(err.message.contains("blok tanımında resim olamaz"), "{err}");
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/image-in-schema-23.kcad",
            Code::UnknownKind,
            "entities/0/image",
        ),
        (
            "broken/image-in-block.kcad",
            Code::BadValue,
            "blok tanımında resim olamaz",
        ),
        (
            "broken/image-two-sources.kcad",
            Code::BadValue,
            "ikisi birden değil",
        ),
        ("broken/image-no-source.kcad", Code::BadValue, "kaynağı yok"),
        (
            "broken/image-width-zero.kcad",
            Code::BadValue,
            "genişliği ve yüksekliği",
        ),
        (
            "broken/image-clip-outside.kcad",
            Code::BadValue,
            "kırpma sınırı",
        ),
        (
            "broken/image-clip-two-corners.kcad",
            Code::BadValue,
            "kırpma sınırı",
        ),
        ("broken/image-opacity-low.kcad", Code::BadValue, "donukluğu"),
        (
            "broken/image-mirror-false.kcad",
            Code::BadValue,
            "image/mirror",
        ),
        (
            "broken/image-file-line-break.kcad",
            Code::BadValue,
            "denetim karakteri",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}

#[test]
fn the_typed_columns_carry_pictures() {
    let doc = content("images.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("images.kcad"));
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(
        serde_json::to_string(&entities).expect("serializes"),
        serde_json::to_string(&doc.entities).expect("serializes")
    );
}
