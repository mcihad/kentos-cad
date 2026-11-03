//! Arazi yüzeyleri (terrain_window.rs, plan/road.md Y2.4): the window writes through `road.terrains`, and its pictures.

use kentos_domain::Slot;

use crate::app::{App, Dialog, Message};
use crate::terrain_window::Event as W;

/// The terrain commands' shared drawing (`fixtures/commands/v1/road.terrains.query.json`'s setup) with no surface.
fn app(work: &str) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    setup["settings"].as_object_mut().expect("settings").remove("road");
    setup["settings"]["cadWork"] = serde_json::json!(work);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Terrains(e));
}

/// The boundary of the shared drawing: its tenth object.
const BOUNDARY: Slot = Slot(10);
const VOID: Slot = Slot(11);

fn made(app: &mut App) {
    let _ = app.update(Message::Run("terrain.new"));
    assert_eq!(app.dialog, Some(Dialog::Terrains));
    send(app, W::PointLayer("zemin".into()));
    send(app, W::BreaklineLayer("kirik".into()));
    app.selection.set([BOUNDARY]);
    send(app, W::Boundary);
    app.selection.set([VOID]);
    send(app, W::Voids);
    send(app, W::MaxEdge("60".into()));
    send(app, W::Save);
}

#[test]
fn the_window_makes_a_surface_and_accepts_it_again() {
    let mut app = app("road");
    made(&mut app);
    assert!(app.terrain_window.is_none(), "Kaydet closes the window");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let road = doc.settings().road.clone().expect("road settings");
    assert_eq!(road.surfaces.len(), 1);
    let s = &road.surfaces[0];
    assert_eq!((s.id.as_str(), s.name.as_str()), ("t-1", "Mevcut zemin"));
    assert_eq!(s.points.layers, ["zemin"]);
    assert_eq!(s.voids.len(), 1);
    assert_eq!(s.max_edge, Some(60.0));
    assert!(s.built.is_some(), "a new surface is accepted");
    // A changed edge method makes it stale; Kabul et accepts it as its sources give it now.
    let _ = app.update(Message::Run("terrain.surfaces"));
    send(&mut app, W::MaxEdge("25".into()));
    send(&mut app, W::Save);
    let made = kentos_native_application::road_terrains::made_surfaces(&app.document.as_ref().expect("doc").model);
    assert_eq!(made[0].1.state, kentos_commands::TerrainState::Stale);
    let _ = app.update(Message::Run("terrain.surfaces"));
    send(&mut app, W::Accept);
    let made = kentos_native_application::road_terrains::made_surfaces(&app.document.as_ref().expect("doc").model);
    assert_eq!(made[0].1.state, kentos_commands::TerrainState::Current);
    // Eğrileri çiz: the saved surface's contour lines on a layer, one undo step.
    let before = app.document.as_ref().expect("doc").model.entities().count();
    send(&mut app, W::Interval("0,5".into()));
    send(&mut app, W::ContourLayer("esyukselti".into()));
    send(&mut app, W::Contours);
    let doc = &app.document.as_ref().expect("doc").model;
    assert!(doc.entities().count() > before, "the lines are drawn");
    assert!(doc.can_undo());
    // Removed and saved: the surface goes; the contour lines' record stays, its source gone (docs/adr/0911 §2), so the
    // Yol projesi panel says “Kaynağı yok”.
    send(&mut app, W::Remove);
    send(&mut app, W::Save);
    let road = app.document.as_ref().expect("doc").model.settings().road.clone().expect("the drawing's record");
    assert!(road.surfaces.is_empty());
    let tree = kentos_native_application::road_project_query::preview(&app.document.as_ref().expect("doc").model, false)
        .expect("a road project");
    assert!(tree.nodes.iter().any(|n| n.kind == kentos_commands::RoadNodeKind::Drawing && n.state == kentos_commands::RoadNodeState::Gone));
}

#[test]
fn landxml_comes_in_as_a_surface() {
    let mut app = app("road");
    let _ = app.update(Message::Run("terrain.surfaces"));
    send(&mut app, W::ContourLayer("esyukselti".into()));
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/landxml/v1/kirikli.xml");
    send(&mut app, W::LandXmlChosen(Some(path)));
    let doc = &app.document.as_ref().expect("doc").model;
    let road = doc.settings().road.clone().expect("road settings");
    assert_eq!(road.surfaces[0].name, "Zemin");
    assert!(road.surfaces[0].boundary.is_some() && road.surfaces[0].voids.len() == 1);
    assert!(doc.can_undo(), "the objects are one undo step");
}

#[test]
fn a_map_project_has_no_surfaces() {
    let mut app = app("map");
    let _ = app.update(Message::Run("terrain.surfaces"));
    assert!(app.terrain_window.is_none());
    assert_ne!(app.dialog, Some(Dialog::Terrains));
}

/// The window and the surface over the drawing, wide and narrow, light and dark; `.run/gui-evidence/y2-4-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib terrain_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app("road");
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        snapshot.settle(&mut app, App::view, &mut update);
        made(&mut app);
        app.selection.clear();
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y2-4-yuzey-cizimde-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::Run("terrain.surfaces"));
        send(&mut app, W::Interval("0,25".into()));
        send(&mut app, W::ContourLayer("esyukselti".into()));
        send(&mut app, W::Contours);
        let file = out.join(format!("y2-5-esyukselti-penceresi-{name}.png"));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        send(&mut app, W::Cancel);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y2-5-esyukselti-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::Run("terrain.surfaces"));
        send(&mut app, W::MaxEdge("25".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y2-4-yuzey-penceresi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        // LandXML al: a file's surface beside the drawing's, on its own layer.
        send(&mut app, W::Cancel);
        let _ = app.update(Message::Run("terrain.surfaces"));
        send(&mut app, W::ContourLayer("0".into()));
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/landxml/v1/kirikli.xml");
        send(&mut app, W::LandXmlChosen(Some(path)));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y2-5-landxml-penceresi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
