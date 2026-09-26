//! The GeoJSON and Shapefile windows driven as the user drives them
//! (docs/adr/0046, 0053), with the files the shared readers are tested
//! with (fixtures/formats/v1/gis). The sample drawing is in TUREF / TM36
//! (EPSG:5256); its “Parsel” layer is open and its “Bina” layer locked.

use std::path::PathBuf;

use kentos_contracts::Entity;

use super::gis_import::{self, Source};
use super::tests::{count, run, said, send};
use super::{Event, Kind, Picked, Window, dxf_export, geojson_export};
use crate::app::{App, Dialog, Message, Picker};
use crate::crs::Note;
use crate::files_testing::{app_with_drawing, last_said, scratch};

fn gis(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formats/v1/gis")
        .join(name)
}

fn picked(name: &str) -> Picked {
    Picked {
        name: name.to_owned(),
        bytes: std::fs::read(gis(name)).expect("a fixture").into(),
    }
}

fn window(app: &App) -> &gis_import::State {
    match &app.exchange {
        Some(Window::GisImport(s)) => s,
        other => panic!("the GeoJSON/Shapefile window, not {other:?}"),
    }
}

fn import(app: &mut App) {
    send(app, Event::GisImport(gis_import::Event::Run));
}

fn warnings(app: &App) -> String {
    let format = kentos_interaction::Format::default();
    window(app)
        .crs
        .notes(5256, &format)
        .into_iter()
        .filter_map(|n| match n {
            Note::Warn(w) => Some(w),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn a_geojson_in_the_projects_system_goes_in_as_one_undo_step() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(gis("tm.geojson"));
    let before = count(&app);
    run(&mut app, "file.import.geojson");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    let s = window(&app);
    assert!(!s.reading && s.result.is_some(), "{:?}", s.failed);
    // The file's crs member names the project's system: that is the answer.
    assert_eq!(s.crs.srid, Some(5256));
    import(&mut app);
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    let model = &app.document.as_ref().expect("open").model;
    // “Parsel” joins the drawing's layer, the objects without a layer go to a
    // new “tm”, the locked “Bina” takes nothing: 1 area and 2 points of 5.
    assert_eq!(model.len() - before, 3);
    let parcel = model
        .entities()
        .find(|e| e.base().label.as_deref() == Some("101/5"))
        .expect("the parcel");
    assert_eq!(parcel.base().layer_id, "parsel");
    let attr = |k: &str| parcel.base().attrs.get(k).map(String::as_str);
    assert_eq!(
        (attr("mahalle"), attr("alan")),
        (Some("Çamlıbel"), Some("2345.678"))
    );
    assert_eq!(
        last_said(&app),
        "“tm.geojson”: 3 nesne 2 katmana alındı; 1 yeni katman “tm.geojson” grubunda. Tek adımda geri alınabilir."
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before, "one undo step takes all of it back");
}

#[test]
fn a_wgs84_geojson_is_refused_until_the_user_says_it_is_the_projects() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(gis("crs84.geojson"));
    let before = count(&app);
    run(&mut app, "file.import.geojson");
    assert_eq!(window(&app).crs.srid, Some(4326));
    import(&mut app);
    assert_eq!(count(&app), before, "another system: nothing goes in");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    // The user says the coordinates are TM36 after all: allowed, and the
    // window says the file says otherwise and that they look like degrees.
    send(&mut app, Event::GisImport(gis_import::Event::Crs(5256)));
    let w = warnings(&app);
    assert!(w.contains("Dosya EPSG:4326 diyor"), "{w}");
    assert!(w.contains("derece (boylam, enlem) gibi görünüyor"), "{w}");
    import(&mut app);
    assert!(count(&app) > before);
}

#[test]
fn a_shapefile_is_its_files_chosen_together() {
    let mut app = app_with_drawing();
    let before = count(&app);
    let files = [
        "noktalar.shp",
        "noktalar.shx",
        "noktalar.dbf",
        "noktalar.prj",
        "parseller.dbf",
    ]
    .map(picked)
    .to_vec();
    send(
        &mut app,
        Event::PickedMany(Kind::Shapefile, Some(Ok(files))),
    );
    let s = window(&app);
    assert!(s.result.is_some(), "{:?}", s.failed);
    // The .prj says TUREF / TM36.
    assert!(s.crs.matches(5256));
    let Source::Shapefile(files) = &s.source else {
        panic!("a layer's files")
    };
    let set = gis_import::shapefile_set(files).expect("a layer");
    assert_eq!(set.unused, ["parseller.dbf"]);
    let points = s.result.as_ref().map_or(0, |r| r.entities.len());
    import(&mut app);
    assert_eq!(count(&app) - before, points);
    assert!(said(&app, "“noktalar.shp”:"), "{:?}", app.history);
}

#[test]
fn a_zipped_shapefile_offers_its_layers_one_at_a_time() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(gis("katmanlar.zip"));
    let before = count(&app);
    run(&mut app, "file.import.shp");
    let s = window(&app);
    let Source::Zip { layers, layer, .. } = &s.source else {
        panic!("an archive")
    };
    assert_eq!(layers, &["katmanlar/parseller", "katmanlar/yollar"]);
    assert_eq!(*layer, 0);
    let first = s.result.as_ref().expect("the first layer read");
    assert_eq!(first.layers[0].name, "parseller");
    assert_eq!(first.report.source[0].label, "Zip arşivinden");
    // Another layer of the archive is read in its place.
    send(&mut app, Event::GisImport(gis_import::Event::Layer(1)));
    let s = window(&app);
    let read = s.result.as_ref().expect("the second layer read");
    assert_eq!(read.layers[0].name, "yollar");
    let roads = read.entities.len();
    // ED50 / TM30: said by its .prj, refused until the user answers otherwise.
    assert!(!s.crs.matches(5256));
    send(&mut app, Event::GisImport(gis_import::Event::Crs(5256)));
    import(&mut app);
    assert_eq!(count(&app) - before, roads);
    let model = &app.document.as_ref().expect("open").model;
    let group = model
        .layers()
        .nodes()
        .iter()
        .find(|n| n.name == "katmanlar.zip")
        .expect("a group named after the archive");
    assert_eq!(group.children[0].name, "yollar");
}

#[test]
fn files_without_a_layer_say_why_and_nothing_is_read() {
    let mut app = app_with_drawing();
    send(
        &mut app,
        Event::PickedMany(
            Kind::Shapefile,
            Some(Ok(vec![picked("parseller.dbf"), picked("parseller.prj")])),
        ),
    );
    let s = window(&app);
    assert!(s.failed.as_deref().is_some_and(|e| e.contains(".shp yok")));
    assert!(s.result.is_none() && !s.reading);
    // A broken archive: the reader's reason, never a panic.
    let mut app = app_with_drawing();
    let mut broken = picked("parseller.zip");
    broken.bytes = broken.bytes[..broken.bytes.len() / 2].to_vec().into();
    send(
        &mut app,
        Event::PickedMany(Kind::Shapefile, Some(Ok(vec![broken]))),
    );
    assert!(window(&app).failed.is_some());
}

#[test]
fn a_geojson_export_writes_what_the_reader_takes_back() {
    let mut app = app_with_drawing();
    let dir = scratch("export-geojson");
    let path = dir.join("cizim.geojson");
    app.picker = Picker::File(path.clone());
    run(&mut app, "file.export.geojson");
    send(
        &mut app,
        Event::GeoJsonExport(geojson_export::Event::Scope(dxf_export::Scope::All)),
    );
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    assert!(said(&app, "“cizim.geojson” yazıldı:"), "{:?}", app.history);
    let bytes = std::fs::read(&path).expect("written");
    let back = kentos_formats::geojson::read(
        &bytes,
        &kentos_contracts::GeoJsonReadOptions {
            layer: "cizim".into(),
            max_entities: 0,
        },
    )
    .expect("reads");
    let model = &app.document.as_ref().expect("open").model;
    let writable = model
        .entities()
        .filter(|e| {
            !matches!(
                e,
                Entity::Text(_) | Entity::Dimension(_) | Entity::Xline(_) | Entity::Ray(_)
            )
        })
        .count();
    assert_eq!(back.entities.len(), writable);
    // A TM36 project: its own coordinates, named by the crs member, not RFC 7946.
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("EPSG::5256"),
        "{}",
        &text[..200.min(text.len())]
    );
    assert!(!model.is_dirty(), "an export is not an edit");
}

/// Pictures of the GeoJSON and Shapefile windows for the owner, dark and
/// light, at 1440×900 and at the smallest window (1100×650), written to
/// `.run/shots` (never committed); not run by default:
///
/// ```text
/// cargo test -p kentos-desktop exchange::gis_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            let file = out.join(format!("{name}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for mode in ["dark", "light"] {
            let fresh = || {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app
            };
            let mut app = fresh();
            app.picker = Picker::File(gis("tm.geojson"));
            run(&mut app, "file.import.geojson");
            picture(&mut app, &format!("gis-{mode}-1-geojson"));

            let mut app = fresh();
            app.picker = Picker::File(gis("crs84.geojson"));
            run(&mut app, "file.import.geojson");
            picture(&mut app, &format!("gis-{mode}-2-wgs84"));
            send(&mut app, Event::GisImport(gis_import::Event::Crs(5256)));
            picture(&mut app, &format!("gis-{mode}-3-wgs84-proje-sistemi"));

            let mut app = fresh();
            app.picker = Picker::File(gis("katmanlar.zip"));
            run(&mut app, "file.import.shp");
            picture(&mut app, &format!("gis-{mode}-4-zip"));

            let mut app = fresh();
            let files = [
                "noktalar.shp",
                "noktalar.shx",
                "noktalar.dbf",
                "noktalar.prj",
                "parseller.dbf",
            ]
            .map(picked)
            .to_vec();
            send(
                &mut app,
                Event::PickedMany(Kind::Shapefile, Some(Ok(files))),
            );
            picture(&mut app, &format!("gis-{mode}-5-shapefile"));

            let mut app = fresh();
            run(&mut app, "file.export.geojson");
            picture(&mut app, &format!("gis-{mode}-6-geojson-ver"));
        }
    }
}
