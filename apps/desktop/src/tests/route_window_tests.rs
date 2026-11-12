//! Ön güzergâh (route_window.rs, plan/road.md Y12.1): the window reads the route as it is typed and draws it, one undo
//! step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::road;
use crate::route_window::Event as W;

/// A road project with `road.routes.find`'s drawing: the terrain and an area to avoid.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.routes.find.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::Route(e)));
}

fn typed(app: &mut App) {
    for (k, v) in [(0, "70"), (1, "80"), (2, "520"), (3, "280"), (4, "10"), (5, "30"), (6, "8"), (7, "10"), (8, "3")] {
        send(app, W::Field(k, v.into()));
    }
}

#[test]
fn the_window_finds_and_draws_the_route() {
    let mut app = app();
    let _ = app.update(Message::Run("route.finder"));
    assert_eq!(app.dialog, Some(Dialog::Road(road::RoadDialogKind::Route)));
    let why = app
        .road
        .route_window
        .as_ref()
        .and_then(|w| w.preview.why.clone())
        .expect("nothing typed yet");
    assert!(why.contains("yazılmalı"), "{why}");
    typed(&mut app);
    let open = app
        .road
        .route_window
        .as_ref()
        .and_then(|w| w.preview.route.clone())
        .expect("a route");
    send(&mut app, W::Avoid("yasak".into()));
    let around = app
        .road
        .route_window
        .as_ref()
        .and_then(|w| w.preview.route.clone())
        .expect("a route");
    assert!(
        around.length > open.length,
        "{} → {}",
        open.length,
        around.length
    );
    send(&mut app, W::Field(6, "0.5".into()));
    let why = app
        .road
        .route_window
        .as_ref()
        .and_then(|w| w.preview.why.clone())
        .expect("refused");
    assert!(why.contains("eğimi aşmayan"), "{why}");
    send(&mut app, W::Field(6, "8".into()));
    let before = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .count();
    send(&mut app, W::Layer("guzergah".into()));
    send(&mut app, W::Draw);
    assert!(app.road.route_window.is_none());
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .count(),
        before + 1
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .count(),
        before
    );
}

/// The window, wide and narrow, light and dark, and the route drawn in plan; `.run/gui-evidence/yol-y12-guzergah-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib route_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y12-guzergah-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("route.finder"));
        typed(&mut app);
        send(&mut app, W::Avoid("yasak".into()));
        shot(&mut app, "pencere");
        send(&mut app, W::Layer("guzergah".into()));
        send(&mut app, W::Draw);
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "plan");
    }
}
