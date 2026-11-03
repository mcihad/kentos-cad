//! Yol paftaları (road_sheets_window.rs, plan/road.md Y10.5): the window reads the sheets as it is typed, lays them as
//! one undo step and shows them; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::road_sheets_window::Event as W;

/// A road project with `road.sheets.plan`'s drawing: two alignments, three profiles, a terrain.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.sheets.plan.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::RoadSheets(e));
}

#[test]
fn the_window_lays_the_sheets_as_one_step() {
    let mut app = app();
    let _ = app.update(Message::Run("road.sheets"));
    assert_eq!(app.dialog, Some(Dialog::RoadSheets));
    let w = app.road_sheets_window.as_ref().expect("open");
    let laid = w.preview.laid.as_ref().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert_eq!(laid.scale, 1000);
    assert!(laid.sheets.iter().all(|s| s.view.is_some()), "{laid:?}");
    let count = laid.sheets.len();
    send(&mut app, W::Profile(None));
    let plan_only = app.road_sheets_window.as_ref().and_then(|w| w.preview.laid.clone()).expect("reads");
    assert!(plan_only.sheets.iter().all(|s| s.view.is_none()));
    send(&mut app, W::Field(1, "bir".into()));
    let why = app.road_sheets_window.as_ref().and_then(|w| w.preview.why.clone()).expect("refused");
    assert!(why.contains("bir"), "{why}");
    send(&mut app, W::Field(1, "1000".into()));
    send(&mut app, W::Profile(Some("b-1".into())));
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::Make);
    assert!(app.road_sheets_window.is_none());
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.road_sheet_sets().len(), 1);
    assert_eq!(doc.road_sheet_sets()[0].sheets.len(), count);
    assert!(doc.entities().count() > before);
    assert_eq!(app.sheets.book().sheets.len(), count, "the designer shows the sheets");
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Model));
    let _ = app.update(Message::Run("edit.undo"));
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(doc.road_sheet_sets().is_empty());
    assert_eq!(doc.entities().count(), before, "the profile views go with the step");
}

/// The window and the first sheet, wide and narrow, light and dark; `.run/gui-evidence/yol-y10-pafta-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_sheets_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y10-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        // The alignment's km written into the drawing, so the plan maps show it (the alignment itself is drawn over the
        // model, not in it).
        if let Some(doc) = app.document.as_mut() {
            let label = kentos_commands::AlignmentsLabel {
                alignment: "g-1".into(),
                layer_id: "0".into(),
                every: Some(20.0),
                major: Some(100.0),
                height: None,
                curves: Some(true),
                expected_revision: None,
            };
            let done = kentos_native_application::road_alignments_label::execute(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                label,
            );
            assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
        }
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("road.sheets"));
        shot(&mut app, "pafta-pencere");
        send(&mut app, W::Make);
        shot(&mut app, "pafta-ilk");
    }
}
