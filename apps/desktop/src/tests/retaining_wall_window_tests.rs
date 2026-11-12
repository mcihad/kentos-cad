//! İstinat duvarı (retaining_wall_window.rs, plan/road.md Y11.3): the window reads the wall as it is typed, saves it and
//! draws it, each one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::retaining_wall_window::Event as W;
use crate::road;

/// A road project with `road.walls.edit`'s drawing: corridors and two walls.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.walls.edit.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::RetainingWalls(e)));
}

#[test]
fn the_window_saves_and_draws_the_wall() {
    let mut app = app();
    let _ = app.update(Message::Run("retainingWall.editor"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::RetainingWalls))
    );
    let w = app.road.retaining_wall_window.as_ref().expect("open");
    assert_eq!(w.chosen.as_deref(), Some("d-1"));
    let r = w
        .preview
        .report
        .clone()
        .unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert_eq!(r.rows.len(), 5);
    // A wider footing makes more concrete.
    send(&mut app, W::Field(8, "3.2".into()));
    let more = app
        .road
        .retaining_wall_window
        .as_ref()
        .and_then(|w| w.preview.report.clone())
        .expect("reads");
    assert!(
        more.concrete > r.concrete,
        "{} → {}",
        r.concrete,
        more.concrete
    );
    send(&mut app, W::Field(10, "3".into()));
    let why = app
        .road
        .retaining_wall_window
        .as_ref()
        .and_then(|w| w.preview.why.clone())
        .expect("refused");
    assert!(why.contains("temelin"), "{why}");
    send(&mut app, W::Field(10, "0.6".into()));
    send(&mut app, W::Save);
    let saved = app.document.as_ref().expect("a drawing").model.road_walls()[0].clone();
    assert_eq!(saved.shape.footing, 3.2);
    assert_eq!(saved.revision, 2);
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::Layer("korkuluk".into()));
    send(&mut app, W::Draw);
    assert!(app.road.retaining_wall_window.is_none());
    let after = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .count();
    assert_eq!(after, before + 3);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.entities().count(), before);
}

#[test]
fn a_new_wall_needs_its_stations_and_cross() {
    let mut app = app();
    let _ = app.update(Message::Run("retainingWall.editor"));
    send(&mut app, W::Choose(None));
    let why = app
        .road
        .retaining_wall_window
        .as_ref()
        .and_then(|w| w.preview.why.clone())
        .expect("no stations yet");
    assert!(why.contains("yazılmalı"), "{why}");
    for (k, v) in [
        (1, "360"),
        (2, "400"),
        (3, "10"),
        (6, "0.3"),
        (8, "2.4"),
        (9, "0.5"),
    ] {
        send(&mut app, W::Field(k, v.into()));
    }
    let w = app.road.retaining_wall_window.as_ref().expect("open");
    assert!(w.preview.report.is_some(), "{:?}", w.preview.why);
    send(&mut app, W::Save);
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .road_walls()
            .len(),
        3
    );
}

/// The window, wide and narrow, light and dark, and the wall drawn in plan; `.run/gui-evidence/yol-y11-istinat-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib retaining_wall_window_tests::screens -- --ignored --nocapture
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
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y11-istinat-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("retainingWall.editor"));
        shot(&mut app, "dolgu");
        send(&mut app, W::Choose(Some("d-2".into())));
        shot(&mut app, "yarma");
        send(&mut app, W::Choose(Some("d-1".into())));
        send(&mut app, W::Layer("korkuluk".into()));
        send(&mut app, W::Draw);
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "plan");
    }
}
