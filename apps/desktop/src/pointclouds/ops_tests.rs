//! İşlemler's point cloud tools on the desktop, end to end (docs/adr/0207 §7):
//! the village's drawing with the reference's areas and a second cloud, each
//! tool run from its window as the user runs it (in the background, through
//! the desktop's files), its file written into a scratch folder and read back,
//! its object on the drawing; the answers are those of the independent
//! reference (fixtures/pointcloud/v1/ops.json).

use std::path::{Path, PathBuf};

use kentos_contracts::{CloudFormat, DocumentSnapshotV1, Entity};
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::processing::{Event, RunStatus};

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/pointclouds.kcad");

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn reference() -> Value {
    let text = std::fs::read_to_string(fixtures().join("pointcloud/v1/ops.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The village's drawing, the reference's areas on “alanlar”, the second cloud and the boundary's pattern beside.
fn app() -> App {
    crate::cloud_scenes::test_cache();
    let mut d: Value = serde_json::from_str(DRAWING).unwrap();
    let (x0, y0) = (crate::cloud_scenes::X0, crate::cloud_scenes::Y0);
    let p = |x: f64, y: f64| json!({ "x": x0 + x, "y": y0 + y });
    let rect = |r: [f64; 4]| vec![p(r[0], r[1]), p(r[2], r[1]), p(r[2], r[3]), p(r[0], r[3])];
    let layers = d["layers"].as_array_mut().unwrap();
    for (id, name) in [
        ("alanlar", "Alanlar"),
        ("ikinci", "İkinci"),
        ("sinir", "Sınır"),
    ] {
        layers.push(json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false,
            "expanded": true, "style": { "color": "#4C7A9A", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }));
    }
    let ops = fixtures().join("pointcloud/v1/ops");
    let cloud_of = |id: u32, layer: &str, file: &Path, count: u64, bounds: [f64; 6]| {
        json!({ "kind": "pointcloud", "id": id, "layerId": layer, "attrs": {},
            "sources": [{ "file": file.to_string_lossy(), "format": "laz", "count": count, "bounds": bounds }],
            "bounds": bounds, "count": count, "srid": 5256, "style": { "render": "elevation", "min": 0.0, "max": 1.0, "size": 2 } })
    };
    let head = |f: &Path| {
        let bytes = std::fs::read(f).unwrap();
        let mut o = kentos_pointcloud::source::Opening::new(bytes.len() as u64);
        let c = loop {
            match o.step().unwrap() {
                kentos_pointcloud::Step::Done(c) => break c,
                kentos_pointcloud::Step::Need(n) => o.put(
                    n.offset,
                    bytes[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
                ),
            }
        };
        (c.head.count, c.bounds())
    };
    let entities = d["entities"].as_array_mut().unwrap();
    entities.push(json!({ "kind": "polygon", "id": 20, "layerId": "alanlar", "attrs": { "Ad": "A" }, "pts": rect([4.0, 54.0, 36.0, 88.0]) }));
    entities.push(json!({ "kind": "circle", "id": 21, "layerId": "alanlar", "attrs": { "Ad": "B" }, "c": p(90.0005, 20.0005), "r": 12.0 }));
    entities.push(json!({ "kind": "polygon", "id": 22, "layerId": "alanlar", "attrs": { "Ad": "C" },
        "pts": rect([40.0, 4.0, 70.0, 34.0]), "holes": [{ "pts": rect([50.0, 14.0, 60.0, 24.0]) }] }));
    let ikinci = ops.join("ikinci.laz");
    let (n, b) = head(&ikinci);
    entities.push(cloud_of(23, "ikinci", &ikinci, n, b));
    let sinir = ops.join("sinir.laz");
    let (n, b) = head(&sinir);
    entities.push(cloud_of(24, "sinir", &sinir, n, b));
    let snapshot = DocumentSnapshotV1::from_json(&d.to_string()).expect("the drawing reads");
    let path = fixtures().join("interaction/v1/pointclouds.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

fn layer(id: &str) -> Value {
    json!({ "scope": "layer", "layerId": id })
}

/// Runs the tool whose window is open, in the background, to its end.
fn run(app: &mut App) -> RunStatus {
    let task = app.update(Message::Processing(Event::Run));
    crate::files_testing::drive(app, task);
    app.processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

fn ok_text(s: &RunStatus) -> String {
    match s {
        RunStatus::Ok { text, .. } => text.clone(),
        other => panic!("{other:?}"),
    }
}

/// A result file read back: its kind, points and classes' hash.
fn read_back(path: &Path) -> (kentos_pointcloud::source::Kind, u64, String) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut o = kentos_pointcloud::source::Opening::new(bytes.len() as u64);
    let c = loop {
        match o.step().unwrap() {
            kentos_pointcloud::Step::Done(c) => break c,
            kentos_pointcloud::Step::Need(n) => o.put(
                n.offset,
                bytes[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
            ),
        }
    };
    let mut recs = Vec::new();
    for r in c.runs() {
        let raw = &bytes[r.need.offset as usize..(r.need.offset + r.need.len) as usize];
        c.records(&r, raw, &mut recs).unwrap();
    }
    let l = c.layout;
    let classes: Vec<u8> = recs.chunks_exact(l.len).map(|r| l.class(r)).collect();
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in &classes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    (c.kind, c.head.count, format!("{h:016x}"))
}

/// The clouds the drawing has on `layer`, by their files.
fn clouds_on(app: &App, layer: &str) -> Vec<kentos_contracts::PointCloudFields> {
    let doc = app.document.as_ref().expect("open");
    doc.model
        .entities()
        .filter_map(|e| match e {
            Entity::PointCloud(c) if c.base.layer_id == layer => Some(c.cloud.clone()),
            _ => None,
        })
        .collect()
}

fn layer_named(app: &App, name: &str) -> String {
    let doc = app.document.as_ref().expect("open");
    doc.model
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == name)
        .map(|l| l.id.clone())
        .unwrap_or_else(|| panic!("{name}"))
}

#[test]
fn the_tools_write_the_references_answers() {
    let r = reference();
    let c = &r["cases"];
    let dir = crate::files_testing::scratch("bulut-islemleri");
    let mut app = app();

    // Alan sorgusu: the three areas' figures, a table; the drawing does not change.
    let _ = app.update(Message::Run("processing.run.pointcloud.areaStats"));
    value(&mut app, "input", layer("koy"));
    value(&mut app, "areas", layer("alanlar"));
    let s = run(&mut app);
    let RunStatus::Ok { table, .. } = &s else {
        panic!("{s:?}");
    };
    let table = table.clone().expect("the table");
    let counts: Vec<String> = table.rows.iter().map(|row| row[2].clone()).collect();
    let want: Vec<String> = c["areaStats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["count"].as_u64().unwrap().to_string())
        .collect();
    assert_eq!(counts, want);
    assert_eq!(table.rows[0][1], "A");

    // Seyrelt, Hücreyle 1 m: the cells' nearest points, as LAZ beside nothing else.
    let _ = app.update(Message::Run("processing.run.pointcloud.thin"));
    value(&mut app, "input", layer("koy"));
    value(&mut app, "size", json!(1.0));
    let out = dir.join("seyrek.laz");
    value(&mut app, "output", json!(out.to_string_lossy()));
    let s = run(&mut app);
    assert!(ok_text(&s).contains("13.967 nokta kaldı"), "{s:?}");
    let (kind, n, _) = read_back(&out);
    assert_eq!(
        (kind, n),
        (
            kentos_pointcloud::source::Kind::Laz,
            c["thinCell"]["count"].as_u64().unwrap()
        )
    );
    let added = clouds_on(&app, &layer_named(&app, "Seyreltilmiş bulut"));
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].count, n);
    assert_eq!(added[0].sources[0].format, CloudFormat::Laz);

    // Seyrelt as COPC: indexed from its LAZ, the LAZ gone.
    value(&mut app, "format", json!("copc"));
    let out = dir.join("seyrek-dizinli.copc.laz");
    value(&mut app, "output", json!(out.to_string_lossy()));
    let _ = run(&mut app);
    let (kind, n, _) = read_back(&out);
    assert_eq!(
        (kind, n),
        (
            kentos_pointcloud::source::Kind::Copc,
            c["thinCell"]["count"].as_u64().unwrap()
        )
    );
    assert!(!dir.join("seyrek-dizinli.copc.laz.yaziliyor.laz").exists());

    // Zemin süzgeci: the reference's classes.
    let _ = app.update(Message::Run("processing.run.pointcloud.ground"));
    value(&mut app, "input", layer("koy"));
    let out = dir.join("zemin.laz");
    value(&mut app, "output", json!(out.to_string_lossy()));
    let s = run(&mut app);
    assert!(ok_text(&s).contains("57.461 nokta zemin"), "{s:?}");
    let (_, n, classes) = read_back(&out);
    assert_eq!(n, 71_212);
    assert_eq!(classes, c["ground"]["classes"]);

    // Yüksekliğe göre sınıfla, every class but ground.
    let _ = app.update(Message::Run("processing.run.pointcloud.classify"));
    value(&mut app, "input", layer("koy"));
    value(&mut app, "all", json!(true));
    let out = dir.join("sinifli.las");
    value(&mut app, "format", json!("las"));
    value(&mut app, "output", json!(out.to_string_lossy()));
    let _ = run(&mut app);
    let (kind, _, classes) = read_back(&out);
    assert_eq!(kind, kentos_pointcloud::source::Kind::Las);
    assert_eq!(classes, c["height"]["classes"]);

    // Kırp: inside the three areas.
    let _ = app.update(Message::Run("processing.run.pointcloud.clip"));
    value(&mut app, "input", layer("koy"));
    value(&mut app, "areas", layer("alanlar"));
    let out = dir.join("kirpik.laz");
    value(&mut app, "output", json!(out.to_string_lossy()));
    let _ = run(&mut app);
    assert_eq!(read_back(&out).1, c["clip"]["count"].as_u64().unwrap());

    // Birleştir: the village and the second cloud, the latter's 0.4 mm grid rounded onto mm … the finest.
    let _ = app.update(Message::Run("processing.run.pointcloud.merge"));
    app.selection.set(
        app.document
            .as_ref()
            .unwrap()
            .model
            .entities()
            .filter(|e| matches!(e, Entity::PointCloud(c) if c.base.layer_id == "koy" || c.base.layer_id == "ikinci"))
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect::<Vec<_>>(),
    );
    value(&mut app, "input", json!({ "scope": "selection" }));
    let out = dir.join("birlesik.laz");
    value(&mut app, "output", json!(out.to_string_lossy()));
    let s = run(&mut app);
    assert!(ok_text(&s).contains("2 dosyanın"), "{s:?}");
    assert_eq!(read_back(&out).1, c["merge"]["count"].as_u64().unwrap());

    // Karola 50 m with its virtual cloud.
    let _ = app.update(Message::Run("processing.run.pointcloud.tile"));
    value(&mut app, "input", layer("koy"));
    value(&mut app, "size", json!(50.0));
    value(
        &mut app,
        "output",
        json!(dir.join("koy.laz").to_string_lossy()),
    );
    let _ = run(&mut app);
    let tiles = c["tile"]["tiles"].as_array().unwrap();
    for t in tiles {
        let (x, y, n) = (
            t[0].as_i64().unwrap(),
            t[1].as_i64().unwrap(),
            t[2].as_u64().unwrap(),
        );
        let path = dir.join(format!("koy_{}_{}.laz", x * 50, y * 50));
        assert_eq!(read_back(&path).1, n, "{}", path.display());
    }
    let vpc = std::fs::read_to_string(dir.join("koy.vpc")).unwrap();
    assert_eq!(
        kentos_pointcloud::vpc::read(&vpc).unwrap().len(),
        tiles.len()
    );
    let karolar = clouds_on(&app, &layer_named(&app, "Karolar"));
    assert_eq!(karolar[0].sources.len(), tiles.len());

    // Rasterleştir: the least heights in 2 m cells, a GeoTIFF and its raster.
    let _ = app.update(Message::Run("processing.run.pointcloud.rasterize"));
    value(&mut app, "input", layer("koy"));
    value(&mut app, "cell", json!(2.0));
    let out = dir.join("dem.tif");
    value(&mut app, "output", json!(out.to_string_lossy()));
    let s = run(&mut app);
    assert!(ok_text(&s).contains("60 × 45 hücrelik"), "{s:?}");
    assert!(std::fs::metadata(&out).unwrap().len() > 1000);
    let doc = app.document.as_ref().unwrap();
    let raster = doc
        .model
        .entities()
        .find_map(|e| match e {
            Entity::Raster(r) => Some(r.raster.clone()),
            _ => None,
        })
        .expect("the raster");
    assert_eq!((raster.width, raster.height), (60, 45));

    // Sınır çıkar on the pattern: six parts, two with a hole.
    let _ = app.update(Message::Run("processing.run.pointcloud.boundary"));
    value(&mut app, "input", layer("sinir"));
    value(&mut app, "cell", json!(1.0));
    value(&mut app, "least", json!(2));
    let s = run(&mut app);
    assert!(ok_text(&s).contains("6 parçalı"), "{s:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
