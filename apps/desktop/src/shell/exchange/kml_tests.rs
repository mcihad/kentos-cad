//! KML/KMZ in and out, GPX out and GeoJSON in WGS 84 driven as the user
//! drives them (docs/adr/0222), with the sample drawing (TUREF / TM36,
//! EPSG:5256) and fixtures/formats/v1/kml/parsel.kml: its parcel 101/7 as a
//! receiver measured it in WGS 84 (scripts/fixtures/kml_cases.py).

use std::path::PathBuf;

use kentos_commands::Entity;

use super::gis_import::{self, Source};
use super::tests::{count, run, said, send};
use super::{Event, Window, geojson_export, wgs84_export};
use crate::app::{App, Dialog, Message, Picker};
use crate::crs::Note;
use crate::files_testing::{app_with_drawing, last_said, scratch};

fn kml(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formats/v1/kml")
        .join(name)
}

fn window(app: &App) -> &gis_import::State {
    match &app.exchange {
        Some(Window::GisImport(s)) => s,
        other => panic!("the KML window, not {other:?}"),
    }
}

fn gis(app: &mut App, e: gis_import::Event) {
    send(app, Event::GisImport(e));
}

/// The parcel's KML in the TM36 drawing: not taken as it is; ticked, moved
/// into the project's system off the UI thread, its corner on the drawing's
/// corner, in one undo step.
#[test]
fn a_kml_goes_in_moved_into_the_projects_system_when_asked() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(kml("parsel.kml"));
    let before = count(&app);
    run(&mut app, "file.import.kml");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    let s = window(&app);
    assert!(matches!(s.source, Source::Kml(_)));
    assert_eq!(s.crs.srid, Some(4326), "KML is WGS 84");
    let own = app.project_system();
    assert!(!s.crs.ready(&own), "not taken as it is");
    let notes = s.crs.notes(&own, &kentos_interaction::Format::default());
    assert!(
        notes.iter().any(|n| matches!(n, Note::Info(i) if i.contains("“Projenin sistemine çevir”i işaretleyin") && i.contains("EPSG:5261"))),
        "{notes:?}"
    );
    gis(&mut app, gis_import::Event::Run);
    assert_eq!(count(&app), before, "nothing before the move is asked");
    gis(&mut app, gis_import::Event::Convert(true));
    let s = window(&app);
    assert!(s.crs.ready(&own));
    gis(&mut app, gis_import::Event::Run);
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    assert_eq!(count(&app) - before, 3);
    let doc = app.document.as_ref().expect("open");
    let parcel = doc
        .model
        .entities()
        .find_map(|e| match e {
            Entity::Polygon(p) if p.base.attrs.get("Ad").map(String::as_str) == Some("101/7") => {
                Some(p.clone())
            }
            _ => None,
        })
        .expect("the parcel");
    // The receiver's corner falls on the drawing's (the GNSS sample's positions are its TM36 coordinates through PROJ).
    let corner = parcel.pts[0];
    assert!(
        (corner.x - 486_512.352).abs() < 0.001 && (corner.y - 4_420_187.531).abs() < 0.001,
        "{corner:?}"
    );
    assert_eq!(
        parcel.base.attrs.get("Parsel").map(String::as_str),
        Some("7")
    );
    assert_eq!(parcel.base.color.as_deref(), Some("#2f8f2f"));
    assert!(
        last_said(&app).contains("Tek adımda geri alınabilir"),
        "{}",
        last_said(&app)
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before, "one undo step takes all of it back");
}

/// KML, KMZ and GPX out: the objects moved into WGS 84 and written where the
/// user says; what was written reads back.
#[test]
fn kml_kmz_and_gpx_are_written_in_wgs84() {
    let dir = scratch("kml-export");
    for (command, format, other, ext) in [
        ("file.export.kml", "kml", false, "kml"),
        ("file.export.kml", "kml", true, "kmz"),
        ("file.export.gpx", "gpx", true, "gpx"),
    ] {
        let mut app = app_with_drawing();
        let path = dir.join(format!("cizim.{ext}"));
        app.picker = Picker::File(path.clone());
        run(&mut app, command);
        assert!(
            matches!(app.exchange, Some(Window::Wgs84Export(_))),
            "{format}"
        );
        send(
            &mut app,
            Event::Wgs84Export(wgs84_export::Event::Scope(super::dxf_export::Scope::All)),
        );
        if other {
            send(
                &mut app,
                Event::Wgs84Export(wgs84_export::Event::Other(true)),
            );
        }
        send(&mut app, Event::Wgs84Export(wgs84_export::Event::Run));
        assert_eq!(app.dialog, None, "{format}: {:?}", app.exchange);
        assert!(
            said(&app, &format!("“cizim.{ext}” yazıldı")),
            "{}",
            last_said(&app)
        );
        // The sample's infinite lines and dimension are said, not written.
        assert!(
            last_said(&app).contains("Sonsuz doğru: 2"),
            "{}",
            last_said(&app)
        );
        let bytes = std::fs::read(&path).expect("written");
        match ext {
            "gpx" => {
                let text = String::from_utf8(bytes).expect("UTF-8");
                assert!(text.contains("<gpx version=\"1.1\" creator=\"KentOS CAD\""));
                assert!(
                    text.contains("<rte>") && text.contains("<wpt "),
                    "routes asked for"
                );
                let read = kentos_formats::gnss::read(text.as_bytes());
                assert!(
                    read.points
                        .iter()
                        .all(|p| (39.0..41.0).contains(&p.lat) && (35.0..37.0).contains(&p.lon))
                );
            }
            _ => {
                assert_eq!(bytes.starts_with(b"PK"), ext == "kmz");
                let back = kentos_formats::kml::read(&bytes, 0).expect("reads back");
                assert!(!back.entities.is_empty());
                let b = back.bounds.expect("a box");
                assert!(
                    (35.0..37.0).contains(&b.min_x) && (39.0..41.0).contains(&b.min_y),
                    "{b:?}"
                );
            }
        }
    }
}

/// GeoJSON's “WGS 84'e çevir”: RFC 7946, no crs member, degrees.
#[test]
fn geojson_is_written_in_wgs84_when_asked() {
    let dir = scratch("geojson-wgs84");
    let mut app = app_with_drawing();
    let path = dir.join("cizim.geojson");
    app.picker = Picker::File(path.clone());
    run(&mut app, "file.export.geojson");
    send(
        &mut app,
        Event::GeoJsonExport(geojson_export::Event::Scope(super::dxf_export::Scope::All)),
    );
    send(
        &mut app,
        Event::GeoJsonExport(geojson_export::Event::Wgs84(true)),
    );
    send(&mut app, Event::GeoJsonExport(geojson_export::Event::Run));
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    assert!(
        app.log
            .lines()
            .any(|l| l.text.starts_with("“cizim.geojson” yazıldı")
                && l.text.contains("RFC 7946, WGS 84'e çevrildi")),
        "{}",
        last_said(&app)
    );
    let text = std::fs::read_to_string(&path).expect("written");
    assert!(!text.contains("\"crs\""));
    let back = kentos_formats::geojson::read(
        text.as_bytes(),
        &kentos_commands::GeoJsonReadOptions {
            layer: "cizim".to_owned(),
            max_entities: 0,
        },
    )
    .expect("reads back");
    let b = back.bounds.expect("a box");
    assert!(
        (35.0..37.0).contains(&b.min_x) && (39.0..41.0).contains(&b.min_y),
        "{b:?}"
    );
}

/// Pictures of the windows, both themes, at 1440×900 and 1100×650.
///
/// ```text
/// cargo test -p kentos-desktop exchange::kml_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").unwrap_or_default();
    let wanted = |name: &str| only.is_empty() || only.split(',').any(|w| name.contains(w));
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            if !wanted(name) {
                return;
            }
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
            app.picker = Picker::File(kml("parsel.kml"));
            run(&mut app, "file.import.kml");
            picture(&mut app, &format!("kml-{mode}-1-ice-aktar"));
            gis(&mut app, gis_import::Event::Convert(true));
            picture(&mut app, &format!("kml-{mode}-2-cevrildi"));
            gis(&mut app, gis_import::Event::Run);
            picture(&mut app, &format!("kml-{mode}-3-alindi"));

            let mut app = fresh();
            run(&mut app, "file.export.kml");
            picture(&mut app, &format!("kml-{mode}-4-kml-ver"));

            let mut app = fresh();
            run(&mut app, "file.export.gpx");
            send(
                &mut app,
                Event::Wgs84Export(wgs84_export::Event::Other(true)),
            );
            picture(&mut app, &format!("kml-{mode}-5-gpx-ver"));

            let mut app = fresh();
            run(&mut app, "file.export.geojson");
            send(
                &mut app,
                Event::GeoJsonExport(geojson_export::Event::Wgs84(true)),
            );
            picture(&mut app, &format!("kml-{mode}-6-geojson-wgs84"));

            let mut app = fresh();
            app.picker = Picker::File(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gnss/v1/sample.gpx"),
            );
            run(&mut app, "file.import.gnss");
            picture(&mut app, &format!("kml-{mode}-7-gnss-cizgiler"));
            send(&mut app, Event::GnssImport(super::gnss_import::Event::Run));
            picture(&mut app, &format!("kml-{mode}-8-gnss-alindi"));
        }
    }
}
