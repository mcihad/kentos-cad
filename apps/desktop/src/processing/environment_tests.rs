//! Analiz ortamı in the desktop's window (docs/adr/0643 §5), over the environment cases' drawing
//! (fixtures/raster-env/v1, opened beside its rasters): the section shows for a raster tool and keeps the session's
//! environment from window to window; Raster hesaplayıcı with a cell size and a snap raster writes its result on the
//! new lattice; a number that does not read keeps the run from starting; a tool that does not take the environment
//! does not show it and runs as ever.

use std::path::PathBuf;

use kentos_commands::{DocumentSnapshotV1, Entity};
use serde_json::{Value, json};

use super::environment::{AreaFrom, EnvironmentEvent};
use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/raster-env/v1/drawing.kcad");

fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/raster-env/v1/drawing.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

fn env(app: &mut App, e: EnvironmentEvent) {
    let _ = app.update(Message::Processing(Event::Environment(e)));
}

fn uid_on(app: &App, layer: &str) -> String {
    let doc = &app.document.as_ref().expect("open").model;
    doc.entities_with_uids()
        .find(|(_, e)| e.base().layer_id == layer)
        .map(|(u, _)| u.to_string())
        .unwrap_or_else(|| panic!("{layer}"))
}

/// Raster hesaplayıcı on D16, its result written at `out`.
fn calculator(app: &mut App, out: &std::path::Path) {
    let _ = app.update(Message::Run("processing.run.raster.calculator"));
    value(app, "input", json!({ "scope": "layer", "layerId": "d16" }));
    value(app, "expression", json!("[D16] * 2"));
    value(app, "output", json!(out.to_string_lossy()));
}

#[test]
fn a_cell_size_and_a_snap_raster_give_the_result_a_new_lattice() {
    let dir = crate::files_testing::scratch("ortam-hesap");
    let out = dir.join("hesap.tif");
    let mut app = app();
    calculator(&mut app, &out);
    assert!(
        app.processing
            .dialog
            .as_ref()
            .expect("open")
            .takes_environment
    );
    let hiza = uid_on(&app, "hiza");
    env(&mut app, EnvironmentEvent::Toggle);
    env(&mut app, EnvironmentEvent::Cell("25".into()));
    env(&mut app, EnvironmentEvent::Snap(Some(hiza)));
    env(&mut app, EnvironmentEvent::Nodata("-9999".into()));
    let status = super::surface_tests::run(&mut app);
    assert!(matches!(status, RunStatus::Ok { .. }), "{status:?}");
    assert!(out.exists(), "the result is written");
    let doc = &app.document.as_ref().expect("open").model;
    let made = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Raster(r) if r.raster.file.as_deref() == Some(out.to_str().unwrap()) => Some(r),
            _ => None,
        })
        .next()
        .expect("the result's raster object");
    // 25 m cells on the snap raster's lines (x ≡ 1, y ≡ 299 mod 25) over the DEM's whole extent.
    assert_eq!(
        made.raster.affine,
        [499976.0, 25.0, 0.0, 4420324.0, 0.0, -25.0]
    );
    assert_eq!((made.raster.width, made.raster.height), (17, 13));
}

#[test]
fn the_session_keeps_its_environment_and_a_bad_number_stops_the_run() {
    let dir = crate::files_testing::scratch("ortam-oturum");
    let mut app = app();
    calculator(&mut app, &dir.join("a.tif"));
    env(&mut app, EnvironmentEvent::Area(AreaFrom::View));
    env(&mut app, EnvironmentEvent::Cell("on".into()));
    let _ = app.update(Message::Processing(Event::Close));
    // Another raster tool's window takes the session's environment.
    let _ = app.update(Message::Run("processing.run.raster.focalStatistics"));
    let w = app.processing.dialog.as_ref().expect("open");
    assert_eq!(
        (w.environment.area, w.environment.cell.as_str()),
        (AreaFrom::View, "on")
    );
    let status = super::surface_tests::run(&mut app);
    assert!(
        matches!(&status, RunStatus::Invalid(why) if why == "Ortamın hücre boyu bir sayı olmalı."),
        "{status:?}"
    );
    env(&mut app, EnvironmentEvent::Clear);
    assert_eq!(app.processing.environment.count(), 0);
}

#[test]
fn a_tool_that_does_not_take_it_neither_shows_nor_reads_it() {
    let mut app = app();
    env(&mut app, EnvironmentEvent::Cell("5".into()));
    let _ = app.update(Message::Run("processing.run.attributes.calculate"));
    assert!(
        !app.processing
            .dialog
            .as_ref()
            .expect("open")
            .takes_environment
    );
    assert_eq!(app.processing_environment(), Ok(None));
}

#[test]
#[ignore = "pictures: cargo test -p kentos-desktop processing::environment_tests::screens -- --ignored --nocapture"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["acik", "kapali", "hata"] {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let dir = crate::files_testing::scratch(&format!("ortam-resim-{name}"));
                let mut app = app();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                calculator(&mut app, &dir.join("hesap.tif"));
                let hiza = uid_on(&app, "hiza");
                if name != "kapali" {
                    env(&mut app, EnvironmentEvent::Toggle);
                }
                env(&mut app, EnvironmentEvent::Area(AreaFrom::View));
                env(
                    &mut app,
                    EnvironmentEvent::Cell(if name == "hata" {
                        "5,x".into()
                    } else {
                        "25".into()
                    }),
                );
                env(&mut app, EnvironmentEvent::Snap(Some(hiza)));
                env(&mut app, EnvironmentEvent::Nodata("-9999".into()));
                if name == "hata" {
                    let _ = super::surface_tests::run(&mut app);
                }
                snapshot.settle(&mut app, App::view, &mut update);
                snapshot.operate(app.view(), Box::new(crate::files_testing::SnapDown));
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("islem-ortam-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
