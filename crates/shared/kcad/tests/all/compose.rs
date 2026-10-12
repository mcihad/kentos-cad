//! Layer compositions in the project file (docs/adr/0226 §3, §4,
//! docs/specs/kcad-v2.md §6.5): document schema 41 is written only for a
//! drawing with a layer's opacity, blending mode or mask; every other drawing
//! keeps its schema and its bytes. What says nothing (an opacity of 1, the
//! normal mode) is left out; they come back as they were; the writer refuses
//! what the reader would; the reader's errors name their places.

use kentos_kcad::Code;
use kentos_kcad::contracts::{
    BlendMode, DocumentSnapshotV2, LayerCompose, LayerMask, LayerNode, LayerNodeType,
};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says (one byte below 24, else the byte after 0x18).
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

fn opacity(o: f64) -> LayerCompose {
    LayerCompose {
        opacity: Some(o),
        ..LayerCompose::default()
    }
}

/// The minimal drawing with a second, empty layer `hat` to mask.
fn two_layers() -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    let mut hat = doc.layers[0].clone();
    hat.id = "hat".into();
    hat.name = "Hatlar".into();
    hat.children = Vec::new();
    doc.layers.push(hat);
    doc
}

fn masking(targets: &[&str]) -> LayerCompose {
    LayerCompose {
        mask: Some(LayerMask {
            labels: true,
            markers: false,
            size: 1.0,
            targets: targets.iter().map(|t| (*t).to_owned()).collect(),
        }),
        ..LayerCompose::default()
    }
}

fn layer<'a>(doc: &'a mut DocumentSnapshotV2, id: &str) -> &'a mut LayerNode {
    doc.layers
        .iter_mut()
        .find(|n| n.id == id)
        .expect("the layer")
}

#[test]
fn only_a_drawing_with_a_composition_is_schema_41() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("compose.kcad");
    assert_eq!(schema(&data), 41);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    let mut doc = content("minimal.json");
    doc.layers[0].compose = Some(opacity(0.5));
    assert_eq!(of(&doc), 41);
    // What says nothing is the documents' to leave out (`normalized`): a drawing without
    // a composition keeps its schema and its bytes.
    let plain = LayerCompose {
        opacity: Some(1.0),
        blend: Some(BlendMode::Normal),
        mask: None,
    };
    let mut doc = content("minimal.json");
    doc.layers[0].compose = plain.normalized();
    assert!(kentos_kcad::encode_verified(&doc).expect("writes") == read("minimal.kcad"));
    // The newest drawings before it keep their schemas.
    assert_eq!(of(&content("attachments.json")), 40);
    assert_eq!(of(&content("measures.json")), 39);
}

#[test]
fn they_come_back_as_they_were() {
    let doc = content("compose.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let altlik = again.layers[0].compose.as_ref().expect("a composition");
    assert_eq!(altlik.opacity, Some(0.6));
    assert_eq!(altlik.blend, Some(BlendMode::Multiply));
    assert!(again.layers[1].compose.is_none());
    let mask = again.layers[3]
        .compose
        .as_ref()
        .and_then(|c| c.mask.as_ref())
        .expect("a mask");
    assert!(mask.labels && mask.markers);
    assert_eq!(mask.targets, ["altlik", "hat"]);
    // Each of the modes is written and read back by its key.
    for blend in BlendMode::ALL.into_iter().skip(1) {
        let mut doc = content("compose.json");
        doc.layers[0].compose = Some(LayerCompose {
            blend: Some(blend),
            ..LayerCompose::default()
        });
        let again = kentos_kcad::decode(&kentos_kcad::encode_verified(&doc).expect("writes"))
            .expect("reads");
        assert_eq!(again.layers[0].compose, doc.layers[0].compose);
    }
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |doc: DocumentSnapshotV2| kentos_kcad::encode(&doc).expect_err("refused");
    // What says nothing is not written: an opacity of 1, the normal mode, nothing at all.
    for (compose, words) in [
        (opacity(1.0), "1 saydamlığı yazılmaz"),
        (
            LayerCompose {
                blend: Some(BlendMode::Normal),
                ..opacity(0.5)
            },
            "normal karışım kipi yazılmaz",
        ),
        (LayerCompose::default(), "katman çizimi boş"),
    ] {
        let mut doc = content("minimal.json");
        doc.layers[0].compose = Some(compose);
        let e = refused(doc);
        assert_eq!(e.code, Code::BadValue);
        assert!(e.message.contains(words), "{e}");
    }
    // A group has none.
    let mut doc = content("minimal.json");
    doc.layers[0].kind = LayerNodeType::Group;
    doc.layers[0].compose = Some(opacity(0.5));
    let e = refused(doc);
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("katman çizimi yalnız katmanın"), "{e}");
    // An opacity outside 0–1.
    let mut doc = content("minimal.json");
    doc.layers[0].compose = Some(opacity(1.5));
    let e = refused(doc);
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("saydamlık 0 ile 1 arasında"), "{e}");
    // A mask without a source.
    let mut doc = two_layers();
    let mut c = masking(&["hat"]);
    c.mask.as_mut().expect("a mask").labels = false;
    doc.layers[0].compose = Some(c);
    assert!(refused(doc).message.contains("kaynağı yok"));
    // A target the tree lacks (deleted, another editor's), itself or a group is
    // kept as written: skipped when drawn, never refused.
    let mut doc = two_layers();
    let own = doc.layers[0].id.clone();
    layer(&mut doc, "hat").kind = LayerNodeType::Group;
    doc.layers[0].compose = Some(masking(&["yol", own.as_str(), "hat"]));
    let again =
        kentos_kcad::decode(&kentos_kcad::encode_verified(&doc).expect("writes")).expect("reads");
    assert_eq!(again.layers[0].compose, doc.layers[0].compose);
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/compose-in-schema-40.kcad",
            Code::UnknownField,
            "layers/0/compose",
        ),
        ("broken/compose-on-group.kcad", Code::BadValue, "layers/0"),
        (
            "broken/compose-opacity-one.kcad",
            Code::BadValue,
            "compose/opacity",
        ),
        (
            "broken/compose-blend-normal.kcad",
            Code::BadValue,
            "compose/blend",
        ),
        (
            "broken/compose-unknown-field.kcad",
            Code::UnknownField,
            "compose/shadow",
        ),
        (
            "broken/mask-switch-off.kcad",
            Code::BadValue,
            "mask/markers",
        ),
        (
            "broken/mask-without-targets.kcad",
            Code::MissingField,
            "compose/mask",
        ),
        ("broken/mask-targets-empty.kcad", Code::BadValue, "compose"),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}
