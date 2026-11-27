//! The data layers' rows in GeoJSON dışa aktar and Paketle (docs/adr/9338; the CBS review of 27 November 2026, items 2
//! and 5). The project holds no object of its own: three data layers read a GeoPackage in EPSG:5256 (TUREF / TM36),
//! `qa_noktalar` with the points A, B, C, `qa_yollar` with the roads AB, BC and CB (BC again, the other way), `qa_secim`
//! with AB alone.
//!
//! - **GeoJSON:** while the sources are read the scopes wait and Dışa aktar does nothing; read, Görünen katmanlar counts
//!   what the map draws (the roads' filter lets two of three through) and Tümü every row. RFC 7946's longitudes and
//!   latitudes are those PROJ gives on its own (`cs2cs EPSG:5256 EPSG:4326`), the project's coordinates the source's
//!   metres with the old `crs` member.
//! - **Paketle:** the window counts each layer's rows before the folder is asked. The package copies the GeoPackage under
//!   `veri/`; its GeoPackage open file has the three tables with 3, 3 and 1 rows in 5256. Moved to another folder with
//!   the first file gone, the package's project reads its rows from its own copy. A source that cannot be read refuses
//!   the package with the layer's name and writes nothing.
//!
//! Pictures: `kullanim-9338` (`screens`, ignored).

use std::path::{Path, PathBuf};

use kentos_commands::ExchangeFormat;
use kentos_data::{Coord, Curve, Driver as _, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};

use super::dxf_export::Scope;
use super::package::Event as P;
use super::tests::{run, said, send};
use super::{Event, Window, geojson_export};
use crate::app::{App, Picker};
use crate::files_testing::{app_with_drawing, drive, scratch};

/// The three points, east and north in EPSG:5256.
const A: (f64, f64) = (487_000.0, 4_420_000.0);
const B: (f64, f64) = (487_100.0, 4_420_050.0);
const C: (f64, f64) = (487_200.0, 4_420_100.0);
/// A, B and C in WGS 84 longitude and latitude as PROJ 9 gives them (`cs2cs -f %.10f EPSG:5256 EPSG:4326`).
const A84: (f64, f64) = (35.847_954_652_0, 39.914_079_341_1);
const B84: (f64, f64) = (35.849_123_239_7, 39.914_531_183_8);

/// The GeoPackage of the review in `dir`: three tables in EPSG:5256.
pub(super) fn qa_gpkg(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    // Made anew: a picture's two themes make it twice.
    kentos_data::registry().forget(&path.display().to_string());
    let _ = std::fs::remove_file(&path);
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut tx = source.begin().expect("begin");
    for (table, kind) in [("qa_noktalar", "POINT"), ("qa_yollar", "LINESTRING"), ("qa_secim", "LINESTRING")] {
        tx.create_table(
            &NewTable {
                name: table.into(),
                title: None,
                description: None,
                key: "fid".into(),
                fields: vec![Field::new("ad", FieldKind::Text { max: Some(16) })],
                geometry: Some(GeometryField {
                    name: "geom".into(),
                    type_name: kind.into(),
                    z: false,
                    m: false,
                    srid: Some(5256),
                }),
                spatial_index: true,
                system: None,
            },
            false,
        )
        .expect("table");
    }
    let row = |ad: &str, geometry: Geometry| NewFeature {
        fid: None,
        geometry: Some(geometry),
        values: vec![("ad".into(), Value::Text(ad.into()))],
    };
    let point = |p: (f64, f64)| Geometry::Point(Some(Coord::xy(p.0, p.1)));
    let line = |a: (f64, f64), b: (f64, f64)| Geometry::Curve(Curve::Line(vec![Coord::xy(a.0, a.1), Coord::xy(b.0, b.1)]));
    tx.insert("qa_noktalar", &[row("A", point(A)), row("B", point(B)), row("C", point(C))]).expect("points");
    tx.insert("qa_yollar", &[row("AB", line(A, B)), row("BC", line(B, C)), row("CB", line(C, B))]).expect("roads");
    tx.insert("qa_secim", &[row("AB", line(A, B))]).expect("chosen");
    tx.commit().expect("commit");
    path
}

/// A project of the review: the sample drawing's objects taken away, the three tables of `file` added as data layers.
pub(super) fn scene(file: &Path) -> App {
    crate::data_layers::register();
    let mut app = app_with_drawing();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let all: Vec<kentos_domain::Slot> = model.entities().map(|e| kentos_domain::Slot(e.base().id)).collect();
        model.remove(&all);
        let mut settings = model.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Gis);
        model.set_settings(settings);
    }
    let address = file.display().to_string();
    let tables = ["qa_noktalar", "qa_yollar", "qa_secim"]
        .into_iter()
        .map(|t| crate::data_layers::describe(&address, t).expect("described"))
        .collect();
    app.add_data_layers(tables).expect("added");
    app
}

/// A fourth layer whose GeoPackage is not there any more (moved away by someone else): its rows cannot be read.
fn gone_layer(app: &mut App, dir: &Path) {
    let file = qa_gpkg(dir, "gidecek.gpkg");
    let described = crate::data_layers::describe(&file.display().to_string(), "qa_secim").expect("described");
    let added = app.add_data_layers(vec![described]).expect("added");
    let model = &mut app.document.as_mut().expect("a drawing").model;
    model.set_layer_data_address(&added[0], &dir.join("tasindi").join("gidecek.gpkg").display().to_string());
}

pub(super) fn layer_id(app: &App, name: &str) -> String {
    crate::data_layers_tests::node(app, name).expect("the layer").id.clone()
}

/// The sources' rows read, as the window's frame asks for them.
pub(super) fn read_rows(app: &mut App) {
    let task = app.data_rows_tasks();
    drive(app, task);
}

fn geojson(app: &App) -> &geojson_export::State {
    match &app.exchange {
        Some(Window::GeoJsonExport(s)) => s,
        _ => panic!("GeoJSON dışa aktar's window"),
    }
}

/// A feature's first position.
fn first(f: &serde_json::Value) -> (f64, f64) {
    let c = &f["geometry"]["coordinates"];
    let p = if c[0].is_array() { &c[0] } else { c };
    (p[0].as_f64().expect("x"), p[1].as_f64().expect("y"))
}

fn feature<'a>(collection: &'a serde_json::Value, layer: &str, ad: &str) -> &'a serde_json::Value {
    collection["features"]
        .as_array()
        .expect("features")
        .iter()
        .find(|f| f["properties"]["ad"] == ad && f["kentos"]["layer"] == layer)
        .unwrap_or_else(|| panic!("no {layer} {ad} in {collection}"))
}

#[test]
fn geojson_writes_the_data_layers_rows_from_their_sources() {
    let dir = scratch("kaynak-geojson");
    let mut app = scene(&qa_gpkg(&dir, "qa.gpkg"));
    let roads = layer_id(&app, "qa_yollar");
    let filtered = crate::files_testing::command(
        &mut app,
        "cad.layers.filter",
        serde_json::json!({ "layer": roads, "filter": { "expression": "ad <> 'BC'" } }),
    );
    assert_eq!(filtered["status"], "completed", "{filtered}");
    let wgs = dir.join("qa-wgs84.geojson");
    app.picker = Picker::File(wgs.clone());
    run(&mut app, "file.export.geojson");
    // The sources asked, not read yet: each scope waits for its rows, Dışa aktar does nothing.
    assert_eq!(geojson(&app).sources.reading(Scope::Visible).len(), 3);
    assert_eq!(app.geojson_scope_counts(), [0, 0, 0]);
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    assert!(!wgs.exists(), "nothing written while the sources are read");
    read_rows(&mut app);
    assert!(geojson(&app).sources.reading(Scope::All).is_empty());
    // Seçili, Görünen katmanlar (the roads' filter keeps AB and CB), Tümü.
    assert_eq!(app.geojson_scope_counts(), [0, 6, 7]);
    assert_eq!(geojson(&app).sources.filtered(&roads), Some((2, 3)));
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Scope(Scope::All)));
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Wgs84(true)));
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    assert!(said(&app, "“qa-wgs84.geojson” yazıldı: 7 nesne"), "{:?}", app.log.lines().map(|l| &l.text).collect::<Vec<_>>());
    let collection: serde_json::Value = serde_json::from_slice(&std::fs::read(&wgs).expect("written")).expect("JSON");
    assert_eq!(collection["features"].as_array().map(Vec::len), Some(7));
    assert!(collection.get("crs").is_none(), "RFC 7946 has no crs member");
    // Two lines and a point of the same collection, each its own geometry.
    assert_eq!(feature(&collection, "qa_noktalar", "A")["geometry"]["type"], "Point");
    assert_eq!(feature(&collection, "qa_secim", "AB")["geometry"]["type"], "LineString");
    for ((lon, lat), want) in [(first(feature(&collection, "qa_noktalar", "A")), A84), (first(feature(&collection, "qa_yollar", "BC")), B84)] {
        assert!((lon - want.0).abs() < 1e-7 && (lat - want.1).abs() < 1e-7, "({lon}, {lat}) for {want:?}");
    }
    // The project's coordinates: the source's metres as they are, named by the old crs member.
    let own = dir.join("qa-proje.geojson");
    app.picker = Picker::File(own.clone());
    run(&mut app, "file.export.geojson");
    read_rows(&mut app);
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Scope(Scope::Visible)));
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Wgs84(false)));
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    let collection: serde_json::Value = serde_json::from_slice(&std::fs::read(&own).expect("written")).expect("JSON");
    assert_eq!(collection["features"].as_array().map(Vec::len), Some(6), "the filtered road stays out");
    assert!(collection["crs"].to_string().contains("EPSG::5256"), "{}", collection["crs"]);
    assert_eq!(first(feature(&collection, "qa_noktalar", "A")), A);
    assert_eq!(first(feature(&collection, "qa_yollar", "CB")), C);
}

#[test]
fn a_source_that_cannot_be_read_is_named_and_the_rest_is_written() {
    let dir = scratch("kaynak-geojson-eksik");
    let file = qa_gpkg(&dir, "qa.gpkg");
    let mut app = scene(&file);
    gone_layer(&mut app, &dir);
    let path = dir.join("qa.geojson");
    app.picker = Picker::File(path.clone());
    run(&mut app, "file.export.geojson");
    read_rows(&mut app);
    let failed = geojson(&app).sources.failed(Scope::All);
    assert_eq!(failed.len(), 1, "{failed:?}");
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Scope(Scope::All)));
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    assert!(said(&app, "“qa.geojson” yazıldı: 7 nesne"), "{:?}", app.log.lines().map(|l| &l.text).collect::<Vec<_>>());
    assert!(
        app.log.lines().any(|l| l.text.contains("katmanı “qa.geojson” içine yazılmadı: kaynağı okunamadı")),
        "{:?}",
        app.log.lines().map(|l| &l.text).collect::<Vec<_>>()
    );
}

fn package(app: &App) -> &super::package::State {
    match &app.exchange {
        Some(Window::Package(s)) => s,
        _ => panic!("Paketle's window"),
    }
}

/// Rows of `table` in the GeoPackage at `path`, and its geometry column's system.
fn table_rows(path: &Path, table: &str) -> (i64, i64) {
    let gp = kentos_geopackage::GeoPackage::open(path, false).expect("opens");
    let rows = gp
        .connection()
        .query_row(&format!("SELECT count(*) FROM {}", kentos_geopackage::quote(table)), [], |r| r.get(0))
        .expect("counted");
    let srs = gp
        .connection()
        .query_row("SELECT srs_id FROM gpkg_geometry_columns WHERE table_name = ?1", [table], |r| r.get(0))
        .expect("its system");
    (rows, srs)
}

#[test]
fn paketle_carries_the_data_layers_sources_and_their_rows() {
    let dir = scratch("kaynak-paket");
    let file = qa_gpkg(&dir, "qa.gpkg");
    let mut app = scene(&file);
    let target = dir.join("teslim");
    app.picker = Picker::File(target.clone());
    run(&mut app, "file.package");
    send(&mut app, Event::Package(P::Format(ExchangeFormat::Gpkg)));
    // Before the folder: each layer's rows counted from the source, in the tree's order (the last added on top).
    read_rows(&mut app);
    assert_eq!(app.package_rows_said(), [Some(1), Some(3), Some(3)]);
    assert_eq!(package(&app).geopackages, kentos_commands::PackageWay::Copy);
    send(&mut app, Event::Package(P::Pack));
    assert!(said(&app, "Paket yapıldı:"), "{:?}", app.log.lines().map(|l| &l.text).collect::<Vec<_>>());
    assert!(target.join("veri/qa.gpkg").is_file(), "the source copied");
    let open = std::fs::read_dir(target.join("devir"))
        .expect("the open files")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "gpkg"))
        .expect("the GeoPackage open file");
    assert_eq!(table_rows(&open, "qa_noktalar"), (3, 5256));
    assert_eq!(table_rows(&open, "qa_yollar"), (3, 5256));
    assert_eq!(table_rows(&open, "qa_secim"), (1, 5256));
    // Moved to another folder, the first file gone: the package reads its own copy.
    let moved = dir.join("baska").join("teslim");
    std::fs::create_dir_all(moved.parent().expect("a parent")).expect("a folder");
    std::fs::rename(&target, &moved).expect("moved");
    std::fs::remove_file(&file).expect("the first file gone");
    let project = std::fs::read_dir(&moved)
        .expect("the package")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "kcad"))
        .expect("the project file");
    let session = kentos_headless::Session::open(&project).expect("opens");
    assert_eq!(session.located().layers_moved.len(), 3, "each layer found at veri/qa.gpkg");
    let doc = session.document();
    assert_eq!(doc.settings().srid, 5256);
    let counts: Vec<(String, usize, Option<u32>)> = doc
        .layers()
        .leaves()
        .into_iter()
        .filter(|l| l.data.is_some())
        .map(|l| {
            let read = kentos_native_application::data_write::all_rows(l, doc.settings(), &Default::default(), &|| false)
                .expect("read from the package's copy");
            (l.name.clone(), read.rows.len(), l.data.as_ref().and_then(|d| d.geometry.as_ref()).and_then(|g| g.srid))
        })
        .collect();
    assert_eq!(
        counts,
        [
            ("qa_secim".to_owned(), 1, Some(5256)),
            ("qa_yollar".to_owned(), 3, Some(5256)),
            ("qa_noktalar".to_owned(), 3, Some(5256))
        ]
    );
    assert!(
        doc.layers().leaves().iter().filter_map(|l| l.data.as_ref()).all(|d| d.relative.as_deref() == Some("veri/qa.gpkg")),
        "each layer's path from the project"
    );
}

#[test]
fn a_package_whose_source_cannot_be_read_writes_nothing() {
    let dir = scratch("kaynak-paket-eksik");
    let mut app = scene(&qa_gpkg(&dir, "qa.gpkg"));
    gone_layer(&mut app, &dir);
    let target = dir.join("teslim");
    app.picker = Picker::File(target.clone());
    run(&mut app, "file.package");
    send(&mut app, Event::Package(P::Format(ExchangeFormat::Gpkg)));
    read_rows(&mut app);
    assert_eq!(app.package_rows_said().first(), Some(&None), "the gone source is not counted");
    // The command on its own: refused with the layer's name, nothing written.
    let input = app.package_input(&target).expect("an input");
    let model = app.document.as_ref().expect("a drawing").model.clone();
    let refused = kentos_headless::package::plan(&model, &input).err().expect("refused");
    assert_eq!(refused.code, "source_unreachable", "{refused:?}");
    assert!(refused.message.contains("qa_secim"), "{}", refused.message);
    assert!(!target.exists(), "no folder made");
    // Without the open files the package is made, the gone file said as missing.
    send(&mut app, Event::Package(P::Format(ExchangeFormat::Gpkg)));
    send(&mut app, Event::Package(P::Pack));
    assert!(said(&app, "Paket yapıldı:"), "{:?}", app.log.lines().map(|l| &l.text).collect::<Vec<_>>());
    assert!(app.log.lines().any(|l| l.text.contains("tasindi")), "the missing file said");
}

/// A database's data layer copied (docs/adr/9338): with Veritabanı kaynakları on Kopyala its rows go into a GeoPackage
/// under `veri/`, and the package's layer reads that table without the database. On a real PostgreSQL when
/// `KENTOS_DATA_PG` names one (`postgresql://user@host:port/db`, its password in `PGPASSWORD`).
#[test]
fn a_database_layer_is_copied_as_a_geopackage() {
    let Some(address) = std::env::var("KENTOS_DATA_PG").ok().filter(|a| !a.is_empty()) else {
        eprintln!("KENTOS_DATA_PG yok: veritabanı katmanının paket kopyası atlandı");
        return;
    };
    crate::data_layers::register();
    let source = kentos_data::registry().source(&address).expect("the database");
    let table = format!("public.kentos_paket_{}", std::process::id());
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: table.clone(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![Field::new("ad", FieldKind::Text { max: Some(16) })],
            geometry: Some(GeometryField { name: "geom".into(), type_name: "POINT".into(), z: false, m: false, srid: Some(5256) }),
            spatial_index: true,
            system: None,
        },
        false,
    )
    .expect("table");
    let rows: Vec<NewFeature> = [("A", A), ("B", B), ("C", C)]
        .into_iter()
        .map(|(ad, p)| NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(p.0, p.1)))),
            values: vec![("ad".into(), Value::Text(ad.into()))],
        })
        .collect();
    tx.insert(&table, &rows).expect("rows");
    tx.commit().expect("commit");
    let dir = scratch("kaynak-paket-veritabani");
    let mut app = scene(&qa_gpkg(&dir, "qa.gpkg"));
    app.add_data_layers(vec![crate::data_layers::describe(&address, &table).expect("described")]).expect("added");
    let target = dir.join("teslim");
    app.picker = Picker::File(target.clone());
    run(&mut app, "file.package");
    send(&mut app, Event::Package(P::Databases(super::package::Way(kentos_commands::PackageWay::Copy))));
    send(&mut app, Event::Package(P::Format(ExchangeFormat::Gpkg)));
    read_rows(&mut app);
    assert_eq!(app.package_rows_said(), [Some(3), Some(1), Some(3), Some(3)]);
    send(&mut app, Event::Package(P::Pack));
    assert!(said(&app, "Paket yapıldı:"), "{:?}", app.log.lines().map(|l| &l.text).collect::<Vec<_>>());
    let short = table.rsplit('.').next().expect("a name").to_owned();
    let copy = target.join("veri").join(format!("{short}.gpkg"));
    assert_eq!(table_rows(&copy, &short), (3, 5256));
    let open = std::fs::read_dir(target.join("devir"))
        .expect("the open files")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "gpkg"))
        .expect("the GeoPackage open file");
    assert_eq!(table_rows(&open, &short).0, 3, "the database's rows in the open file too");
    let mut tx = source.begin().expect("begin");
    tx.drop_table(&table).expect("drop");
    tx.commit().expect("commit");
    // Moved, the database out of reach: the package's layer reads its GeoPackage.
    let moved = dir.join("baska");
    std::fs::rename(&target, &moved).expect("moved");
    let project = std::fs::read_dir(&moved)
        .expect("the package")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "kcad"))
        .expect("the project file");
    let session = kentos_headless::Session::open(&project).expect("opens");
    let doc = session.document();
    let node = doc.layers().leaves().into_iter().find(|l| l.name == short).expect("the layer");
    let data = node.data.as_ref().expect("a data layer");
    assert_eq!((data.driver.as_str(), data.relative.as_deref()), ("gpkg", Some(format!("veri/{short}.gpkg").as_str())));
    let read = kentos_native_application::data_write::all_rows(node, doc.settings(), &Default::default(), &|| false)
        .expect("read from the copy");
    assert_eq!(read.rows.len(), 3);
}

#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    let dir = scratch("kaynak-resim");
    let file = qa_gpkg(&dir, "qa.gpkg");
    crate::files_testing::usage_shots("9338", "01-geojson-kaynak", || {
        let mut app = scene(&file);
        run(&mut app, "file.export.geojson");
        read_rows(&mut app);
        app
    });
    crate::files_testing::usage_shots("9338", "02-paketle-kaynak", || {
        let mut app = scene(&file);
        run(&mut app, "file.package");
        send(&mut app, Event::Package(P::Format(ExchangeFormat::Gpkg)));
        read_rows(&mut app);
        app
    });
}
