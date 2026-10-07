//! The geometry tools in the desktop's window (docs/adr/0201), on the shared
//! geometry drawing (fixtures/processing/v1/geometry.kcad): Tampon's rings on
//! a layer of their own in one step, Geçerliliği denetle's table and
//! selection, Koordinat sistemine dönüştür's refusal of the project's own
//! system, and the pictures for the owner.

use kentos_contracts::DocumentSnapshotV1;
use serde_json::json;

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const GEOMETRY: &str = include_str!("../../../../fixtures/processing/v1/geometry.kcad");

/// The app with the geometry drawing open.
fn app_with_geometry() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(GEOMETRY).expect("the drawing reads");
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

#[test]
fn buffer_rings_go_to_a_layer_of_their_own_in_one_step() {
    let mut app = app_with_geometry();
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    value(&mut app, "input", layer("agac"));
    value(&mut app, "distance", json!(2));
    value(&mut app, "rings", json!(2));
    value(&mut app, "dissolve", json!(true));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "3 nesnenin tamponu birleştirilerek yazıldı (2 alan).");
    let doc = &mut app.document.as_mut().expect("open").model;
    let layer = doc
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == "Tampon")
        .map(|l| l.id.clone())
        .expect("the new layer");
    assert_eq!(
        doc.entities()
            .filter(|e| e.base().layer_id == layer)
            .count(),
        2
    );
    assert_eq!(doc.undo().as_deref(), Some("Tampon"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "Tampon"));
}

#[test]
fn the_validity_check_shows_its_table_selects_and_changes_nothing() {
    let mut app = app_with_geometry();
    let _ = app.update(Message::Run("processing.run.geometry.validity"));
    value(&mut app, "input", layer("hatali"));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, table, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(
        text,
        "6 nesneden 5 nesnede 5 sorun bulundu; sorunlu nesneler seçildi."
    );
    let table = table.expect("the table");
    assert_eq!(table.columns, ["Nesne", "Katman", "Sorun", "Doğu", "Kuzey"]);
    assert_eq!(
        table.rows[0],
        [
            "#12",
            "Hatalı",
            "Kendini kesen halka",
            "487075.000",
            "4420005.000"
        ]
    );
    let selected: Vec<u32> = app.selection.ids().iter().map(|s| s.0).collect();
    assert_eq!(selected, [12, 13, 14, 15, 16]);
    assert!(!app.document.as_ref().expect("open").model.can_undo());
}

#[test]
fn reprojecting_from_the_projects_own_system_is_refused() {
    let mut app = app_with_geometry();
    let _ = app.update(Message::Run("processing.run.geometry.reproject"));
    value(&mut app, "input", layer("eski"));
    value(&mut app, "source", json!("5254"));
    event(&mut app, Event::Run);
    assert_eq!(
        status(&app),
        RunStatus::Error(
            "Kaynak sistem projenin sistemiyle aynı; dönüştürülecek bir şey yok.".into()
        )
    );
    assert!(!app.document.as_ref().expect("open").model.can_undo());
}

/// The geometry tools' windows for the owner, after a run, and two results in
/// the drawing: Tampon (rings around the roads), Kesişim, Geçerliliği denetle
/// with its table, Onar with its report, Sadeleştir, Koordinat sistemine
/// dönüştür with its systems' list open. Not run by default:
/// `cargo test -p kentos-desktop processing::geometry_tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "tampon",
        "tampon-cizim",
        "kesisim",
        "kesisim-cizim",
        "gecerlilik",
        "onar",
        "sadelestir",
        "donustur",
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
                let mut app = app_with_geometry();
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
                    "tampon" | "tampon-cizim" => {
                        let _ = app.update(Message::Run("processing.run.geometry.buffer"));
                        value(&mut app, "input", layer("yol"));
                        value(&mut app, "distance", json!(3));
                        value(&mut app, "rings", json!(2));
                        value(&mut app, "dissolve", json!(true));
                        event(&mut app, Event::Run);
                    }
                    "kesisim" | "kesisim-cizim" => {
                        let _ = app.update(Message::Run("processing.run.geometry.intersection"));
                        value(&mut app, "input", layer("parsel"));
                        value(&mut app, "overlay", layer("imar"));
                        value(&mut app, "prefix", json!("İmar "));
                        value(&mut app, "apportion", json!("Değer"));
                        event(&mut app, Event::Run);
                    }
                    "gecerlilik" => {
                        let _ = app.update(Message::Run("processing.run.geometry.validity"));
                        value(&mut app, "input", layer("hatali"));
                        event(&mut app, Event::Run);
                    }
                    "onar" => {
                        let _ = app.update(Message::Run("processing.run.geometry.repair"));
                        value(&mut app, "input", layer("hatali"));
                        event(&mut app, Event::Run);
                    }
                    "sadelestir" => {
                        let _ = app.update(Message::Run("processing.run.geometry.simplify"));
                        value(&mut app, "input", layer("sinir"));
                        value(&mut app, "tolerance", json!(0.05));
                        event(&mut app, Event::Run);
                    }
                    _ => {
                        let _ = app.update(Message::Run("processing.run.geometry.reproject"));
                        value(&mut app, "input", layer("eski"));
                        value(&mut app, "source", json!("2320"));
                        event(&mut app, Event::Run);
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if name.ends_with("-cizim") {
                    // The window closed, the view on everything but the old sheet (17 km away).
                    event(&mut app, Event::Close);
                    let near: Vec<kentos_domain::Slot> = app
                        .document
                        .as_ref()
                        .expect("open")
                        .model
                        .entities()
                        .filter(|e| e.base().layer_id != "eski")
                        .map(|e| kentos_domain::Slot(e.base().id))
                        .collect();
                    app.selection.set(near);
                    let _ = app.update(Message::Run("view.zoomSelection"));
                    app.selection.clear();
                    // Kesişim's pieces lie on the zoning areas: the inputs hidden, the pieces show.
                    if name == "kesisim-cizim" {
                        let doc = &mut app.document.as_mut().expect("open").model;
                        doc.set_layer_visible("parsel", false);
                        doc.set_layer_visible("imar", false);
                    }
                    snapshot.settle(&mut app, App::view, &mut update);
                } else if matches!(name, "gecerlilik" | "onar" | "sadelestir") {
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
                    "islem-geometri-{name}-{width}x{height}{suffix}.png"
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
