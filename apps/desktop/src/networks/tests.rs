//! Ağ analizi in the desktop (docs/adr/0209), on the network traces' drawing
//! (fixtures/interaction/v1/networks.kcad): Ağlar lists the project's
//! networks and checks the chosen one on the network thread, Kaydet writes a
//! changed one, En yakın tesis writes its ways in one step; and the pictures
//! for the owner (the web's `shots.mjs networks`).

use kentos_contracts::DocumentSnapshotV1;
use serde_json::json;

use super::Event as Networks;
use super::window::Event;
use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::drive;
use crate::processing::Event as Processing;
use crate::processing::dialog::RunStatus;

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/networks.kcad");

/// The app with the network traces' drawing open.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// A message and the tasks it starts, to their end (the network thread's answers come back as messages).
fn send(app: &mut App, message: Message) {
    let task = app.update(message);
    drive(app, task);
}

fn window(app: &mut App, e: Event) {
    send(app, Message::Networks(Networks::Window(e)));
}

fn processing(app: &mut App, e: Processing) {
    send(app, Message::Processing(e));
}

fn value(app: &mut App, name: &str, v: serde_json::Value) {
    processing(app, Processing::Value(name.into(), v));
}

fn layer(id: &str) -> serde_json::Value {
    json!({ "scope": "layer", "layerId": id })
}

/// Denetle's counts for the chosen network, as the window keeps them.
fn checked(app: &App) -> (usize, usize, f64) {
    let c = app
        .networks
        .window
        .as_ref()
        .and_then(|w| w.checked())
        .expect("Denetle answered");
    (c.nodes, c.pieces, c.length)
}

#[test]
fn the_window_lists_the_networks_and_checks_the_chosen_one_off_the_thread() {
    let mut app = app();
    send(&mut app, Message::Run("network.manage"));
    let names: Vec<String> = app.networks.window.as_ref().expect("Ağlar is open").names();
    assert_eq!(names, ["Yollar", "İçme suyu"]);
    window(&mut app, Event::Check);
    // The street grid: 12 crossings, 17 blocks, the curved one 123.175 m (scripts/fixtures/network_cases.py).
    let (nodes, pieces, length) = checked(&app);
    assert_eq!((nodes, pieces), (12, 17));
    assert!((length - 2_043.174_829_346_325_6).abs() < 1e-6, "{length}");
    window(&mut app, Event::Choose(1));
    window(&mut app, Event::Check);
    // The pipes: the depot, five valves and the two junctions; ten pieces, 526 m and the south branch's 54 m past V-5.
    let (nodes, pieces, length) = checked(&app);
    assert_eq!((nodes, pieces), (11, 10));
    assert!((length - 580.0).abs() < 1e-9, "{length}");
    // Denetle builds the window's definition under a key of its own: the project's network is not built for it.
    assert!(!app.networks.built_for("yollar"));
}

#[test]
fn kaydet_writes_the_changed_network_as_a_project_setting() {
    let mut app = app();
    send(&mut app, Message::Run("network.manage"));
    window(&mut app, Event::Name("Ana yollar".into()));
    window(&mut app, Event::Save);
    assert!(app.networks.window.is_none(), "Kaydet closes the window");
    let doc = &app.document.as_ref().expect("open").model;
    let names: Vec<&str> = doc
        .settings()
        .networks
        .iter()
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(names, ["Ana yollar", "İçme suyu"]);
    assert!(
        app.log
            .said(kentos_interaction::Level::Info, "Ağlar kaydedildi: 2 ağ.")
    );
    // A project setting, not an undo step (docs/adr/0209 §2).
    assert!(!doc.can_undo());
}

#[test]
fn en_yakin_tesis_writes_a_way_for_each_school_in_one_step() {
    let mut app = app();
    send(
        &mut app,
        Message::Run("processing.run.network.closestFacility"),
    );
    value(
        &mut app,
        "network",
        json!({ "network": "yollar", "cost": "Süre" }),
    );
    value(&mut app, "incidents", layer("okul"));
    value(&mut app, "facilities", layer("itfaiye"));
    value(&mut app, "count", json!(1));
    processing(&mut app, Processing::Run);
    let status = app.processing.dialog.as_ref().map(|w| w.status.clone());
    let Some(RunStatus::Ok { text, .. }) = status else {
        panic!("{status:?}");
    };
    assert_eq!(
        text,
        "3 olay için 3 tesis bulundu; 3 yol “Yollar” ağında Süre ile yazıldı."
    );
    let doc = &mut app.document.as_mut().expect("open").model;
    let written = doc
        .entities()
        .filter(|e| e.base().layer_id.starts_with("islem-en-yakin-tesis"))
        .count();
    assert_eq!(written, 3);
    assert_eq!(doc.undo().as_deref(), Some("En yakın tesis"));
}

/// Ağlar with Yollar and with İçme suyu after Denetle, En yakın tesis and
/// Maliyet matrisi after a run, two results in the drawing and the CBS
/// ribbon's Ağ panel; `.run/shots/aglar-*` (the web's `shots.mjs networks`).
/// Not run by default:
/// `cargo test -p kentos-desktop networks::tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "yollar",
        "sebeke",
        "en-yakin",
        "en-yakin-cizim",
        "matris",
        "alanlar-cizim",
        "serit",
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
                let mut app = app();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| send(app, message);
                snapshot.settle(&mut app, App::view, &mut update);
                send(&mut app, Message::Run("view.zoomExtents"));
                match name {
                    "yollar" | "sebeke" => {
                        send(&mut app, Message::Run("network.manage"));
                        if name == "sebeke" {
                            window(&mut app, Event::Choose(1));
                        }
                        window(&mut app, Event::Check);
                    }
                    "en-yakin" | "en-yakin-cizim" => {
                        send(
                            &mut app,
                            Message::Run("processing.run.network.closestFacility"),
                        );
                        value(
                            &mut app,
                            "network",
                            json!({ "network": "yollar", "cost": "Süre" }),
                        );
                        value(&mut app, "incidents", layer("okul"));
                        value(&mut app, "facilities", layer("itfaiye"));
                        value(&mut app, "count", json!(1));
                        processing(&mut app, Processing::Run);
                    }
                    "matris" => {
                        send(&mut app, Message::Run("processing.run.network.odMatrix"));
                        value(
                            &mut app,
                            "network",
                            json!({ "network": "yollar", "cost": "Süre" }),
                        );
                        value(&mut app, "origins", layer("okul"));
                        value(&mut app, "destinations", layer("itfaiye"));
                        processing(&mut app, Processing::Run);
                    }
                    "alanlar-cizim" => {
                        send(
                            &mut app,
                            Message::Run("processing.run.network.serviceAreas"),
                        );
                        value(
                            &mut app,
                            "network",
                            json!({ "network": "yollar", "cost": "Süre" }),
                        );
                        value(&mut app, "facilities", layer("okul"));
                        value(&mut app, "breaks", json!("0.2 0.4"));
                        value(&mut app, "trim", json!(20));
                        processing(&mut app, Processing::Run);
                    }
                    _ => {
                        // The CBS ribbon's Analiz: Ağlar and the network tools in their own panel.
                        let _ = app.update(Message::RibbonTab("analysis"));
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if name.ends_with("-cizim") {
                    // The window closed: the result in the drawing.
                    processing(&mut app, Processing::Close);
                    snapshot.settle(&mut app, App::view, &mut update);
                } else if matches!(name, "matris" | "yollar" | "sebeke") {
                    // The form scrolled to its end: the table, or Denetle's answer under the definition.
                    let x = if name == "matris" {
                        width / 2.0 - 120.0
                    } else {
                        width / 2.0 + 150.0
                    };
                    snapshot.step(
                        &mut app,
                        App::view,
                        &mut update,
                        &[
                            iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                                position: iced::Point::new(x, height / 2.0),
                            }),
                            iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                                delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: -40.0 },
                            }),
                        ],
                    );
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("aglar-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
