//! The Katman tools in the desktop's window (docs/adr/0217), on the shared
//! drawing (fixtures/processing/v1/layers.kcad): every shared case run from
//! the window as a user fills it, the Katmanlar list's check, and the
//! pictures for the owner.

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Slot;
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/processing/v1/layers.kcad");
const CASES: &str = include_str!("../../../../fixtures/processing/v1/layers.json");

/// The app with the drawing open.
fn app_with_layers() -> App {
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

/// A case's tool opened and filled as a user does, its selection made first.
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
    for (name, v) in c["values"].as_object().expect("values") {
        event(app, Event::Value(name.clone(), v.clone()));
    }
}

#[test]
fn every_shared_case_runs_from_the_window_as_it_says() {
    let mut seen = 0;
    for c in cases() {
        let mut app = app_with_layers();
        fill(&mut app, &c);
        event(&mut app, Event::Run);
        let want = &c["expect"];
        let id = c["id"].as_str().unwrap_or("?");
        match (want["status"].as_str(), status(&app)) {
            (Some("ok"), RunStatus::Ok { text, table, .. }) => {
                assert_eq!(text, want["summary"], "{id}");
                if let Some(t) = want["outputs"].get("table") {
                    let table = table.expect("the table");
                    assert_eq!(
                        json!({ "columns": table.columns, "rows": table.rows }),
                        *t,
                        "{id}"
                    );
                }
                let doc = &app.document.as_ref().expect("open").model;
                let names: Vec<&str> = doc
                    .layers()
                    .leaves()
                    .iter()
                    .map(|l| l.name.as_str())
                    .collect();
                for l in want["layers"].as_array().into_iter().flatten() {
                    let name = l["name"].as_str().unwrap_or("");
                    assert!(names.contains(&name), "{id}: {name} in {names:?}");
                }
            }
            // The window's check keeps it from starting, or the run's: the reason under the field.
            (Some("invalid"), RunStatus::Idle | RunStatus::Invalid(_)) => {
                let param = want["issues"][0]["param"].as_str().unwrap_or("");
                let issue = app
                    .processing
                    .dialog
                    .as_ref()
                    .and_then(|w| w.issue_of(param))
                    .map(|i| i.message.clone());
                assert_eq!(json!(issue), want["issues"][0]["message"], "{id}");
            }
            (_, got) => panic!("{id}: {got:?}"),
        }
        seen += 1;
    }
    assert!(seen >= 15, "{seen} cases");
}

/// The Katman tools for the owner: CBS's Veri tab with them in its Katman
/// panel, Özniteliğe göre böl's report and its new group in the layer tree,
/// Vektör katmanlarını birleştir's checked list and field map, Yinelenenleri
/// sil's report, İfadeye göre sırala's form. Not run by default:
/// `cargo test -p kentos-desktop processing::layer_tools_tests::screens -- --ignored --nocapture --test-threads=1`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "serit",
        "bol",
        "bol-agac",
        "birlestir",
        "yinelenen",
        "sirala",
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
                let mut app = app_with_layers();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let scroll = |snapshot: &mut Snapshot, app: &mut App, lines: f32| {
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    snapshot.step(
                        app,
                        App::view,
                        &mut update,
                        &[
                            iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                                position: iced::Point::new(width / 2.0 - 120.0, height / 2.0),
                            }),
                            iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                                delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: -lines },
                            }),
                        ],
                    );
                    snapshot.settle(app, App::view, &mut update);
                };
                match name {
                    // CBS's Veri tab: the five beside Yeni katman in the Katman panel.
                    "serit" => {
                        let _ = app.update(Message::RibbonTab("data"));
                        snapshot.settle(&mut app, App::view, &mut update);
                    }
                    "bol" => {
                        fill(&mut app, &case("split-copy"));
                        event(&mut app, Event::Run);
                        snapshot.settle(&mut app, App::view, &mut update);
                        scroll(&mut snapshot, &mut app, 40.0);
                    }
                    "bol-agac" => {
                        fill(&mut app, &case("split-copy"));
                        event(&mut app, Event::Run);
                        // The window closed: the new group and its layers in the tree, the view on the drawing.
                        event(&mut app, Event::Close);
                        let _ = app.update(Message::Run("view.zoomExtents"));
                        snapshot.settle(&mut app, App::view, &mut update);
                    }
                    "birlestir" => {
                        let mut c = case("merge-typed");
                        c["values"]["sourceField"] = json!("Kaynak katman");
                        fill(&mut app, &c);
                        snapshot.settle(&mut app, App::view, &mut update);
                    }
                    "yinelenen" => {
                        fill(&mut app, &case("dedupe-fields"));
                        event(&mut app, Event::Run);
                        snapshot.settle(&mut app, App::view, &mut update);
                        scroll(&mut snapshot, &mut app, 40.0);
                    }
                    _ => {
                        fill(&mut app, &case("order-area"));
                        snapshot.settle(&mut app, App::view, &mut update);
                    }
                }
                let file = out.join(format!("islem-katman-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
