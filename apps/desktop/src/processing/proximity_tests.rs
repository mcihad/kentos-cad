//! The proximity tools in the desktop's window (docs/adr/0215), on the shared
//! proximity drawing (fixtures/processing/v1/proximity.kcad): Komşu alanlar's
//! table under the form and its counts written in one step, En yakını bul's
//! fields, and the pictures for the owner.

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Slot;
use serde_json::json;

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const PROXIMITY: &str = include_str!("../../../../fixtures/processing/v1/proximity.kcad");
const CASES: &str = include_str!("../../../../fixtures/processing/v1/proximity.json");

/// The app with the proximity drawing open.
fn app_with_proximity() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(PROXIMITY).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Processing(e));
}

fn value(app: &mut App, name: &str, v: serde_json::Value) {
    event(app, Event::Value(name.into(), v));
}

fn status(app: &App) -> RunStatus {
    app.processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

fn layer(id: &str) -> serde_json::Value {
    json!({ "scope": "layer", "layerId": id })
}

/// A shared case's values and what it expects (fixtures/processing/v1/proximity.json).
fn case(id: &str) -> serde_json::Value {
    let file: serde_json::Value = serde_json::from_str(CASES).expect("the cases read");
    file["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["id"] == id)
        .cloned()
        .expect("the case")
}

/// Komşu alanlar as the shared case runs it: the parcels, corners and overlaps too, the counts written.
fn neighbours(app: &mut App) {
    let _ = app.update(Message::Run("processing.run.proximity.neighbors"));
    value(app, "input", layer("parsel"));
    value(app, "corners", json!(true));
    value(app, "name", json!("Ad"));
    value(app, "write", json!(true));
}

/// En yakını bul for five parcels: the nearest stop's name and bearing (the shared case's `nearest-stops`).
fn nearest(app: &mut App) {
    app.selection.set((1..=5).map(Slot));
    let _ = app.update(Message::Run("processing.run.proximity.nearest"));
    value(app, "input", json!({ "scope": "selection" }));
    value(app, "targets", layer("durak"));
    value(app, "fields", json!("Ad"));
    value(app, "bearing", json!(true));
}

#[test]
fn the_neighbours_table_shows_under_the_form_and_the_counts_go_in_one_step() {
    let mut app = app_with_proximity();
    neighbours(&mut app);
    event(&mut app, Event::Run);
    let want = case("neighbours-parcels");
    let RunStatus::Ok {
        text, table, undo, ..
    } = status(&app)
    else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, want["expect"]["summary"]);
    assert!(undo, "the counts are written");
    let table = table.expect("the table");
    assert_eq!(
        json!({ "columns": table.columns, "rows": table.rows }),
        want["expect"]["outputs"]["table"]
    );
    let doc = &mut app.document.as_mut().expect("open").model;
    let p2 = doc.get(Slot(2)).expect("P2");
    assert_eq!(
        p2.base().attrs.get("Komşular").map(String::as_str),
        Some("P1, P3")
    );
    // One step takes every count back.
    assert_eq!(doc.undo().as_deref(), want["expect"]["undo"].as_str());
    assert!(
        doc.entities()
            .all(|e| !e.base().attrs.contains_key("Komşu sayısı"))
    );
}

#[test]
fn the_nearest_stop_is_written_as_the_shared_case_says() {
    let mut app = app_with_proximity();
    nearest(&mut app);
    event(&mut app, Event::Run);
    let want = case("nearest-stops");
    let RunStatus::Ok { text, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, want["expect"]["summary"]);
    let doc = &app.document.as_ref().expect("open").model;
    for u in want["expect"]["updated"].as_array().expect("updated") {
        let id = u["id"].as_u64().expect("an id") as u32;
        let attrs = &doc.get(Slot(id)).expect("the parcel").base().attrs;
        assert_eq!(json!(attrs), u["attrs"], "{id}");
    }
}

/// The proximity tools for the owner: CBS's Analiz tab with them in its
/// Analiz panel, En yakını bul's window after a run,
/// Uzaklık matrisi's matrix and Komşu alanlar's table under the form, and
/// the lines En yakın merkeze bağla and En kısa çizgi draw. Not run by default:
/// `cargo test -p kentos-desktop processing::proximity_tests::screens -- --ignored --nocapture`.
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
        "en-yakin",
        "matris",
        "komsular",
        "merkez-cizim",
        "kisa-cizgi-cizim",
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
                let mut app = app_with_proximity();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    // CBS's Analiz tab: the five in the Analiz panel, beside Özet istatistik.
                    "serit" => {
                        let _ = app.update(Message::RibbonTab("analysis"));
                    }
                    "en-yakin" => nearest(&mut app),
                    "matris" => {
                        let _ = app.update(Message::Run("processing.run.proximity.matrix"));
                        value(&mut app, "input", layer("parsel"));
                        value(&mut app, "targets", layer("durak"));
                        value(&mut app, "k", json!(0));
                        value(&mut app, "form", json!("matrix"));
                        value(&mut app, "name", json!("Ad"));
                        value(&mut app, "targetName", json!("Ad"));
                    }
                    "komsular" => neighbours(&mut app),
                    "merkez-cizim" => {
                        let _ = app.update(Message::Run("processing.run.proximity.hub"));
                        value(&mut app, "input", layer("parsel"));
                        value(&mut app, "hubs", layer("okul"));
                        value(&mut app, "hubName", json!("Ad"));
                    }
                    _ => {
                        let _ = app.update(Message::Run("processing.run.proximity.shortestLine"));
                        value(&mut app, "input", layer("parsel"));
                        value(&mut app, "targets", layer("yol"));
                        value(&mut app, "name", json!("Ad"));
                        value(&mut app, "targetName", json!("Ad"));
                    }
                }
                if name != "serit" {
                    event(&mut app, Event::Run);
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if name.ends_with("-cizim") {
                    // The window closed, the view on the whole drawing.
                    event(&mut app, Event::Close);
                    let _ = app.update(Message::Run("view.zoomExtents"));
                    snapshot.settle(&mut app, App::view, &mut update);
                } else if matches!(name, "matris" | "komsular") {
                    // The form scrolled to its end: the table.
                    snapshot.step(
                        &mut app,
                        App::view,
                        &mut update,
                        &[
                            iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                                position: iced::Point::new(width / 2.0 - 120.0, height / 2.0),
                            }),
                            iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                                delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: -40.0 },
                            }),
                        ],
                    );
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!(
                    "islem-yakinlik-{name}-{width}x{height}{suffix}.png"
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
