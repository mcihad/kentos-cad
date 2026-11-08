//! İş grafiği (project_graph_window.rs, docs/adr/1940) on the shared cases' map project of `road.project.graph`: the
//! zero line of Tümsek over Düz drawn and Pafta 1's map over it; one of Tümsek's heights set. The window, opened from
//! the project tree's panel, counts the stale drawing and the sheet that cannot be delivered, its row names the changed
//! source and the sheet, Güncel olmayanlar filters, Tarifi kaydet writes the graph; nothing is written to the drawing.
//! Drawn again, the chain is up to date. And its pictures.

use serde_json::{Value, json};

use crate::app::{App, Dialog, Message};
use crate::project_graph_window::{Event as G, Show, drawing_rows};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.project.graph.json")).expect("cases")
}

/// The cases' map project (with its sheets), the zero line drawn and Tümsek's corner set lower.
fn app() -> App {
    app_drawn().0
}

/// [`app`] and the drawing's revision right after the zero line was drawn (docs/adr/2840).
fn app_drawn() -> (App, String) {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(kentos_launch::Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::program_scenes::command(&mut app, "road.terrains.zeroline", json!({"base": "Düz", "comparison": "Tümsek", "layerId": "sifir"}));
    let drawn = app.document.as_ref().map(|d| d.model.revision().to_string()).unwrap_or_default();
    let surface = json!({"id": "t-10", "name": "Tümsek", "points": {"layers": ["tumsek"]},
                         "edits": [{"op": "setHeight", "x": 6000.0, "y": 0.0, "z": 119.0}]});
    crate::program_scenes::command(&mut app, "road.terrains.edit", json!({"operation": "set", "surface": surface}));
    (app, drawn)
}

#[test]
fn the_stale_zero_line_and_the_sheet_that_shows_it() {
    let (mut app, drawn) = app_drawn();
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::ProjectGraph(G::Open));
    assert_eq!(app.dialog, Some(Dialog::ProjectGraph));
    let g = app.project_graph.answer.as_ref().expect("an answer").as_ref().expect("read").clone();
    assert_eq!((g.drawings, g.outdated, g.sheets, g.blocked), (1, 1, 1, 1));
    assert_eq!((g.epsg.as_deref(), g.system.as_deref()), (Some("EPSG:5254"), Some("TUREF / TM30")));
    let rows = drawing_rows(&g, false);
    assert_eq!(
        rows,
        [vec![
            "Sıfır hattı · Sıfır hattı".to_owned(),
            "zeroline v1".to_owned(),
            "Düz, Tümsek (değişti)".to_owned(),
            "1/1".to_owned(),
            // Docs/adr/2840: the revision the zero line's write made, as its record keeps it.
            drawn,
            "Güncel değil".to_owned(),
            "Pafta 1 › Tümsek haritası".to_owned(),
        ]]
    );
    let _ = app.update(Message::ProjectGraph(G::Show(Show::Outdated)));
    assert_eq!(app.project_graph.show, Show::Outdated);
    // Tarifi kaydet: the graph as JSON, every recipe in it.
    let dir = std::env::temp_dir().join(format!("kentos-is-grafigi-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("is-grafigi.json");
    app.project_graph_save(file.clone());
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(&file).expect("written")).expect("JSON");
    let drawing = saved["nodes"].as_array().expect("nodes").iter().find(|n| n["kind"] == "drawing").expect("the drawing");
    assert_eq!(drawing["recipe"]["input"], json!({"base": "t-5", "comparison": "Tümsek", "layerId": "sifir"}));
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
    // Drawn again: the chain is up to date.
    let _ = app.update(Message::ProjectGraph(G::Close));
    crate::program_scenes::command(&mut app, "road.project.rebuild", json!({}));
    let rebuilt = app.document.as_ref().map(|d| d.model.revision().to_string()).unwrap_or_default();
    let _ = app.update(Message::ProjectGraph(G::Open));
    let g = app.project_graph.answer.as_ref().expect("an answer").as_ref().expect("read");
    assert_eq!((g.outdated, g.blocked), (0, 0));
    assert!(drawing_rows(g, true).is_empty(), "nothing out of date");
    assert_eq!(drawing_rows(g, false)[0][4], rebuilt, "drawn again: the rebuild's revision");
}

#[test]
fn a_gis_project_has_no_graph() {
    let mut app = app();
    if let Some(doc) = app.document.as_mut() {
        let mut settings = doc.model.settings().clone();
        settings.cad_work = None;
        settings.road = None;
        doc.model.set_settings(settings);
    }
    let _ = app.update(Message::ProjectGraph(G::Open));
    let answer = app.project_graph.answer.clone().expect("an answer");
    assert!(answer.is_err_and(|why| why.contains("yol ve harita projesinde")), "refused in words");
}

/// `.run/gui-evidence/is-grafigi-1940/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib project_graph_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/is-grafigi-1940");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(crate::app::Panel::RoadProject));
        app.docks.close(crate::app::Panel::Properties);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        // The panel with its İş grafiği… button.
        let _ = app.update(Message::RibbonTab("terrain"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-panel-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::ProjectGraph(G::Open));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-grafik-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/2840: İş grafiği with the Revizyon column, the zero line's record keeping the revision its write made;
/// `.run/gui-evidence/islem-surumu-2840/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib project_graph_window_tests::provenance_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn provenance_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/islem-surumu-2840");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::ProjectGraph(G::Open));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-grafik-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
