//! The Dönüştürme tools in the desktop's window (docs/adr/0218), on the
//! shared drawing (fixtures/processing/v1/convert.kcad): every shared case
//! run from the window as a user fills it (a table chosen as the dialog reads
//! it), and the pictures for the owner.

use std::path::PathBuf;

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Slot;
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/processing/v1/convert.kcad");
const CASES: &str = include_str!("../../../../fixtures/processing/v1/convert.json");
const SEMT: &[u8] = include_bytes!("../../../../fixtures/processing/v1/convert-semt.csv");
const XY: &[u8] = include_bytes!("../../../../fixtures/processing/v1/convert-xy.csv");
const ELIPS: &[u8] = include_bytes!("../../../../fixtures/processing/v1/convert-elips.csv");

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

/// A case's tool opened and filled as a user does: its selection made first,
/// the kind of source before the file, a file chosen as the dialog reads it.
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
    if let Some(kind) = values.get("sourceKind") {
        event(app, Event::Value("sourceKind".into(), kind.clone()));
    }
    for (name, v) in values {
        if name == "sourceKind" {
            continue;
        }
        if name == "file" {
            let file = v.as_str().expect("a file's name");
            let bytes = match file {
                "convert-semt.csv" => SEMT,
                "convert-xy.csv" => XY,
                _ => ELIPS,
            };
            let path = PathBuf::from("/veri").join(file);
            event(
                app,
                Event::FileChosen(name.clone(), Some((file.to_owned(), path, bytes.to_vec()))),
            );
        } else {
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
        let RunStatus::Ok { text, table, .. } = status(&app) else {
            panic!("{id}: {:?}", status(&app));
        };
        assert_eq!(text, want["summary"], "{id}");
        if let Some(t) = want["outputs"].get("table") {
            let table = table.expect("the report");
            assert_eq!(
                json!({ "columns": table.columns, "rows": table.rows }),
                *t,
                "{id}"
            );
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
    assert!(seen >= 23, "{seen} cases");
}

/// The Dönüştürme tools for the owner: CBS's Veri tab with Toplu çevir,
/// Nesneden nokta's interior points on the parcels (the L's centroid outside
/// it, its interior point inside), Semt ve uzunluktan çizgi's window with
/// its report, the ellipses and the bearing lines drawn, Çizginin parçası's
/// window with its distances as expressions, Köşe ekle on the arc. Not run
/// by default:
/// `cargo test -p kentos-desktop processing::convert_tests::screens -- --ignored --nocapture --test-threads=1`.
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
        "nokta",
        "nokta-cizim",
        "semt",
        "tablo-cizim",
        "parca",
        "sik-cizim",
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
                // Runs a case and closes the window: the result on the drawing, the view on it all.
                let drawn = |app: &mut App, ids: &[&str]| {
                    for id in ids {
                        fill(app, &case(id));
                        event(app, Event::Run);
                        event(app, Event::Close);
                    }
                    let _ = app.update(Message::Run("view.zoomExtents"));
                };
                match name {
                    // CBS's Veri tab: Toplu çevir beside Öznitelik.
                    "serit" => {
                        let _ = app.update(Message::RibbonTab("data"));
                    }
                    "nokta" => {
                        fill(&mut app, &case("points-inside"));
                        event(&mut app, Event::Run);
                    }
                    "nokta-cizim" => drawn(&mut app, &["points-centroid", "points-inside"]),
                    "semt" => {
                        fill(&mut app, &case("bearing-file"));
                        event(&mut app, Event::Run);
                        snapshot.settle(&mut app, App::view, &mut update);
                        scroll(&mut snapshot, &mut app, 40.0);
                    }
                    "tablo-cizim" => drawn(&mut app, &["bearing-file", "xy-file", "ellipse-file"]),
                    "parca" => fill(&mut app, &case("substring-expr")),
                    // The densified objects selected: their grips are their vertices, the added ones among them.
                    _ => {
                        drawn(&mut app, &["densify-interval"]);
                        let doc = &app.document.as_ref().expect("open").model;
                        let ids: Vec<Slot> = doc
                            .entities()
                            .filter(|e| e.base().layer_id == "islem-siklastirilan")
                            .map(|e| Slot(e.base().id))
                            .collect();
                        app.selection.set(ids);
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!(
                    "islem-donustur-{name}-{width}x{height}{suffix}.png"
                ));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
