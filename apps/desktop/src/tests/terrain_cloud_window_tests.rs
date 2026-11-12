//! Buluttan arazi (terrain_cloud_window.rs, plan/road.md Y15.3): the window previews the cloud's ground points without
//! writing, takes an area from the selection, writes the points and the surface in one step; and its pictures.

use crate::road;
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
    let _ = app.update(Message::Road(road::Event::TerrainCloud(e)));
}

#[test]
fn the_window_previews_then_writes_the_ground_points() {
    let mut app = app();
    let _ = app.update(Message::Run("terrain.cloud"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::TerrainCloud))
    );
    send(&mut app, W::Preview);
    let why = match &app
        .road
        .terrain_cloud_window
        .as_ref()
        .expect("open")
        .preview
    {
        Some(Err(why)) => why.clone(),
        other => panic!("no cell yet: {other:?}"),
    };
    assert!(why.contains("Hücre"), "{why}");
    send(&mut app, W::Field(0, "2".into()));
    send(&mut app, W::Layer("zemin".into()));
    send(&mut app, W::Preview);
    let why = match &app
        .road
        .terrain_cloud_window
        .as_ref()
        .expect("open")
        .preview
    {
        Some(Err(why)) => why.clone(),
        other => panic!("no class yet: {other:?}"),
    };
    assert!(why.contains("sınıf"), "{why}");
    send(&mut app, W::Class(2));
    send(&mut app, W::Preview);
    let o = match &app
        .road
        .terrain_cloud_window
        .as_ref()
        .expect("open")
        .preview
    {
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
    assert!(
        app.road
            .terrain_cloud_window
            .as_ref()
            .expect("open")
            .preview
            .is_none()
    );
    send(&mut app, W::Preview);
    send(&mut app, W::Apply);
    assert!(app.road.terrain_cloud_window.is_none());
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
    let w = app.road.terrain_cloud_window.as_ref().expect("open");
    assert_eq!(w.area.as_ref().map(|a| a.1), Some(5));
    send(&mut app, W::Field(0, "2".into()));
    send(&mut app, W::Class(2));
    send(&mut app, W::Layer("zemin".into()));
    send(&mut app, W::Preview);
    let o = match &app
        .road
        .terrain_cloud_window
        .as_ref()
        .expect("open")
        .preview
    {
        Some(Ok(o)) => o.clone(),
        other => panic!("previewed: {other:?}"),
    };
    assert!(o.cloud.read < 2232 && o.cloud.area.is_some(), "{o:?}");
    send(&mut app, W::ClearArea);
    assert!(
        app.road
            .terrain_cloud_window
            .as_ref()
            .expect("open")
            .area
            .is_none()
    );
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

/// `road.terrains.cloud`'s case drawing (docs/adr/1032: a road project in TUREF / TM36 whose cloud's heights are
/// ellipsoidal), the synthetic TG geoid read into the core as a device's grid library gives it.
fn tm36_app() -> (App, serde_json::Value) {
    let (mut app, _) = App::boot(None);
    let text = std::fs::read_to_string(fixtures().join("commands/v1/road.terrains.cloud.json")).expect("cases");
    let cases: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Elipsoit yükseklikli bulut geoitle projenin kotlarına")
        .expect("the case")
        .clone();
    let whole = case["setup"].to_string().replace("$fixture:", &format!("{}/", fixtures().display()));
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(&whole).expect("the drawing reads");
    let bytes = std::fs::read(fixtures().join("geoid/v1/tg.gtx")).expect("the geoid");
    let model = kentos_native_application::crs_models::read(&bytes).expect("reads");
    kentos_geometry_core::crs::geoid::register(&kentos_native_application::crs_models::sha256(&bytes), model.grid);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    (app, case)
}

/// ROAD-NUM-03 (docs/adr/1032): Bulutun kotları › Elipsoit yüksekliği turns the kept points' heights into the
/// project's by its geoid, as the shared case does; the preview says so. The file's own heights are taken unturned.
#[test]
fn the_window_turns_ellipsoidal_heights_by_the_geoid() {
    let (mut app, case) = tm36_app();
    let want = &case["steps"][0]["result"]["output"]["surface"]["figures"];
    let _ = app.update(Message::Run("terrain.cloud"));
    send(&mut app, W::Field(0, "2".into()));
    send(&mut app, W::Layer("zemin".into()));
    send(&mut app, W::Class(2));
    send(&mut app, W::Field(3, "Lidar zemini".into()));
    let m = app
        .terrain_cloud_control(crate::traces::Control::Pick(crate::terrain_cloud_window::HEIGHTS, "Elipsoit yüksekliği (geoitle çevir)"))
        .expect("a control")
        .expect("a message");
    let _ = app.update(m);
    send(&mut app, W::Preview);
    let o = match &app
        .road
        .terrain_cloud_window
        .as_ref()
        .expect("open")
        .preview
    {
        Some(Ok(o)) => o.clone(),
        other => panic!("previewed: {other:?}"),
    };
    let f = o.surface.figures.clone().expect("figures");
    assert!((f.min_z.expect("a lowest height") - want["minZ"].as_f64().unwrap()).abs() < 1e-6, "{f:?}");
    assert!(o.cloud.heights.as_ref().is_some_and(|h| h.from.kind == kentos_commands::HeightKind::Ellipsoidal));
    send(&mut app, W::Heights(0));
    send(&mut app, W::Preview);
    let o = match &app
        .road
        .terrain_cloud_window
        .as_ref()
        .expect("open")
        .preview
    {
        Some(Ok(o)) => o.clone(),
        other => panic!("previewed: {other:?}"),
    };
    assert!(
        o.cloud.heights.is_none() && o.surface.figures.and_then(|f| f.min_z).is_some_and(|z| z > 100.0),
        "the file's own heights"
    );
}

/// The window turning the heights, light; `.run/gui-evidence/yol-y97/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib terrain_cloud_window_tests::screens_heights -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_heights() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y97");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 760.0), "koyu")] {
        let (mut app, _) = tm36_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.cloud"));
        send(&mut app, W::Field(0, "2".into()));
        send(&mut app, W::Layer("zemin".into()));
        send(&mut app, W::Class(2));
        send(&mut app, W::Field(3, "Lidar zemini".into()));
        send(&mut app, W::Heights(1));
        send(&mut app, W::Preview);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y97-bulut-kotlari-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
