//! Buluttan arazi (terrain_cloud_window.rs, plan/road.md Y15.3): the window previews the cloud's ground points without
//! writing, takes an area from the selection, writes the points and the surface in one step; and its pictures.

use std::path::PathBuf;

use crate::app::{App, Dialog, Message};
use crate::terrain_cloud_window::Event as W;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// A road project with `road.terrains.cloud`'s drawing: a lidar cloud of a 40 × 30 m terrain, an area, a breakline.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let text = std::fs::read_to_string(fixtures().join("commands/v1/road.terrains.cloud.json")).expect("cases");
    let cases: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let whole = cases["setup"].to_string().replace("$fixture:", &format!("{}/", fixtures().display()));
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(&whole).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::TerrainCloud(e));
}

#[test]
fn the_window_previews_then_writes_the_ground_points() {
    let mut app = app();
    let _ = app.update(Message::Run("terrain.cloud"));
    assert_eq!(app.dialog, Some(Dialog::TerrainCloud));
    send(&mut app, W::Preview);
    let why = match &app.terrain_cloud_window.as_ref().expect("open").preview {
        Some(Err(why)) => why.clone(),
        other => panic!("no cell yet: {other:?}"),
    };
    assert!(why.contains("Hücre"), "{why}");
    send(&mut app, W::Field(0, "2".into()));
    send(&mut app, W::Layer("zemin".into()));
    send(&mut app, W::Preview);
    let why = match &app.terrain_cloud_window.as_ref().expect("open").preview {
        Some(Err(why)) => why.clone(),
        other => panic!("no class yet: {other:?}"),
    };
    assert!(why.contains("sınıf"), "{why}");
    send(&mut app, W::Class(2));
    send(&mut app, W::Preview);
    let o = match &app.terrain_cloud_window.as_ref().expect("open").preview {
        Some(Ok(o)) => o.clone(),
        other => panic!("previewed: {other:?}"),
    };
    assert_eq!((o.cloud.read, o.cloud.kept), (2232, 291));
    assert!(o.cloud.error_max.is_some());
    // The preview wrote nothing; a change says it is old.
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.settings().road.as_ref().map(|r| r.surfaces.len()), Some(1));
    let before = doc.entities().count();
    send(&mut app, W::Returns(kentos_commands::CloudReturns::Last));
    assert!(app.terrain_cloud_window.as_ref().expect("open").preview.is_none());
    send(&mut app, W::Preview);
    send(&mut app, W::Apply);
    assert!(app.terrain_cloud_window.is_none());
    let doc = &app.document.as_ref().expect("a drawing").model;
    let road = doc.settings().road.as_ref().expect("road");
    assert_eq!(road.surfaces.len(), 2);
    let s = &road.surfaces[1];
    assert_eq!(s.points.layers, vec!["zemin".to_owned()]);
    assert_eq!(s.clouds[0].returns, kentos_commands::CloudReturns::Last);
    assert_eq!(doc.entities().count(), before + 1);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.entities().count(), before);
}

#[test]
fn an_area_comes_from_the_selection() {
    let mut app = app();
    let slot = {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.slot_of("0192f5b0-7c3e-7d4a-9b1e-000000000006".parse().expect("a uid")).expect("the area")
    };
    app.selection.set([slot]);
    let _ = app.update(Message::Run("terrain.cloud"));
    let w = app.terrain_cloud_window.as_ref().expect("open");
    assert_eq!(w.area.as_ref().map(|a| a.1), Some(5));
    for (k, v) in [(0, "2")] {
        send(&mut app, W::Field(k, v.into()));
    }
    send(&mut app, W::Class(2));
    send(&mut app, W::Layer("zemin".into()));
    send(&mut app, W::Preview);
    let o = match &app.terrain_cloud_window.as_ref().expect("open").preview {
        Some(Ok(o)) => o.clone(),
        other => panic!("previewed: {other:?}"),
    };
    assert!(o.cloud.read < 2232 && o.cloud.area.is_some(), "{o:?}");
    send(&mut app, W::ClearArea);
    assert!(app.terrain_cloud_window.as_ref().expect("open").area.is_none());
}

/// The window, wide and narrow, light and dark, and the surface drawn; `.run/gui-evidence/yol-y15-bulut-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib terrain_cloud_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y15-bulut-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("terrain.cloud"));
        send(&mut app, W::Class(2));
        send(&mut app, W::Returns(kentos_commands::CloudReturns::Last));
        for (k, v) in [(0, "1.5"), (1, "0.15")] {
            send(&mut app, W::Field(k, v.into()));
        }
        send(&mut app, W::Layer("zemin".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "onizleme");
        send(&mut app, W::Class(3));
        send(&mut app, W::Class(6));
        send(&mut app, W::Preview);
        shot(&mut app, "uyari");
        send(&mut app, W::Class(3));
        send(&mut app, W::Class(6));
        send(&mut app, W::Preview);
        send(&mut app, W::Apply);
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "yuzey");
    }
}
