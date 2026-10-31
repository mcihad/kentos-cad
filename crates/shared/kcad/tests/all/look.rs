//! An object's look and a layer's plot in the project file (docs/adr/0260, docs/specs/kcad-v2.md §6.6): document
//! schema 48 is written only for a drawing that has one; it comes back as written, through the codec and the typed
//! columns; the writer refuses what the reader would; the reader's errors name their places.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity, LineType, LookPart};
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

#[test]
fn the_sample_reads_to_the_contracts_value() {
    let doc = kentos_kcad::decode(&read("look.kcad")).expect("reads");
    assert_eq!(doc, content("look.json"));
    let look = &doc.entities[0].base().look;
    assert_eq!(look.line_type, Some(LineType::Dashed));
    assert_eq!(
        (look.line_type_scale, look.transparency, look.order),
        (Some(2.0), Some(0.5), Some(2))
    );
    assert_eq!(doc.entities[1].base().look.order, Some(-1));
    assert_eq!(doc.layers[2].plot, Some(false));
    assert_eq!(
        doc.blocks[0].entities[0].base().look.by_layer,
        vec![LookPart::Color, LookPart::LineWeight]
    );
}

#[test]
fn the_look_comes_back_through_the_codec_and_the_columns() {
    let mut doc = content("look.json");
    for (scale, order) in [
        (Some(1e-9), Some(i32::MIN)),
        (Some(1000.0), Some(0)),
        (None, None),
    ] {
        let look = &mut doc.entities[0].base_mut().look;
        look.line_type_scale = scale;
        look.order = order;
        let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
        let back = kentos_kcad::decode(&bytes).expect("reads");
        assert_eq!(back, doc, "{scale:?} {order:?}");
        let (head, cols) = kentos_kcad::split(back).expect("splits");
        let (again, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
        assert!(again == bytes, "{scale:?} {order:?}");
    }
}

#[test]
fn only_a_drawing_with_a_look_is_schema_48_and_others_keep_their_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("look.kcad");
    assert_eq!(schema(&data), 48);
    let mut doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    for e in &mut doc.entities {
        e.base_mut().look = Default::default();
    }
    // The block's object's and the layer's still make it 48.
    assert_eq!(of(&doc), 48);
    for e in &mut doc.blocks[0].entities {
        e.base_mut().look = Default::default();
    }
    assert_eq!(of(&doc), 48);
    doc.layers[2].plot = None;
    assert!(of(&doc) < 48);
    for (name, n) in [("nurbs", 47), ("polyline-extras", 46), ("requests", 45)] {
        let file = read(&format!("{name}.kcad"));
        assert_eq!(schema(&file), n, "{name}");
        let doc = kentos_kcad::decode(&file).expect("reads");
        assert!(kentos_kcad::encode(&doc).expect("writes") == file, "{name}");
    }
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |f: &dyn Fn(&mut DocumentSnapshotV2)| {
        let mut doc = content("look.json");
        f(&mut doc);
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    for (err, words) in [
        (
            refused(&|d| d.entities[0].base_mut().look.line_type_scale = Some(0.0)),
            "ölçeği",
        ),
        (
            refused(&|d| d.entities[0].base_mut().look.transparency = Some(0.95)),
            "Saydamlık",
        ),
        (
            refused(&|d| d.entities[0].base_mut().look.by_layer = vec![LookPart::Color]),
            "yalnız bloğun",
        ),
        (
            refused(&|d| d.blocks[0].entities[0].base_mut().look.order = Some(1)),
            "çizim sırası",
        ),
        (refused(&|d| d.layers[2].plot = Some(true)), "plot"),
    ] {
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains(words), "{err}");
    }
    let mut doc = content("look.json");
    let Entity::Line(l) = &mut doc.blocks[0].entities[0] else {
        panic!("a line")
    };
    l.base.color = Some("#FF0000".into());
    let err = kentos_kcad::encode(&doc).expect_err("own colour and by layer");
    assert!(err.message.contains("Renk"), "{err}");
}

#[test]
fn the_readers_errors_name_their_places() {
    for (file, code, place) in [
        (
            "broken/look-in-schema-47.kcad",
            Code::UnknownField,
            "line/lineType",
        ),
        (
            "broken/look-line-type-unknown.kcad",
            Code::BadValue,
            "lineType",
        ),
        ("broken/look-scale-zero.kcad", Code::BadValue, "ölçeği"),
        (
            "broken/look-transparency-high.kcad",
            Code::BadValue,
            "Saydamlık",
        ),
        (
            "broken/look-order-float.kcad",
            Code::WrongType,
            "line/order",
        ),
        (
            "broken/look-by-layer-in-drawing.kcad",
            Code::UnknownField,
            "line/byLayer",
        ),
        ("broken/plot-true.kcad", Code::BadValue, "layers/0/plot"),
        ("broken/schema-version-154.kcad", Code::SchemaVersion, ""),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(err.message.contains(place), "{file}: {err}");
    }
}
