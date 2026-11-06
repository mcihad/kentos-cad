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

/// `road.sheets.plan`'s KGM case drawing (docs/adr/0980): the same project with a superelevation schedule by rows.
fn kgm_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.sheets.plan.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "KGM bantları ve dever çizelgesiyle").expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// KGM's bands on the sheets (docs/adr/0980, 0981): they take KGM's A1 sheet, whose plan map is 791 mm long against
/// the A3 one's 370 mm and whose profile map holds the profile and its 107.5 mm of bands; the Dever band's schedule is
/// the alignment's; one undo step.
#[test]
fn the_sheets_take_kgm_s_bands() {
    let mut app = kgm_app();
    let _ = app.update(Message::Run("road.sheets"));
    send(&mut app, W::Field(1, "1000".into()));
    send(&mut app, W::Profile(Some("b-1".into())));
    let plain = app.road_sheets_window.as_ref().and_then(|w| w.preview.laid.clone()).expect("reads");
    send(&mut app, W::Kgm(true));
    send(&mut app, W::Dever(Some("d-1".into())));
    let kgm = app.road_sheets_window.as_ref().and_then(|w| w.preview.laid.clone()).expect("reads");
    assert_eq!((plain.length, plain.sheets.len()), (370.0, 2), "KentOS's A3 sheet at 1:1000");
    assert_eq!((kgm.length, kgm.height, kgm.sheets.len()), (791.0, 230.0, 1), "KGM's A1 sheet at 1:1000");
    let view = kgm.sheets[0].view.expect("a profile view");
    assert!(view[1][1] - view[0][1] + 107.5 <= 318.0, "the profile and KGM's bands fit the A1 profile map: {view:?}");
    send(&mut app, W::Make);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(doc.entities().any(|e| matches!(e, kentos_commands::Entity::Text(t) if t.text == "Yatay Geometri")));
    // The sheet goes to PDF: every text of the bands and of the plan map's axis is in the drawing font (docs/adr/0982:
    // the bands' Δ was not, and the book refused).
    let pdf = kentos_plot::book::book_pdf(doc, &[], "test", "2026-11-06");
    assert!(pdf.is_ok(), "{:?}", pdf.err());
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Model));
    let _ = app.update(Message::Run("edit.undo"));
    assert!(app.document.as_ref().expect("a drawing").model.road_sheet_sets().is_empty());
}

/// The window with KGM's bands and the first sheet, light and dark; `.run/gui-evidence/yol-y75/` (KGM's A1 sheet):
///
/// ```text
/// cargo test -p kentos-desktop --lib road_sheets_window_tests::screens_kgm -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_kgm() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y75");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = kgm_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y75-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("road.sheets"));
        send(&mut app, W::Field(1, "1000".into()));
        send(&mut app, W::Profile(Some("b-1".into())));
        send(&mut app, W::Kgm(true));
        send(&mut app, W::Dever(Some("d-1".into())));
        shot(&mut app, "pencere");
        send(&mut app, W::Make);
        shot(&mut app, "pafta-ilk");
        let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::InspectorTab(kentos_sheet_ui::InspectorTab::Preflight)));
        shot(&mut app, "pafta-ondenetim");
        if theme == "light" {
            // The sheet as its PDF writes it (docs/adr/0982: the plan map's axis on paper).
            let doc = &app.document.as_ref().expect("a drawing").model;
            let pdf = kentos_plot::book::book_pdf(doc, &[], "test", "2026-11-06").expect("the book's PDF");
            let file = out.join("y75-pafta.pdf");
            std::fs::write(&file, pdf.bytes).expect("writes the PDF");
            println!("{}", file.display());
        }
    }
}

/// The plan map's axis (docs/adr/0982) on KGM's A1 sheet at 1:2000, where the whole sample alignment fits the map: the
/// red axis, the hectometres, the curve's main points and its data at the PI. Light and dark, and the sheet's PDF
/// (rendered with `pdftoppm -r 150`); `.run/gui-evidence/yol-y76/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_sheets_window_tests::screens_axes -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_axes() {
    shoot_kgm_sheet("yol-y76", "y76", "2026-11-06");
}

/// The PI data with the superelevation (docs/adr/0983): S, Lr, Lt, La and Gn from the sample's schedule, and KGM's
/// simple-curve T and L. The same sheet as [`screens_axes`]; `.run/gui-evidence/yol-y77/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_sheets_window_tests::screens_pi_data -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_pi_data() {
    shoot_kgm_sheet("yol-y77", "y77", "2026-11-06");
}

/// The plan map as KGM's June 2024 sample sheets draw it (docs/adr/0984): the PI data box with D, Se and the
/// clothoid curve's S-D, S-X, S-Y, the main points `TS1=…`, the PI's triangle and the ends' coordinates;
/// `.run/gui-evidence/yol-y78/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_sheets_window_tests::screens_kgm_2024 -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_kgm_2024() {
    shoot_kgm_sheet("yol-y78", "y78", "2026-11-06");
}

/// The PI box with V (docs/adr/0985): the sample's main alignment at a design speed of 80 km/h;
/// `.run/gui-evidence/yol-y79/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib road_sheets_window_tests::screens_design_speed -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_design_speed() {
    shoot_kgm_sheet_with("yol-y79", "y79", "2026-11-06", |app| {
        let doc = app.document.as_mut().expect("a drawing");
        let edit = kentos_commands::AlignmentsEdit {
            alignment: "g-1".into(),
            operation: kentos_commands::AlignmentsEditOperation::Set,
            name: None,
            pis: None,
            stationing: None,
            offset: None,
            design_speed: Some(80.0),
            expected_revision: None,
        };
        let done = kentos_native_application::road_alignments_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            edit,
        );
        assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    });
}

/// The profile's ends (docs/adr/0987): PROJE BAŞI and PROJE SONU with their km and elevation on the design line;
/// `.run/gui-evidence/yol-y81/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib road_sheets_window_tests::screens_profile_ends -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_profile_ends() {
    shoot_kgm_sheet("yol-y81", "y81", "2026-11-06");
}

/// KGM's A1 plan–profile sheet of the sample at 1:2000 with its schedule, light and dark, and its PDF, into
/// `.run/gui-evidence/<folder>/<prefix>-pafta-*`.
fn shoot_kgm_sheet(folder: &str, prefix: &str, date: &str) {
    shoot_kgm_sheet_with(folder, prefix, date, |_| {});
}

/// [`shoot_kgm_sheet`] with the drawing first changed by `prepare`.
fn shoot_kgm_sheet_with(folder: &str, prefix: &str, date: &str, prepare: impl Fn(&mut App)) {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence").join(folder);
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = kgm_app();
        prepare(&mut app);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("road.sheets"));
        send(&mut app, W::Field(1, "2000".into()));
        send(&mut app, W::Profile(Some("b-1".into())));
        send(&mut app, W::Kgm(true));
        send(&mut app, W::Dever(Some("d-1".into())));
        send(&mut app, W::Make);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("{prefix}-pafta-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        if theme == "light" {
            let doc = &app.document.as_ref().expect("a drawing").model;
            let pdf = kentos_plot::book::book_pdf(doc, &[], "test", date).expect("the book's PDF");
            let file = out.join(format!("{prefix}-pafta.pdf"));
            std::fs::write(&file, pdf.bytes).expect("writes the PDF");
            println!("{}", file.display());
        }
    }
}
