//! The Geometri üretimi tools in the desktop's window (docs/adr/0219), on
//! the shared drawing (fixtures/processing/v1/generate.kcad): every shared
//! case run from the window as a user fills it, and the pictures for the
//! owner.

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Slot;
use serde_json::Value;

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/processing/v1/generate.kcad");
const CASES: &str = include_str!("../../../../fixtures/processing/v1/generate.json");

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

/// A case's tool opened and filled as a user does, its selection made first and the grouping before its field.
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
    if let Some(group) = values.get("group") {
        event(app, Event::Value("group".into(), group.clone()));
    }
    for (name, v) in values {
        if name != "group" {
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
        let added = want["added"].as_array().map_or(0, Vec::len);
        if let Some(layer) = want["layers"][0]["id"].as_str() {
            let on = doc
                .entities()
                .filter(|e| e.base().layer_id == layer)
                .count();
            assert_eq!(on, added, "{id}");
        }
        seen += 1;
    }
    assert!(seen >= 20, "{seen} cases");
}

/// The Geometri üretimi tools for the owner: CBS's Analiz tab with the
/// Geometri panel's ▾ open, Voronoi alanları's window and its cells with the
/// Delaunay triangles, Sınırlayıcı geometri's window and the parcels' hulls,
/// rotated boxes and circles, the concave hulls of the C and the O, the
/// parcels' poles. Not run by default:
/// `cargo test -p kentos-desktop processing::generate_tests::screens -- --ignored --nocapture --test-threads=1`.
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
        "voronoi",
        "voronoi-cizim",
        "sinir",
        "sinir-cizim",
        "icbukey-cizim",
        "kutup-cizim",
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
                    // CBS's Analiz tab, the Geometri panel's ▾ open on Geometri üretimi's five.
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
                    "voronoi" => {
                        fill(&mut app, &case("voronoi-sites"));
                        event(&mut app, Event::Run);
                    }
                    "voronoi-cizim" => drawn(
                        &mut app,
                        &["voronoi-sites", "delaunay-named"],
                        &["islem-voronoi"],
                    ),
                    "sinir" => {
                        fill(&mut app, &case("bounding-rotated-ada"));
                        event(&mut app, Event::Run);
                    }
                    "sinir-cizim" => drawn(
                        &mut app,
                        &["bounding-rotated-ada", "bounding-circle"],
                        &["parsel", "diger"],
                    ),
                    "icbukey-cizim" => drawn(
                        &mut app,
                        &["concave-c", "concave-holes"],
                        &["kume", "halka"],
                    ),
                    _ => drawn(&mut app, &["pole-parcels"], &["parsel"]),
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("islem-uret-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
