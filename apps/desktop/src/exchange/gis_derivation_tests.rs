//! GIS türevi dışa aktar and GIS türevini denetle on the desktop (docs/adr/0530): the export window plans the
//! derivation for its choices and writes through `file.export.gis`; the check window compares a derived GeoPackage
//! with the open drawing through `cad.gis.derivation`. Pictures: `cbs-turev-*` (`screens`, ignored).

use std::path::PathBuf;

use super::gis_check::Event as K;
use super::gis_export::{Event as X, Notes, Tolerance};
use super::tests::{run, said, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, scratch};
use kentos_commands::{DocumentSnapshotV1, GisAnnotations};

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
