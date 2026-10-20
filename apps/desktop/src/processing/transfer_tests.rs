//! Veri aktar in the desktop's window (docs/adr/0216), on the shared
//! transfer drawing (fixtures/processing/v1/transfer.kcad): every shared case
//! run from the window as a user fills it (a file chosen as the dialog reads
//! it), Alan eşleme's check under the field, and the pictures for the owner.

use std::path::PathBuf;

use kentos_commands::DocumentSnapshotV1;
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const TRANSFER: &str = include_str!("../../../../fixtures/processing/v1/transfer.kcad");
const CASES: &str = include_str!("../../../../fixtures/processing/v1/transfer.json");
const DEEDS: &[u8] = include_bytes!("../../../../fixtures/processing/v1/tapu.csv");
const POINTS: &[u8] = include_bytes!("../../../../fixtures/processing/v1/noktalar.csv");

/// The app with the transfer drawing open.
fn app_with_transfer() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(TRANSFER).expect("the drawing reads");
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

/// Veri aktar opened and filled with a case's values as a user does: the
/// file chosen (its bytes read as the dialog reads them), the rest typed or picked.
fn fill(app: &mut App, values: &Value) {
    let _ = app.update(Message::Run("processing.run.attributes.transfer"));
    let order = [
        "target",
        "targetKey",
        "sourceKind",
        "file",
        "source",
        "sourceKey",
        "map",
        "existing",
        "duplicates",
        "geometry",
        "append",
        "east",
        "north",
        "reportOnly",
    ];
    for name in order {
        let Some(v) = values.get(name) else {
            continue;
        };
        if name == "file" {
            let file = v.as_str().expect("a file's name");
            let bytes = match file {
                "tapu.csv" => DEEDS,
                _ => POINTS,
            };
            let path = PathBuf::from("/veri").join(file);
            event(
                app,
                Event::FileChosen(name.into(), Some((file.to_owned(), path, bytes.to_vec()))),
            );
        } else {
            event(app, Event::Value(name.into(), v.clone()));
        }
    }
}

#[test]
fn every_shared_case_runs_from_the_window_as_it_says() {
    let mut seen = 0;
    for c in cases() {
        let mut app = app_with_transfer();
        fill(&mut app, &c["values"]);
        event(&mut app, Event::Run);
        let want = &c["expect"];
        let id = c["id"].as_str().unwrap_or("?");
        match (want["status"].as_str(), status(&app)) {
            (Some("ok"), RunStatus::Ok { text, table, .. }) => {
                assert_eq!(text, want["summary"], "{id}");
                let table = table.expect("the report");
                assert_eq!(
                    json!({ "columns": table.columns, "rows": table.rows }),
                    want["outputs"]["table"],
                    "{id}"
                );
                let doc = &app.document.as_ref().expect("open").model;
                for u in want["updated"].as_array().into_iter().flatten() {
                    let slot = kentos_domain::Slot(u["id"].as_u64().expect("an id") as u32);
                    let attrs = &doc.get(slot).expect("the object").base().attrs;
                    assert_eq!(json!(attrs), u["attrs"], "{id}: {slot:?}");
                }
            }
            // The window's check keeps it from starting: the reason under the field.
            (Some("invalid"), RunStatus::Idle | RunStatus::Invalid(_)) => {
                let issue = app
                    .processing
                    .dialog
                    .as_ref()
                    .and_then(|w| w.issue_of("target"))
                    .map(|i| i.message.clone());
                assert_eq!(
                    json!(issue),
                    want["issues"][0]["message"],
                    "{id}: under the field"
                );
            }
            (Some("error"), RunStatus::Error(text)) => assert_eq!(text, want["message"], "{id}"),
            (_, got) => panic!("{id}: {got:?}"),
        }
        seen += 1;
    }
    assert!(seen >= 10, "{seen} cases");
}

#[test]
fn a_map_row_without_a_target_keeps_the_run_from_starting() {
    let mut app = app_with_transfer();
    let mut values = case("deed-mapped")["values"].clone();
    values["map"] = json!([
        { "target": "Malik", "source": "MALIK_ADI", "default": "" },
        { "target": " ", "source": "HISSE", "default": "" },
    ]);
    fill(&mut app, &values);
    event(&mut app, Event::Run);
    assert!(
        matches!(status(&app), RunStatus::Idle),
        "{:?}",
        status(&app)
    );
    let issue = app
        .processing
        .dialog
        .as_ref()
        .and_then(|w| w.issue_of("map"))
        .map(|i| i.message.clone());
    assert_eq!(
        issue.as_deref(),
        Some("“Alan eşleme”: 2. satırın hedef alanı boş.")
    );
    // Nothing changed.
    let doc = &app.document.as_ref().expect("open").model;
    assert!(!doc.can_undo());
}

/// Veri aktar for the owner: CBS's Veri tab with it in its Öznitelik
/// list, the deed list's window with its field map (and a map with
/// problems), the report under the form after a run, and the new survey
/// moved and added on the drawing. Not run by default:
/// `cargo test -p kentos-desktop processing::transfer_tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = ["serit", "tapu", "eslem-sorun", "rapor", "olcum-cizim"];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in views {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let mut app = app_with_transfer();
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
                    // CBS's Veri tab: Öznitelik's list with Veri aktar beside Öznitelik hesapla.
                    "serit" => {
                        let _ = app.update(Message::RibbonTab("data"));
                        snapshot.settle(&mut app, App::view, &mut update);
                    }
                    "tapu" => {
                        fill(&mut app, &case("deed-mapped")["values"]);
                        snapshot.settle(&mut app, App::view, &mut update);
                        scroll(&mut snapshot, &mut app, 6.0);
                    }
                    "eslem-sorun" => {
                        let mut values = case("deed-mapped")["values"].clone();
                        values["map"] = json!([
                            { "target": "Malik", "source": "MALIK_ADI", "default": "" },
                            { "target": "Malik", "source": "HISSE", "default": "" },
                            { "target": "", "source": "YUZOLCUM", "default": "" },
                            { "target": "Durum", "source": "DURUMU", "default": "Tescilli" },
                        ]);
                        fill(&mut app, &values);
                        event(&mut app, Event::Run);
                        snapshot.settle(&mut app, App::view, &mut update);
                        scroll(&mut snapshot, &mut app, 6.0);
                    }
                    "rapor" => {
                        fill(&mut app, &case("deed-mapped")["values"]);
                        event(&mut app, Event::Run);
                        snapshot.settle(&mut app, App::view, &mut update);
                        scroll(&mut snapshot, &mut app, 40.0);
                    }
                    _ => {
                        fill(&mut app, &case("survey-geometry-append")["values"]);
                        event(&mut app, Event::Run);
                        snapshot.settle(&mut app, App::view, &mut update);
                        // The window closed, the view on the whole drawing.
                        event(&mut app, Event::Close);
                        let _ = app.update(Message::Run("view.zoomExtents"));
                        snapshot.settle(&mut app, App::view, &mut update);
                    }
                }
                let file = out.join(format!("islem-aktar-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
