//! Point clouds in the project file (docs/adr/0207 §3, docs/specs/kcad-v2.md
//! §6.1, §6.6): document schema 31 is written only for a drawing with a point
//! cloud or a raster read from an address; every other drawing keeps its
//! schema and its bytes. A cloud is the drawing's only; it comes back bit for
//! bit, its look with its fields' defaults left out; the writer refuses what
//! the reader would; the reader's errors name their places; the typed columns
//! carry it.

use kentos_kcad::contracts::{
    CloudFormat, CloudRender, CloudSource, DocumentSnapshotV2, Entity, EntityBase,
    PointCloudEntity, PointCloudFields, PointCloudStyle, PointShape, PointSizeUnit,
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

fn base() -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: "0".into(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
    }
}

const BOUNDS: [f64; 6] = [
    487_600.0,
    4_420_300.0,
    838.991,
    487_719.838,
    4_420_389.999,
    862.618,
];

fn cloud(fields: impl FnOnce(&mut PointCloudFields)) -> Entity {
    let mut cloud = PointCloudFields {
        sources: vec![CloudSource {
            asset: None,
            file: Some("bulut/koy.laz".into()),
            url: None,
            format: CloudFormat::Laz,
            count: 71_212,
            bounds: BOUNDS,
        }],
        bounds: BOUNDS,
        count: 71_212,
        srid: 5256,
        style: PointCloudStyle {
            render: CloudRender::Rgb,
            ramp: None,
            invert: false,
            min: None,
            max: None,
            hidden: Vec::new(),
            rgb8: false,
            size: 2.0,
            size_unit: PointSizeUnit::Px,
            shape: PointShape::Round,
        },
        opacity: None,
    };
    fields(&mut cloud);
    Entity::PointCloud(PointCloudEntity {
        base: base(),
        cloud,
    })
}

/// The rasters' fixture's ortofoto, read from an address.
fn raster_at(url: &str) -> Entity {
    let mut doc = content("rasters.json");
    let mut e = doc.entities.remove(0);
    let Entity::Raster(r) = &mut e else {
        panic!("the first object of rasters.json is a raster")
    };
    r.raster.asset = None;
    r.raster.file = None;
    r.raster.url = Some(url.into());
    e
}

/// The minimal fixture's drawing (one point, schema 2) with `more` after its point.
fn drawing(more: Vec<Entity>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    for mut e in more {
        let n = doc.entities.len() as u32 + 1;
        let mut uid = [0x78u8; 16];
        uid[15] = n as u8;
        doc.uids.push(kentos_kcad::contracts::EntityId(uid));
        e.base_mut().id = n;
        // minimal.json's one layer.
        e.base_mut().layer_id = "0".into();
        doc.entities.push(e);
    }
    doc
}

#[test]
fn only_a_drawing_with_a_cloud_or_an_address_is_schema_31() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("pointclouds.kcad");
    assert_eq!(schema(&data), 31);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    // A raster from its file stays schema 29; from an address it is 31.
    assert_eq!(of(&content("rasters.json")), 29);
    assert_eq!(
        of(&drawing(vec![raster_at("https://ornek.org/orto.tif")])),
        31
    );
    assert_eq!(of(&drawing(vec![cloud(|_| {})])), 31);
    // A virtual cloud of an address and an embedded file, every field of its look.
    let virtual_cloud = cloud(|c| {
        c.sources.push(CloudSource {
            asset: Some("pointcloud-0011223344556677".into()),
            file: None,
            url: None,
            format: CloudFormat::Xyz,
            count: 3,
            bounds: [
                487_650.0,
                4_420_320.0,
                844.12,
                487_651.0,
                4_420_320.5,
                844.25,
            ],
        });
        c.sources[0].file = None;
        c.sources[0].url = Some("https://ornek.org/koy.copc.laz".into());
        c.sources[0].format = CloudFormat::Copc;
        c.count = 71_215;
        c.style = PointCloudStyle {
            render: CloudRender::Elevation,
            ramp: Some("Viridis".into()),
            invert: true,
            min: Some(839.0),
            max: Some(863.0),
            hidden: vec![7, 18],
            rgb8: true,
            size: 0.5,
            size_unit: PointSizeUnit::M,
            shape: PointShape::Square,
        };
        c.opacity = Some(0.5);
    });
    assert_eq!(of(&drawing(vec![virtual_cloud])), 31);
}

#[test]
fn clouds_come_back_bit_for_bit() {
    let doc = content("pointclouds.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let Entity::PointCloud(virtual_cloud) = &again.entities[1] else {
        panic!("a point cloud")
    };
    let c = &virtual_cloud.cloud;
    assert_eq!(c.sources.len(), 2);
    assert_eq!(c.sources[0].format, CloudFormat::Copc);
    assert!(
        c.sources[0]
            .url
            .as_deref()
            .is_some_and(|u| u.starts_with("https://"))
    );
    assert_eq!(c.sources[1].file.as_deref(), Some("bulut/ada-2.laz"));
    assert_eq!(c.count, 2_230_000);
    assert_eq!(c.style.render, CloudRender::Classification);
    assert_eq!(c.style.hidden, vec![7, 18]);
    assert_eq!(c.style.size_unit, PointSizeUnit::M);
    assert_eq!(c.style.shape, PointShape::Square);
    assert_eq!(c.opacity, Some(0.8));
    let Entity::PointCloud(embedded) = &again.entities[2] else {
        panic!("a point cloud")
    };
    let e = &embedded.cloud;
    assert_eq!(e.sources[0].format, CloudFormat::Xyz);
    assert!(
        e.sources[0]
            .asset
            .as_deref()
            .is_some_and(|a| a.starts_with("pointcloud-"))
    );
    assert!(e.style.invert && e.style.rgb8);
    assert_eq!(e.style.ramp.as_deref(), Some("Spektral"));
    let Entity::Raster(orto) = &again.entities[3] else {
        panic!("a raster")
    };
    assert!(orto.raster.url.is_some() && orto.raster.file.is_none() && orto.raster.asset.is_none());
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |e: Entity| kentos_kcad::encode(&drawing(vec![e])).expect_err("refused");
    for (e, place, words) in [
        (
            cloud(|c| c.sources.clear()),
            "pointcloud/sources",
            "en az bir dosyası",
        ),
        (
            cloud(|c| c.sources[0].url = Some("https://ornek.org/koy.laz".into())),
            "pointcloud/sources",
            "yalnız biri",
        ),
        (
            cloud(|c| c.sources[0].file = None),
            "pointcloud/sources",
            "yalnız biri",
        ),
        (
            cloud(|c| {
                c.sources[0].file = None;
                c.sources[0].url = Some("ftp://ornek.org/koy.laz".into());
            }),
            "pointcloud/sources",
            "HTTPS adresi değil",
        ),
        (
            cloud(|c| c.sources[0].bounds = [1.0, 0.0, 0.0, 0.0, 1.0, 1.0]),
            "pointcloud/sources",
            "1. dosyanın kapsamı",
        ),
        (cloud(|c| c.count += 1), "pointcloud/count", "nokta sayısı"),
        (
            cloud(|c| c.bounds = [487_720.0, 4_420_300.0, 838.9, 487_600.0, 4_420_390.0, 862.7]),
            "pointcloud/bounds",
            "bulutunun kapsamı",
        ),
        (
            cloud(|c| c.style.size = 0.0),
            "pointcloud/style",
            "Noktanın boyu",
        ),
        (
            cloud(|c| c.style.hidden = vec![7, 2]),
            "pointcloud/style",
            "Gizlenen sınıflar",
        ),
        (
            cloud(|c| c.style.render = CloudRender::Elevation),
            "pointcloud/style",
            "aralığı",
        ),
        (
            cloud(|c| c.style.ramp = Some("Gökkuşağı".into())),
            "pointcloud/style",
            "Gökkuşağı",
        ),
        (
            cloud(|c| c.opacity = Some(0.05)),
            "pointcloud/opacity",
            "donukluğu",
        ),
        (
            {
                let mut e = raster_at("https://ornek.org/orto.tif");
                if let Entity::Raster(r) = &mut e {
                    r.raster.file = Some("orto.tif".into());
                }
                e
            },
            "raster/asset",
            "yalnız biri",
        ),
        (
            raster_at("ftp://ornek.org/orto.tif"),
            "raster/url",
            "HTTPS adresi değil",
        ),
    ] {
        let err = refused(e);
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains(place), "{err}");
        assert!(err.message.contains(words), "{err}");
    }
    // A number that is not finite is the float's own refusal.
    let err = refused(cloud(|c| c.bounds[0] = f64::NAN));
    assert_eq!(err.code, Code::NonFinite, "{err}");
    let err = refused(cloud(|c| c.sources[0].bounds[5] = f64::INFINITY));
    assert_eq!(err.code, Code::NonFinite, "{err}");
    // A block definition holds no cloud.
    let mut doc = content("blocks.json");
    let mut e = cloud(|_| {});
    e.base_mut().id = doc.blocks[0].entities.len() as u32 + 1;
    doc.blocks[0].entities.push(e);
    let err = kentos_kcad::encode(&doc).expect_err("refused");
    assert_eq!(err.code, Code::BadValue);
    assert!(
        err.message.contains("blok tanımında nokta bulutu olamaz"),
        "{err}"
    );
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/pointcloud-in-schema-30.kcad",
            Code::UnknownKind,
            "entities/0/pointcloud",
        ),
        (
            "broken/raster-url-in-schema-30.kcad",
            Code::UnknownField,
            "raster/url",
        ),
        (
            "broken/raster-url-and-file.kcad",
            Code::BadValue,
            "yalnız biri",
        ),
        (
            "broken/raster-url-ftp.kcad",
            Code::BadValue,
            "HTTPS adresi değil",
        ),
        (
            "broken/pointcloud-in-block.kcad",
            Code::BadValue,
            "blok tanımında nokta bulutu olamaz",
        ),
        (
            "broken/pointcloud-no-files.kcad",
            Code::BadValue,
            "en az bir dosyası",
        ),
        (
            "broken/pointcloud-two-sources.kcad",
            Code::BadValue,
            "yalnız biri",
        ),
        (
            "broken/pointcloud-no-source.kcad",
            Code::BadValue,
            "yalnız biri",
        ),
        (
            "broken/pointcloud-url-ftp.kcad",
            Code::BadValue,
            "HTTPS adresi değil",
        ),
        (
            "broken/pointcloud-format-unknown.kcad",
            Code::BadValue,
            "sources/0/format",
        ),
        (
            "broken/pointcloud-count-not-sum.kcad",
            Code::BadValue,
            "nokta sayısı",
        ),
        (
            "broken/pointcloud-bounds-five.kcad",
            Code::BadValue,
            "6 sayı olmalı",
        ),
        (
            "broken/pointcloud-bounds-upside-down.kcad",
            Code::BadValue,
            "bulutunun kapsamı",
        ),
        (
            "broken/pointcloud-ramp-without-range.kcad",
            Code::BadValue,
            "aralığı",
        ),
        (
            "broken/pointcloud-size-zero.kcad",
            Code::BadValue,
            "Noktanın boyu",
        ),
        (
            "broken/pointcloud-hidden-unsorted.kcad",
            Code::BadValue,
            "Gizlenen sınıflar",
        ),
        (
            "broken/pointcloud-hidden-empty.kcad",
            Code::BadValue,
            "style/hidden",
        ),
        (
            "broken/pointcloud-invert-false.kcad",
            Code::BadValue,
            "style/invert",
        ),
        (
            "broken/pointcloud-px-written.kcad",
            Code::BadValue,
            "style/sizeUnit",
        ),
        (
            "broken/pointcloud-round-written.kcad",
            Code::BadValue,
            "style/shape",
        ),
        (
            "broken/pointcloud-opacity-low.kcad",
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
fn the_typed_columns_carry_point_clouds() {
    let doc = content("pointclouds.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("pointclouds.kcad"));
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(
        serde_json::to_string(&entities).expect("serializes"),
        serde_json::to_string(&doc.entities).expect("serializes")
    );
}
