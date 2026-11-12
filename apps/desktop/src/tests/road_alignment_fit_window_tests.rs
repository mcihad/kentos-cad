//! Çizgiden aks (road_alignment_fit_window.rs, plan/road.md Y10.6): the window reads the alignment as it is typed,
//! makes it as one undo step; and its pictures.

use kentos_domain::Slot;

use crate::app::{App, Dialog, Message};
use crate::road_alignment_fit_window::Event as W;
use kentos_commands::AlignmentFitMethod;

/// A road project with `road.alignments.fit`'s drawing: an alignment and the lines it is fitted to.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.alignments.fit.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::RoadAlignmentFit(e));
}

#[test]
fn nothing_selected_opens_no_window() {
    let mut app = app();
    let _ = app.update(Message::Run("alignment.fromLine"));
    assert!(app.road_alignment_fit_window.is_none());
    assert_ne!(app.dialog, Some(Dialog::RoadAlignmentFit));
}

#[test]
fn the_window_makes_the_alignment_as_one_step() {
    let mut app = app();
    app.selection.set([Slot(6)]);
    let _ = app.update(Message::Run("alignment.fromLine"));
    assert_eq!(app.dialog, Some(Dialog::RoadAlignmentFit));
    // Element by element the sampled axis is a long run of short tangents: many PIs.
    let w = app.road_alignment_fit_window.as_ref().expect("open");
    let fitted = w.preview.fitted.as_ref().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert!(fitted.pis.len() > 10, "{}", fitted.pis.len());
    send(&mut app, W::Method(AlignmentFitMethod::BestFit));
    let why = app.road_alignment_fit_window.as_ref().and_then(|w| w.preview.why.clone()).expect("refused");
    assert!(why.contains("maxRadius"), "{why}");
    send(&mut app, W::Field(2, "500".into()));
    let w = app.road_alignment_fit_window.as_ref().expect("open");
    let fitted = w.preview.fitted.as_ref().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert_eq!(fitted.pis.len(), 3);
    assert!((fitted.pis[1].radius - 60.0).abs() < 1e-6, "{:?}", fitted.pis[1]);
    assert!(w.preview.line.len() > 100);
    send(&mut app, W::Field(0, "Ana aks".into()));
    let why = app.road_alignment_fit_window.as_ref().and_then(|w| w.preview.why.clone()).expect("refused");
    assert!(why.contains("Ana aks"), "{why}");
    send(&mut app, W::Field(0, "Köy yolu".into()));
    let before = app.document.as_ref().expect("a drawing").model.road_alignments().len();
    send(&mut app, W::Make);
    assert!(app.road_alignment_fit_window.is_none());
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.road_alignments().len(), before + 1);
    assert_eq!(doc.road_alignments().last().map(|a| a.name.as_str()), Some("Köy yolu"));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.road_alignments().len(), before);
}

/// The window on an arced polyline and on a sampled axis, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y10-cizgiden-aks-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_alignment_fit_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y10-cizgiden-aks-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        app.selection.set([Slot(2)]);
        let _ = app.update(Message::Run("alignment.fromLine"));
        send(&mut app, W::Field(1, "30".into()));
        shot(&mut app, "eleman");
        send(&mut app, W::Close);
        app.selection.set([Slot(6)]);
        let _ = app.update(Message::Run("alignment.fromLine"));
        send(&mut app, W::Method(AlignmentFitMethod::BestFit));
        send(&mut app, W::Field(2, "500".into()));
        send(&mut app, W::Field(3, "0.001".into()));
        shot(&mut app, "uyum");
        send(&mut app, W::Close);
        app.selection.set([Slot(4)]);
        let _ = app.update(Message::Run("alignment.fromLine"));
        send(&mut app, W::Field(3, "0.01".into()));
        shot(&mut app, "sapma");
    }
}
