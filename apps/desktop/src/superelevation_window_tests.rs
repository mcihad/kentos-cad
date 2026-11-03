//! Dever (superelevation_window.rs, plan/road.md Y6): the window builds a schedule as it is typed, says why it does
//! not build, and writes through `road.superelevations.edit`, each write one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::superelevation_window::Event as W;

/// A road project with `road.superelevations.query`'s drawing: a surface, the main alignment with one curve (p1), the
/// side alignment, two sample line sets and two schedules (d-1 from the curve, d-2 from rows, stale); or without the
/// schedules.
fn app(schedules: bool) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/road.superelevations.query.json"
    ))
    .expect("cases");
    let mut setup = cases["setup"].clone();
    if !schedules {
        setup["settings"]["road"].as_object_mut().expect("road").remove("superelevations");
    }
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Superelevations(e));
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

/// Dever oluştur on the main alignment: normal −2,5 both sides, the curve's rate, runoff, share and widening.
fn made(app: &mut App) {
    let _ = app.update(Message::Run("superelevation.new"));
    assert_eq!(app.dialog, Some(Dialog::Superelevations));
    send(app, W::Name("Ana aks deveri".into()));
    send(app, W::Normal(0, "-2,5".into()));
    send(app, W::Normal(1, "-2,5".into()));
    send(app, W::AllCurves);
    send(app, W::Cell(0, 1, "6".into()));
    send(app, W::Cell(0, 2, "40".into()));
    send(app, W::Cell(0, 3, "0,3".into()));
    send(app, W::Cell(0, 4, "0,5".into()));
}

#[test]
fn the_window_makes_a_schedule_and_shows_its_diagram() {
    let mut app = app(false);
    made(&mut app);
    let w = app.superelevation_window.as_ref().expect("open");
    let report = w.report.as_ref().expect("built").as_ref().expect("builds");
    assert_eq!(report.critical.len(), 8, "NC, LC, RC and FS in and out");
    assert!(report.right.iter().any(|p| (p[1] - 6.0).abs() < 1e-9), "the left turn puts the right side outside");
    assert!(report.left.iter().any(|p| (p[1] + 6.0).abs() < 1e-9));
    assert!(!report.left_widening.is_empty(), "the inside widens");
    send(&mut app, W::Save);
    assert!(app.superelevation_window.is_none(), "Kaydet closes the window");
    let doc = model(&mut app);
    assert_eq!(doc.road_superelevations().len(), 1);
    assert_eq!(doc.road_superelevations()[0].id, "d-1");
    assert_eq!(doc.undo().as_deref(), Some("Dever"));
    assert!(doc.road_superelevations().is_empty());
}

#[test]
fn a_rate_below_the_normal_slope_says_why_and_a_blank_is_refused() {
    let mut app = app(false);
    made(&mut app);
    send(&mut app, W::Cell(0, 1, "2".into()));
    let w = app.superelevation_window.as_ref().expect("open");
    let why = w.report.as_ref().expect("built").as_ref().expect_err("does not build");
    assert!(why.contains("p1"), "names the curve: {why}");
    // A blank rate is not a number; the window stays and nothing is written.
    send(&mut app, W::Cell(0, 1, String::new()));
    send(&mut app, W::Save);
    assert!(app.superelevation_window.is_some());
    assert!(model(&mut app).road_superelevations().is_empty());
}

#[test]
fn a_stale_schedule_is_rebound_and_a_used_one_stays() {
    let mut app = app(true);
    let _ = app.update(Message::Run("superelevation.editor"));
    send(&mut app, W::Choose(1));
    send(&mut app, W::Rebind);
    let doc = model(&mut app);
    assert_eq!(doc.road_superelevations()[1].alignment_revision, 3);
    assert_eq!(doc.undo().as_deref(), Some("Dever"));
    // A set on the main alignment takes d-1 in Enkesit; then d-1 cannot be removed.
    let _ = app.update(Message::Superelevations(W::Cancel));
    let _ = app.update(Message::Run("section.editor"));
    let _ = app.update(Message::Sections(crate::section_window::Event::Choose(crate::section_window::Chosen::Set(0))));
    let _ = app.update(Message::Sections(crate::section_window::Event::Superelevation(Some("d-1".into()))));
    let _ = app.update(Message::Sections(crate::section_window::Event::Save));
    assert_eq!(model(&mut app).road_sections()[0].superelevation.as_deref(), Some("d-1"));
    let _ = app.update(Message::Run("superelevation.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Remove);
    send(&mut app, W::Save);
    assert_eq!(model(&mut app).road_superelevations().len(), 2, "the set's schedule stays");
}

/// The window over the drawing, wide and narrow, light and dark; `.run/gui-evidence/yol-y6-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib superelevation_window_tests::screens -- --ignored --nocapture
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
        let mut app = app(true);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y6-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("superelevation.editor"));
        shot(&mut app, "dever");
        // The stale side schedule from rows.
        send(&mut app, W::Choose(1));
        shot(&mut app, "dever-satirlar");
        // A rate below the normal slope: why it does not build.
        send(&mut app, W::Choose(0));
        send(&mut app, W::Cell(0, 1, "2".into()));
        shot(&mut app, "dever-red");
        send(&mut app, W::Cancel);
        if name == "acik-genis" {
            // Enkesit's set takes the schedule; the lanes turn with it on the curve.
            let _ = app.update(Message::Run("section.editor"));
            let _ = app.update(Message::Sections(crate::section_window::Event::Choose(crate::section_window::Chosen::Set(0))));
            let _ = app.update(Message::Sections(crate::section_window::Event::Superelevation(Some("d-1".into()))));
            let _ = app.update(Message::Sections(crate::section_window::Event::Save));
            let _ = app.update(Message::Run("section.editor"));
            let _ = app.update(Message::Sections(crate::section_window::Event::Choose(crate::section_window::Chosen::Set(0))));
            for _ in 0..6 {
                let _ = app.update(Message::Sections(crate::section_window::Event::Next));
            }
            shot(&mut app, "enkesit-dever");
        }
    }
}
