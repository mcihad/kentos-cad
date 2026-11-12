//! Araç izi (swept_window.rs, plan/road.md Y17.3): the window previews a vehicle's swept path round a junction's
//! quadrant or along the selected line without writing, takes the selection as obstacles and draws the tracks in one
//! step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::swept_window::{Event as W, Source};

/// `road.swept.query`'s drawing: the junctions' road project with a few lines, an arc, a circle and an island.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.swept.query.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Swept(e));
}

/// The uid of the setup's `k`th added object (road_swept_cases.py's order: line, ell, bend, arc, curb, post, …).
fn select(app: &mut App, uids: &[&str]) {
    let doc = app.document.as_ref().expect("a drawing");
    let slots: Vec<kentos_domain::Slot> = uids.iter().map(|u| doc.model.slot_of(u.parse().expect("a uid")).expect("drawn")).collect();
    app.selection.set(slots);
}

const ELL: &str = "0192f5d0-7c3e-7d4a-9b1e-000000000125";
const POST: &str = "0192f5d0-7c3e-7d4a-9b1e-000000000129";
const ISLAND: &str = "0192f5d0-7c3e-7d4a-9b1e-000000000130";

/// A truck's sizes, typed.
fn truck(app: &mut App) {
    for (k, v) in [(0, "6.1"), (1, "1.2"), (2, "1.8"), (3, "2.6"), (11, "1")] {
        send(app, W::Field(k, v.into()));
    }
}

#[test]
fn the_window_previews_a_quadrant_and_draws_it() {
    let mut app = app();
    let _ = app.update(Message::Run("swept.editor"));
    assert_eq!(app.dialog, Some(Dialog::Swept));
    let w = app.swept_window.as_ref().expect("open");
    assert_eq!((w.source, w.junction.as_deref()), (Source::Junction, Some("j-1")));
    // Nothing is filled in: the sizes are the user's.
    send(&mut app, W::Preview);
    match &app.swept_window.as_ref().expect("open").preview {
        Some(Err(why)) => assert!(why.contains("yazılmalı"), "{why}"),
        other => panic!("refused: {other:?}"),
    }
    truck(&mut app);
    for (k, v) in [(9, "3.5"), (10, "10")] {
        send(&mut app, W::Field(k, v.into()));
    }
    send(&mut app, W::Quadrant(1));
    send(&mut app, W::Preview);
    let w = app.swept_window.as_ref().expect("open");
    let q = match &w.preview {
        Some(Ok(q)) => q.clone(),
        other => panic!("previewed: {other:?}"),
    };
    assert!(q.offtracking > 1.0 && q.clearance.as_ref().is_some_and(|c| c.curb && c.distance > 0.0), "{q:?}");
    assert_eq!(w.junction_read.as_ref().map(|j| j.returns.len()), Some(4));
    // A change says the preview is old.
    send(&mut app, W::Field(9, "0.5".into()));
    assert!(app.swept_window.as_ref().expect("open").preview.is_none());
    send(&mut app, W::Field(9, "3.5".into()));
    send(&mut app, W::Field(12, "5".into()));
    send(&mut app, W::Preview);
    send(&mut app, W::Layer("arac".into()));
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::Draw);
    assert!(app.swept_window.is_none());
    let after = app.document.as_ref().expect("a drawing").model.entities().count();
    let bodies = (0..q.samples as usize).filter(|k| k % 5 == 0 || *k + 1 == q.samples as usize).count();
    assert_eq!(after, before + 1 + 4 + bodies + 1);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.entities().count(), before);
}

#[test]
fn a_trailer_along_the_selected_line_against_the_selected_obstacles() {
    let mut app = app();
    select(&mut app, &[ELL]);
    let _ = app.update(Message::Run("swept.editor"));
    let w = app.swept_window.as_ref().expect("open");
    assert_eq!((w.source, w.path.as_deref()), (Source::Object, Some(ELL)));
    truck(&mut app);
    send(&mut app, W::Trailer(true));
    send(&mut app, W::Preview);
    match &app.swept_window.as_ref().expect("open").preview {
        Some(Err(why)) => assert!(why.contains("Mafsal"), "{why}"),
        other => panic!("refused: {other:?}"),
    }
    for (k, v) in [(4, "0.3"), (5, "8"), (6, "1"), (7, "2"), (8, "2.5")] {
        send(&mut app, W::Field(k, v.into()));
    }
    select(&mut app, &[ISLAND, POST]);
    send(&mut app, W::Obstacles);
    assert_eq!(app.swept_window.as_ref().expect("open").obstacles.len(), 2);
    send(&mut app, W::Preview);
    let w = app.swept_window.as_ref().expect("open");
    let q = match &w.preview {
        Some(Ok(q)) => q.clone(),
        other => panic!("previewed: {other:?}"),
    };
    assert_eq!(q.corners.len(), 8);
    assert!(q.trailer_offtracking.is_some_and(|t| t > q.offtracking), "{q:?}");
    assert!(q.clearance.as_ref().is_some_and(|c| c.distance == 0.0), "{q:?}");
    assert!(w.warnings.iter().any(|m| m.contains("değiyor")), "{:?}", w.warnings);
    send(&mut app, W::Clear);
    assert!(app.swept_window.as_ref().expect("open").obstacles.is_empty());
}

/// The window, wide and narrow, light and dark, and the tracks drawn; `.run/gui-evidence/yol-y17-arac-izi-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib swept_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y17-arac-izi-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("swept.editor"));
        truck(&mut app);
        for (k, v) in [(9, "1.5"), (10, "10"), (12, "6")] {
            send(&mut app, W::Field(k, v.into()));
        }
        send(&mut app, W::Preview);
        shot(&mut app, "kavsak");
        send(&mut app, W::Quadrant(1));
        send(&mut app, W::Field(9, "3.5".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "genis-donus");
        send(&mut app, W::Close);
        select(&mut app, &[ELL]);
        let _ = app.update(Message::Run("swept.editor"));
        truck(&mut app);
        send(&mut app, W::Trailer(true));
        for (k, v) in [(4, "0.3"), (5, "8"), (6, "1"), (7, "2"), (8, "2.5"), (12, "5")] {
            send(&mut app, W::Field(k, v.into()));
        }
        select(&mut app, &[ISLAND, POST]);
        send(&mut app, W::Obstacles);
        send(&mut app, W::Preview);
        shot(&mut app, "romork");
        send(&mut app, W::Layer("arac".into()));
        send(&mut app, W::Draw);
        let drawn: Vec<kentos_domain::Slot> = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .filter(|e| e.base().layer_id == "arac")
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        app.selection.set(drawn);
        let _ = app.update(Message::Run("view.zoomSelection"));
        let _ = app.update(Message::Run("edit.deselect"));
        shot(&mut app, "plan");
    }
}
