//! Kavşak (junction_window.rs, plan/road.md Y12.2): the window reads the junction as it is typed, saves it and draws its
//! curb returns, each one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::junction_window::Event as W;
use kentos_commands::JunctionKind;

/// A road project with `road.junctions.edit`'s drawing: alignments and two junctions.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.junctions.edit.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Junctions(e));
}

#[test]
fn the_window_saves_and_draws_the_junction() {
    let mut app = app();
    let _ = app.update(Message::Run("junction.editor"));
    assert_eq!(app.dialog, Some(Dialog::Junctions));
    let w = app.junction_window.as_ref().expect("open");
    assert_eq!(w.chosen.as_deref(), Some("j-1"));
    let r = w.preview.report.clone().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert_eq!(r.returns.len(), 4);
    send(&mut app, W::Field(2, "25".into()));
    let wider = app.junction_window.as_ref().and_then(|w| w.preview.report.clone()).expect("reads");
    assert!(wider.returns[0].length > r.returns[0].length);
    send(&mut app, W::Field(7, "10".into()));
    let why = app.junction_window.as_ref().and_then(|w| w.preview.why.clone()).expect("three quadrants missing");
    assert!(why.contains("yazılmalı"), "{why}");
    for k in 8..11 {
        send(&mut app, W::Field(k, "18".into()));
    }
    send(&mut app, W::Save);
    let saved = app.document.as_ref().expect("a drawing").model.road_junctions()[0].clone();
    assert_eq!(saved.radius, 25.0);
    assert_eq!(saved.radii, Some(vec![10.0, 18.0, 18.0, 18.0]));
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::Layer("korkuluk".into()));
    send(&mut app, W::Draw);
    assert!(app.junction_window.is_none());
    assert_eq!(app.document.as_ref().expect("a drawing").model.entities().count(), before + 5);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.entities().count(), before);
}

#[test]
fn a_separated_junction_joins_nothing() {
    let mut app = app();
    let _ = app.update(Message::Run("junction.editor"));
    send(&mut app, W::Choose(None));
    send(&mut app, W::Kind(JunctionKind::Separated));
    for (k, v) in [(3, "5"), (4, "5"), (5, "3.5"), (6, "3.5")] {
        send(&mut app, W::Field(k, v.into()));
    }
    let r = app.junction_window.as_ref().and_then(|w| w.preview.report.clone()).expect("reads");
    assert!(!r.connected && r.returns.is_empty());
    send(&mut app, W::Save);
    assert_eq!(app.document.as_ref().expect("a drawing").model.road_junctions().len(), 3);
}

/// The window, wide and narrow, light and dark, and the returns drawn in plan; `.run/gui-evidence/yol-y12-kavsak-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib junction_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y12-kavsak-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("junction.editor"));
        send(&mut app, W::Field(2, "18".into()));
        shot(&mut app, "hemzemin");
        send(&mut app, W::Choose(Some("j-2".into())));
        shot(&mut app, "katli");
        send(&mut app, W::Choose(Some("j-1".into())));
        send(&mut app, W::Layer("korkuluk".into()));
        send(&mut app, W::Draw);
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "plan");
    }
}
