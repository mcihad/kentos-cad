//! Rasters in the project file (docs/adr/0204 §2, docs/specs/kcad-v2.md
//! §6.1, §6.6): document schema 29 is written only for a drawing with a
//! raster; every other drawing keeps its schema and its bytes. A raster is
//! the drawing's only; it comes back bit for bit, its look with its fields'
//! defaults left out; the writer refuses what the reader would; the reader's
//! errors name their places; the typed columns carry it.

use kentos_kcad::contracts::{
    DocumentSnapshotV2, Entity, EntityBase, RasterEntity, RasterFields, RasterRender,
    RasterResampling, RasterSample, RasterStretch, RasterStyle,
};
use kentos_kcad::{Code, Quiet};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says: the number after the `version`
/// key (one byte below 24, else the byte after 0x18).
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

fn raster(fields: impl FnOnce(&mut RasterFields)) -> Entity {
    let mut raster = RasterFields {
        affine: [487_100.0, 0.5, 0.0, 4_420_800.0, 0.0, -0.5],
        width: 2000,
        height: 1500,
        bands: 3,
        sample: RasterSample::U8,
        asset: None,
        file: Some("pafta-12.tif".into()),
        url: None,
        srid: 5256,
        style: RasterStyle {
            render: RasterRender::Rgb,
            bands: vec![1, 2, 3],
            stretch: RasterStretch::None,
            min: None,
            max: None,
            ramp: None,
            invert: false,
            azimuth: None,
            altitude: None,
            z_factor: None,
            nodata: None,
            resampling: RasterResampling::Bilinear,
            edges: None,
        },
        opacity: None,
        dataset: None,
    };
    fields(&mut raster);
    Entity::Raster(RasterEntity {
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
        raster,
    })
}

/// The minimal fixture's drawing (one point, schema 2) with `more` after its point.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x77u8; 16];
        uid[15] = n as u8;
        doc.uids.push(kentos_kcad::contracts::EntityId(uid));
        e.base_mut().id = n;
        doc.entities.push(e);
    }
    doc
}

#[test]
fn only_a_drawing_with_a_raster_is_schema_29() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("rasters.kcad");
    assert_eq!(schema(&data), 29);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    assert_eq!(of(&drawing(vec![raster(|_| {})])), 29);
    // Embedded, a shaded relief with every field of its look, see-through.
    let relief = raster(|r| {
        r.file = None;
        r.asset = Some("raster-0011223344556677".into());
        r.bands = 1;
        r.sample = RasterSample::F32;
        r.style = RasterStyle {
            render: RasterRender::RampShade,
            bands: vec![1],
            stretch: RasterStretch::Manual,
            min: Some(800.0),
            max: Some(1200.0),
            ramp: Some("Arazi".into()),
            invert: true,
            azimuth: Some(300.0),
            altitude: Some(40.0),
            z_factor: Some(2.0),
            nodata: Some(-9999.0),
            resampling: RasterResampling::Nearest,
            edges: None,
        };
        r.opacity = Some(0.5);
    });
    assert_eq!(of(&drawing(vec![relief])), 29);
}

#[test]
fn rasters_come_back_bit_for_bit() {
    let doc = content("rasters.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let Entity::Raster(dem) = &again.entities[1] else {
        panic!("a raster")
    };
    assert_eq!(dem.raster.style.render, RasterRender::RampShade);
    assert_eq!(dem.raster.style.resampling, RasterResampling::Nearest);
    assert!(dem.raster.style.invert);
    assert_eq!(dem.raster.opacity, Some(0.7));
    assert!(
        dem.raster
            .asset
            .as_deref()
            .is_some_and(|a| a.starts_with("raster-"))
    );
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |e: Entity| kentos_kcad::encode(&drawing(vec![e])).expect_err("refused");
    for (e, place, words) in [
        (
            raster(|r| r.affine = [0.0, 1.0, 1.0, 0.0, 1.0, 1.0]),
            "raster/affine",
            "tersinmiyor",
        ),
        (
            raster(|r| r.width = 0),
            "raster/width",
            "genişliği ve yüksekliği",
        ),
        (
            raster(|r| r.asset = Some("raster-0011223344556677".into())),
            "raster/asset",
            "yalnız biri",
        ),
        (raster(|r| r.file = None), "raster/asset", "kaynağı yok"),
        (
            raster(|r| r.style.bands = vec![1, 2]),
            "raster/style",
            "üç bant",
        ),
        (
            raster(|r| r.style.bands = vec![1, 2, 4]),
            "raster/style",
            "4. bant",
        ),
        (
            raster(|r| r.opacity = Some(0.05)),
            "raster/opacity",
            "donukluğu",
        ),
    ] {
        let err = refused(e);
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains(place), "{err}");
        assert!(err.message.contains(words), "{err}");
    }
    // A number that is not finite is the float's own refusal.
    let err = refused(raster(|r| r.affine[0] = f64::NAN));
    assert_eq!(err.code, Code::NonFinite, "{err}");
    // A block definition holds no raster.
    let mut doc = content("blocks.json");
    let mut e = raster(|_| {});
    e.base_mut().id = doc.blocks[0].entities.len() as u32 + 1;
    doc.blocks[0].entities.push(e);
    let err = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(err.code, Code::BadValue);
    assert!(
        err.message.contains("blok tanımında raster olamaz"),
        "{err}"
    );
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/raster-in-schema-28.kcad",
            Code::UnknownKind,
            "entities/0/raster",
        ),
        (
            "broken/raster-in-block.kcad",
            Code::BadValue,
            "blok tanımında raster olamaz",
        ),
        (
            "broken/raster-affine-flat.kcad",
            Code::BadValue,
            "tersinmiyor",
        ),
        (
            "broken/raster-affine-five.kcad",
            Code::BadValue,
            "6 sayı olmalı",
        ),
        (
            "broken/raster-width-zero.kcad",
            Code::BadValue,
            "genişliği ve yüksekliği",
        ),
        (
            "broken/raster-two-sources.kcad",
            Code::BadValue,
            "yalnız biri",
        ),
        (
            "broken/raster-no-source.kcad",
            Code::BadValue,
            "kaynağı yok",
        ),
        (
            "broken/raster-sample-unknown.kcad",
            Code::BadValue,
            "raster/sample",
        ),
        (
            "broken/raster-rgb-two-bands.kcad",
            Code::BadValue,
            "üç bant",
        ),
        ("broken/raster-band-missing.kcad", Code::BadValue, "4. bant"),
        (
            "broken/raster-stretch-none.kcad",
            Code::BadValue,
            "style/stretch",
        ),
        (
            "broken/raster-manual-upside-down.kcad",
            Code::BadValue,
            "Elle gerdirmenin",
        ),
        (
            "broken/raster-ramp-unknown.kcad",
            Code::BadValue,
            "Gökkuşağı",
        ),
        ("broken/raster-light-below.kcad", Code::BadValue, "ışığı"),
        (
            "broken/raster-invert-false.kcad",
            Code::BadValue,
            "style/invert",
        ),
        (
            "broken/raster-bilinear-written.kcad",
            Code::BadValue,
            "style/resampling",
        ),
        (
            "broken/raster-opacity-low.kcad",
            Code::BadValue,
            "donukluğu",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}

#[test]
fn the_typed_columns_carry_rasters() {
    let doc = content("rasters.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("rasters.kcad"));
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(
        serde_json::to_string(&entities).expect("serializes"),
        serde_json::to_string(&doc.entities).expect("serializes")
    );
}
