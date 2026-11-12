//! GIS türevi dışa aktar and GIS türevini denetle on the desktop (docs/adr/0530, 0634): the export window plans the
//! derivation for its choices and writes through `file.export.gis` in the format chosen; the check window compares a
//! derived dataset with the open drawing through `cad.gis.derivation`, takes the GIS's changes back through
//! `cad.gis.takeback` and breaks the link through `cad.gis.detach`. Pictures: `cbs-turev-*` (`screens`, ignored).

use std::path::PathBuf;

use super::gis_check::Event as K;
use super::gis_export::{Event as X, Format, Notes, Tolerance};
use super::tests::{run, said, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, scratch};
use kentos_commands::{DocumentSnapshotV1, GisAnnotations, GisFormat};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/gis-derivation/v1")
        .join(name)
}

/// The app with the derivation's drawing: an arced parcel, a block, curves, a text.
fn gis_app() -> App {
    let json = std::fs::read_to_string(fixture("drawing.json")).expect("the drawing");
    let snapshot = DocumentSnapshotV1::from_json(&json).expect("reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn exporting(app: &App) -> &super::gis_export::State {
    match &app.exchange {
        Some(Window::GisExport(s)) => s,
        _ => panic!("the GIS export window"),
    }
}

fn checking(app: &App) -> &super::gis_check::State {
    match &app.exchange {
        Some(Window::GisCheck(s)) => s,
        _ => panic!("the GIS check window"),
    }
}

#[test]
fn the_window_plans_the_derivation_for_its_choices_and_writes_it() {
    let mut app = gis_app();
    run(&mut app, "file.export.gis");
    let plan = exporting(&app)
        .plan
        .as_ref()
        .expect("planned")
        .as_ref()
        .expect("a plan")
        .0
        .clone();
    let tables: Vec<&str> = plan.tables.iter().map(|t| t.table.as_str()).collect();
    assert!(
        tables.contains(&"blok_Bina") && tables.contains(&"kentos_annotations"),
        "{tables:?}"
    );
    assert!(plan.max_deviation <= 0.0001);
    assert_eq!(plan.chorded, 9);
    // A finer tolerance: more chords, the bound kept.
    send(&mut app, Event::GisExport(X::Tolerance(Tolerance(10))));
    let fine = exporting(&app)
        .plan
        .as_ref()
        .expect("planned")
        .as_ref()
        .expect("a plan")
        .0
        .clone();
    assert!(fine.max_deviation <= 0.00001, "{}", fine.max_deviation);
    assert!(fine.bytes > plan.bytes);
    // Texts and blocks left out: the tables of their own are gone, they are counted instead.
    send(
        &mut app,
        Event::GisExport(X::Annotations(Notes(GisAnnotations::Skip))),
    );
    let skipped = exporting(&app)
        .plan
        .as_ref()
        .expect("planned")
        .as_ref()
        .expect("a plan")
        .0
        .clone();
    assert!(skipped.tables.iter().all(|t| t.table != "blok_Bina"));
    assert!(skipped.left.iter().any(|l| l.kind == "insert"));
    // Written through the command.
    let path = scratch("gis-turev").join("ada.gpkg");
    send(&mut app, Event::GisExport(X::Picked(Some(path.clone()))));
    assert!(app.exchange.is_none(), "written and closed");
    assert!(path.exists());
    assert!(
        said(&app, &format!("{} yazıldı", path.display())),
        "{:?}",
        app.log.last().map(|l| l.text.clone())
    );
}

#[test]
fn the_check_says_what_matches_the_drawing_and_what_does_not() {
    let mut app = gis_app();
    // A derivative of this drawing: all of its rows are current.
    let dir = scratch("gis-denetle");
    let path = dir.join("ada.gpkg");
    run(&mut app, "file.export.gis");
    send(&mut app, Event::GisExport(X::Picked(Some(path.clone()))));
    app.picker = Picker::File(path);
    run(&mut app, "gis.derivation");
    let found = checking(&app)
        .found
        .as_ref()
        .expect("checked")
        .as_ref()
        .expect("an answer");
    assert_eq!((found.0.current, found.0.stale, found.0.edited), (17, 0, 0));
    assert!(found.0.same_project);
    // The same file against another drawing: no row has its source.
    let mut other = app_with_drawing();
    other.picker = Picker::File(fixture("ada.gpkg"));
    run(&mut other, "gis.derivation");
    let found = checking(&other)
        .found
        .as_ref()
        .expect("checked")
        .as_ref()
        .expect("an answer");
    assert_eq!(found.0.source_gone, 17);
    assert!(!found.0.same_project);
    assert!(found.1.iter().any(|w| w.code == "derivation_stale"));
    // The GIS's edits are told apart from the drawing's.
    let mut edited = gis_app();
    edited.picker = Picker::File(fixture("ada-duzenli.gpkg"));
    run(&mut edited, "gis.derivation");
    let found = checking(&edited)
        .found
        .as_ref()
        .expect("checked")
        .as_ref()
        .expect("an answer");
    assert_eq!(found.0.edited, 3);
    assert!(found.1.iter().any(|w| w.code == "derivation_edited"));
    // A file that is not a derivative is said so, and the window offers another.
    let mut plain = gis_app();
    plain.picker = Picker::File(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geopackage/v1/qgis.gpkg"),
    );
    run(&mut plain, "gis.derivation");
    let why = checking(&plain)
        .found
        .as_ref()
        .expect("checked")
        .as_ref()
        .expect_err("refused");
    assert!(why.contains("türetim kaydı yok"), "{why}");
    send(&mut plain, Event::GisCheck(K::Pick));
    assert!(plain.exchange.is_some());
}

#[test]
fn the_window_writes_a_shapefile_zip_and_asks_a_database_its_address() {
    let mut app = gis_app();
    run(&mut app, "file.export.gis");
    send(
        &mut app,
        Event::GisExport(X::Format(Format(GisFormat::Shapefile))),
    );
    let path = scratch("gis-turev-shp").join("ada.zip");
    send(&mut app, Event::GisExport(X::Picked(Some(path.clone()))));
    assert!(app.exchange.is_none(), "written and closed");
    assert!(path.exists());
    // PostGIS: an address with a password is refused and said in the window.
    run(&mut app, "file.export.gis");
    send(
        &mut app,
        Event::GisExport(X::Format(Format(GisFormat::PostGis))),
    );
    send(
        &mut app,
        Event::GisExport(X::Address(
            "postgresql://kentos:gizli@127.0.0.1/turev".into(),
        )),
    );
    send(&mut app, Event::GisExport(X::Save));
    let status = exporting(&app).status.clone().expect("said");
    assert!(status.0, "an error: {}", status.1);
}

#[test]
fn the_gis_changes_are_taken_back_or_the_link_broken() {
    let mut app = gis_app();
    app.picker = Picker::File(fixture("ada-geri.gpkg"));
    run(&mut app, "gis.derivation");
    let edited = checking(&app)
        .found
        .as_ref()
        .expect("checked")
        .as_ref()
        .expect("an answer")
        .0
        .edited;
    assert!(edited >= 8, "{edited}");
    // The GIS's attributes taken back: one undo step, the rows stale afterwards.
    send(&mut app, Event::GisCheck(K::TakeBack));
    let said_back = checking(&app).said.clone().expect("said");
    assert!(!said_back.0, "{}", said_back.1);
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(
        doc.model.undo_labels().first().map(String::as_str),
        Some(kentos_native_application::gis_takeback::LABEL)
    );
    let found = checking(&app)
        .found
        .as_ref()
        .expect("checked again")
        .as_ref()
        .expect("an answer");
    assert!(found.0.stale > 0, "{:?}", found.0);
    // With the geometry: the row whose GIS geometry its source cannot take (a curve made a point) comes with its
    // attributes alone, and the rest is written.
    let mut whole = gis_app();
    whole.picker = Picker::File(fixture("ada-geri.gpkg"));
    run(&mut whole, "gis.derivation");
    send(&mut whole, Event::GisCheck(K::Geometry(true)));
    send(&mut whole, Event::GisCheck(K::TakeBack));
    let said_whole = checking(&whole).said.clone().expect("said");
    assert!(!said_whole.0, "{}", said_whole.1);
    assert!(said_whole.1.contains("1 satırın GIS'teki geometrisi"), "{}", said_whole.1);
    // The link broken on a copy: the file is the GIS's own afterwards.
    let dir = scratch("gis-bag");
    let copy = dir.join("ada.gpkg");
    std::fs::copy(fixture("ada.gpkg"), &copy).expect("a copy");
    let mut other = gis_app();
    other.picker = Picker::File(copy.clone());
    run(&mut other, "gis.derivation");
    send(&mut other, Event::GisCheck(K::Detach));
    let said_detach = checking(&other).said.clone().expect("said");
    assert!(
        !said_detach.0 && said_detach.1.contains("6 tablonun bağı"),
        "{}",
        said_detach.1
    );
    assert!(checking(&other).found.as_ref().is_some_and(|f| f.is_err()));
}

#[test]
#[ignore = "pictures, run by hand"]
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
                let mut app = gis_app();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app
            };
            let mut app = fresh();
            run(&mut app, "file.export.gis");
            picture(&mut app, &format!("cbs-turev-{mode}-1-disa-aktar"));
            send(&mut app, Event::GisExport(X::Tolerance(Tolerance(1000))));
            send(
                &mut app,
                Event::GisExport(X::Annotations(Notes(GisAnnotations::Skip))),
            );
            picture(&mut app, &format!("cbs-turev-{mode}-2-atla"));
            send(
                &mut app,
                Event::GisExport(X::Format(Format(GisFormat::Shapefile))),
            );
            picture(&mut app, &format!("cbs-turev-{mode}-6-shapefile"));
            send(
                &mut app,
                Event::GisExport(X::Format(Format(GisFormat::PostGis))),
            );
            picture(&mut app, &format!("cbs-turev-{mode}-7-postgis"));
            let mut app = fresh();
            app.picker = Picker::File(fixture("ada-geri.gpkg"));
            run(&mut app, "gis.derivation");
            send(&mut app, Event::GisCheck(K::Geometry(true)));
            picture(&mut app, &format!("cbs-turev-{mode}-8-geri-al"));
            send(&mut app, Event::GisCheck(K::TakeBack));
            picture(&mut app, &format!("cbs-turev-{mode}-9-geri-alindi"));
            let mut app = fresh();
            app.picker = Picker::File(fixture("ada-shp-geri.zip"));
            run(&mut app, "gis.derivation");
            picture(&mut app, &format!("cbs-turev-{mode}-10-shapefile-denetle"));
            let mut app = fresh();
            app.picker = Picker::File(fixture("ada.gpkg"));
            run(&mut app, "gis.derivation");
            picture(&mut app, &format!("cbs-turev-{mode}-3-denetle-guncel"));
            let mut app = fresh();
            app.picker = Picker::File(fixture("ada-duzenli.gpkg"));
            run(&mut app, "gis.derivation");
            picture(
                &mut app,
                &format!("cbs-turev-{mode}-4-denetle-gis-degisiklik"),
            );
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            app.picker = Picker::File(fixture("ada.gpkg"));
            run(&mut app, "gis.derivation");
            picture(&mut app, &format!("cbs-turev-{mode}-5-denetle-eski"));
        }
    }
}
