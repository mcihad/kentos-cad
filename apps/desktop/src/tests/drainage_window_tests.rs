//! Drenaj (drainage_window.rs, plan/road.md Y16.3): the window previews a catchment, its peak flow against a culvert
//! and the ponds without writing, and draws them in one step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::drainage_window::Event as W;
use crate::road;

/// A road project with `road.drainage.query`'s drawing: the culverts' corridors and three lattice terrains.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.drainage.query.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::Drainage(e)));
}

/// The valley's outlet with its grid, a culvert and the Rational method's values.
fn valley(app: &mut App) {
    send(app, W::Surface("t-3".into()));
    for (k, v) in [(0, "2000,500"), (1, "50"), (2, "30"), (3, "0.9"), (4, "200")] {
        send(app, W::Field(k, v.into()));
    }
    send(app, W::Culvert(Some("m-1".into())));
}

#[test]
fn the_window_previews_the_catchment_against_the_culvert_and_draws_it() {
    let mut app = app();
    let _ = app.update(Message::Run("drainage.editor"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Drainage))
    );
    valley(&mut app);
    send(&mut app, W::Preview);
    let w = app.road.drainage_window.as_ref().expect("open");
    let q = match &w.preview {
        Some(Ok(q)) => q.clone(),
        other => panic!("previewed: {other:?}"),
    };
    let c = q.catchment.expect("a catchment");
    let k = c.capacity.expect("the culvert's");
    assert!(k.ratio.expect("a ratio") > 1.0, "{k:?}");
    assert!(w.warnings.iter().any(|m| m.contains("kapasitesi")), "{:?}", w.warnings);
    // A change says the preview is old; a culvert's inlet is a discharge point too.
    send(&mut app, W::Field(0, String::new()));
    assert!(
        app.road
            .drainage_window
            .as_ref()
            .expect("open")
            .preview
            .is_none()
    );
    send(&mut app, W::Field(0, "2000,500".into()));
    send(&mut app, W::Preview);
    let before = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .count();
    send(&mut app, W::Layer("drenaj".into()));
    send(&mut app, W::Draw);
    assert!(app.road.drainage_window.is_none());
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .count(),
        before + 3
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

#[test]
fn the_ponds_of_the_pit() {
    let mut app = app();
    let _ = app.update(Message::Run("drainage.editor"));
    send(&mut app, W::Surface("t-4".into()));
    send(&mut app, W::Field(5, "0".into()));
    send(&mut app, W::Preview);
    let q = match &app.road.drainage_window.as_ref().expect("open").preview {
        Some(Ok(q)) => q.clone(),
        other => panic!("previewed: {other:?}"),
    };
    assert_eq!(q.ponds.len(), 1);
    assert!((q.ponds[0].depth - 5.0).abs() < 1e-9 && (q.ponds[0].volume - 250.0 / 3.0).abs() < 1e-9);
    send(&mut app, W::Field(1, "x".into()));
    send(&mut app, W::Field(0, "4004,1".into()));
    send(&mut app, W::Preview);
    let why = match &app.road.drainage_window.as_ref().expect("open").preview {
        Some(Err(why)) => why.clone(),
        other => panic!("refused: {other:?}"),
    };
    assert!(why.contains("sayı değil"), "{why}");
}

/// The window, wide and narrow, light and dark, and the catchment drawn; `.run/gui-evidence/yol-y16-drenaj-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib drainage_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y16-drenaj-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("drainage.editor"));
        valley(&mut app);
        send(&mut app, W::Preview);
        shot(&mut app, "havza");
        send(&mut app, W::Surface("t-4".into()));
        send(&mut app, W::Culvert(None));
        for (k, v) in [(0, ""), (5, "0")] {
            send(&mut app, W::Field(k, v.into()));
        }
        send(&mut app, W::Preview);
        shot(&mut app, "golet");
        send(&mut app, W::Surface("t-3".into()));
        valley(&mut app);
        send(&mut app, W::Layer("drenaj".into()));
        send(&mut app, W::Preview);
        send(&mut app, W::Draw);
        let drawn: Vec<kentos_domain::Slot> = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .filter(|e| e.base().layer_id == "drenaj" || e.base().layer_id == "vadi")
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        app.selection.set(drawn);
        let _ = app.update(Message::Run("view.zoomSelection"));
        let _ = app.update(Message::Run("edit.deselect"));
        shot(&mut app, "plan");
    }
}
