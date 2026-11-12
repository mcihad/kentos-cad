//! Otokorkuluk (guardrail_window.rs, plan/road.md Y11.1): the window reads the runs as the rule is typed, saves the
//! guardrail and draws its runs, each one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::guardrail_window::Event as W;
use crate::road;

/// A road project with `road.guardrails.edit`'s drawing: corridors and two guardrails.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.guardrails.edit.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::Guardrails(e)));
}

#[test]
fn the_window_saves_and_draws_the_guardrail() {
    let mut app = app();
    let _ = app.update(Message::Run("guardrail.editor"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Guardrails))
    );
    let w = app.road.guardrail_window.as_ref().expect("open");
    assert_eq!(w.chosen.as_deref(), Some("o-1"));
    let r = w
        .preview
        .report
        .clone()
        .unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert_eq!(r.runs.len(), 2);
    // A wide gap joins nothing more here; a looser rule makes more.
    send(&mut app, W::Warrant(0, false, "0".into()));
    send(&mut app, W::Warrant(0, true, String::new()));
    let more = app
        .road
        .guardrail_window
        .as_ref()
        .and_then(|w| w.preview.report.clone())
        .expect("reads");
    assert!(more.left + more.right > r.left + r.right, "{more:?}");
    send(&mut app, W::Field(1, " ".into()));
    let why = app
        .road
        .guardrail_window
        .as_ref()
        .and_then(|w| w.preview.why.clone())
        .expect("refused");
    assert!(why.contains("kaynağı"), "{why}");
    send(&mut app, W::Field(1, "Kurum tablosu".into()));
    send(&mut app, W::Save);
    let saved = app.document.as_ref().expect("a drawing").model.road_guardrails()[0].clone();
    assert_eq!(saved.rule.source, "Kurum tablosu");
    assert_eq!(saved.revision, 2);
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::Layer("korkuluk".into()));
    send(&mut app, W::Draw);
    assert!(app.road.guardrail_window.is_none());
    let after = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .count();
    assert!(after > before, "{before} → {after}");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.entities().count(), before);
}

#[test]
fn a_new_guardrail_needs_its_rule() {
    let mut app = app();
    let _ = app.update(Message::Run("guardrail.editor"));
    send(&mut app, W::Choose(None));
    send(&mut app, W::Field(0, "Yeni korkuluk".into()));
    let why = app
        .road
        .guardrail_window
        .as_ref()
        .and_then(|w| w.preview.why.clone())
        .expect("no rule yet");
    assert!(why.contains("dolgu"), "{why}");
    send(&mut app, W::Warrant(0, false, "1".into()));
    for (k, v) in [(1, "Kurum tablosu"), (4, "30"), (5, "15"), (6, "40")] {
        send(&mut app, W::Field(k, v.into()));
    }
    let w = app.road.guardrail_window.as_ref().expect("open");
    assert!(w.preview.report.is_some(), "{:?}", w.preview.why);
    send(&mut app, W::Save);
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .road_guardrails()
            .len(),
        3
    );
}

/// The window, wide and narrow, light and dark, and the runs drawn in plan; `.run/gui-evidence/yol-y11-korkuluk-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib guardrail_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y11-korkuluk-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("guardrail.editor"));
        shot(&mut app, "pencere");
        send(&mut app, W::Layer("korkuluk".into()));
        send(&mut app, W::Draw);
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "plan");
    }
}
