//! The Genelleştirme tools in the desktop's window (docs/adr/0221), on the
//! shared drawing (fixtures/processing/v1/generalize.kcad): every shared case
//! run from the window as a user fills it, and the pictures for the owner.

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Slot;
use serde_json::Value;

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/processing/v1/generalize.kcad");
const CASES: &str = include_str!("../../../../fixtures/processing/v1/generalize.json");

/// The app with the drawing open.
fn app_with_drawing() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Processing(e));
}

fn status(app: &App) -> RunStatus {
    app.processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

fn cases() -> Vec<Value> {
    let file: Value = serde_json::from_str(CASES).expect("the cases read");
    file["cases"].as_array().expect("cases").clone()
}

fn case(id: &str) -> Value {
    cases()
        .into_iter()
        .find(|c| c["id"] == id)
        .expect("the case")
}

/// A case's tool opened and filled as a user does, its selection made first and the choices that show other fields before them.
fn fill(app: &mut App, c: &Value) {
    if let Some(ids) = c["selection"].as_array() {
        app.selection.set(
            ids.iter()
                .filter_map(Value::as_u64)
                .map(|id| Slot(id as u32)),
        );
    }
    let tool = c["run"]["tool"].as_str().expect("a tool");
    let command: &'static str = Box::leak(format!("processing.run.{tool}").into_boxed_str());
    let _ = app.update(Message::Run(command));
    let values = c["values"].as_object().expect("values");
    let first = ["into"];
    for name in first {
        if let Some(v) = values.get(name) {
            event(app, Event::Value(name.into(), v.clone()));
        }
    }
    for (name, v) in values {
        if !first.contains(&name.as_str()) {
            event(app, Event::Value(name.clone(), v.clone()));
        }
    }
}

#[test]
fn every_shared_case_runs_from_the_window_as_it_says() {
    let mut seen = 0;
    for c in cases() {
        let mut app = app_with_drawing();
        fill(&mut app, &c);
        event(&mut app, Event::Run);
        let want = &c["expect"];
        let id = c["id"].as_str().unwrap_or("?");
        match status(&app) {
            RunStatus::Ok { text, .. } => assert_eq!(text, want["summary"], "{id}"),
            RunStatus::Error(text) => assert_eq!(text, want["message"], "{id}"),
            other => panic!("{id}: {other:?}"),
        }
        // The objects the run added are on the output layer, as many as the case says.
        let doc = &app.document.as_ref().expect("open").model;
        let added = want["added"]
            .as_array()
            .or(want["addedShapes"].as_array())
            .map_or(0, Vec::len);
        if let Some(layer) = want["layers"][0]["id"].as_str() {
            let on = doc
                .entities()
                .filter(|e| e.base().layer_id == layer)
                .count();
            assert_eq!(on, added, "{id}");
        }
        seen += 1;
    }
    assert!(seen >= 22, "{seen} cases");
}

/// The Genelleştirme tools for the owner: CBS's Analiz tab with the
/// Geometri panel's ▾ open on its seventeen tools under their categories'
/// names, Yumuşat's window and the smoothed lines, the coverage simplified,
/// the squared buildings, the simplified buildings, Alanları kümele's
/// window and its building blocks, the parts dropped and the small parcels
/// joined. Not run by default:
/// `cargo test -p kentos-desktop processing::generalize_tests::screens -- --ignored --nocapture --test-threads=1`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size, mouse};
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "serit",
        "yumusat",
        "yumusat-cizim",
        "kaplama-cizim",
        "dik-cizim",
        "bina-cizim",
        "kume",
        "kume-cizim",
        "parca-cizim",
        "kat-cizim",
    ];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in views {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                // Runs the cases and closes the window: the results on the drawing, the view on the layers named.
                let drawn = |app: &mut App, ids: &[&str], layers: &[&str]| {
                    for id in ids {
                        fill(app, &case(id));
                        event(app, Event::Run);
                        event(app, Event::Close);
                    }
                    let doc = &app.document.as_ref().expect("open").model;
                    let on: Vec<Slot> = doc
                        .entities()
                        .filter(|e| layers.contains(&e.base().layer_id.as_str()))
                        .map(|e| Slot(e.base().id))
                        .collect();
                    app.selection.set(on);
                    let _ = app.update(Message::Run("view.zoomSelection"));
                    app.selection.set(std::iter::empty());
                };
                match name {
                    // CBS's Analiz tab, the Geometri panel's ▾ open: Geometri üretimi's, Örnekleme's and Genelleştirme's tools under their names.
                    "serit" => {
                        let _ = app.update(Message::RibbonTab("analysis"));
                        snapshot.settle(&mut app, App::view, &mut update);
                        if let Some(at) =
                            crate::files_testing::find_text(&mut snapshot, &app, "Geometri")
                        {
                            let click = Point::new(at.x + at.width / 2.0, at.y + at.height / 2.0);
                            snapshot.step(
                                &mut app,
                                App::view,
                                &mut update,
                                &[
                                    iced::Event::Mouse(mouse::Event::CursorMoved {
                                        position: click,
                                    }),
                                    iced::Event::Mouse(mouse::Event::ButtonPressed(
                                        mouse::Button::Left,
                                    )),
                                    iced::Event::Mouse(mouse::Event::ButtonReleased(
                                        mouse::Button::Left,
                                    )),
                                ],
                            );
                        }
                    }
                    "yumusat" => {
                        fill(&mut app, &case("smooth-shared"));
                        event(&mut app, Event::Run);
                    }
                    "yumusat-cizim" => drawn(&mut app, &["smooth-lines"], &["yol"]),
                    "kaplama-cizim" => drawn(&mut app, &["coverage-blocked"], &["parsel"]),
                    "dik-cizim" => drawn(&mut app, &["square-default"], &["yamuk"]),
                    "bina-cizim" => drawn(&mut app, &["building-default"], &["bina"]),
                    "kume" => {
                        fill(&mut app, &case("aggregate-group"));
                        event(&mut app, Event::Run);
                    }
                    "kume-cizim" => drawn(&mut app, &["aggregate-default"], &["yapi"]),
                    "parca-cizim" => drawn(&mut app, &["parts-area"], &["ada"]),
                    _ => {
                        fill(&mut app, &case("eliminate-longest"));
                        event(&mut app, Event::Run);
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("islem-gen-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
