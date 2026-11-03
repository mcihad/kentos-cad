//! Yüzey analizi (surface_analysis_window.rs, plan/road.md Y18.5): the window reads a surface's slope ranges, two
//! surfaces' difference and the check points by class without writing; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::surface_analysis_window::{Event as W, Mode, Read};

/// `road.terrains.quality`'s drawing: the lattices (valley, pit, plane, terrace, ridge, cut, pad, sparse, spiky) and
/// the check points.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.quality.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// `road.terrains.repair`'s drawing: the same lattices and a plane with two spikes inside its squares.
fn repair_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.repair.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The spikes of the repair's plane asked.
fn spiky(app: &mut App) {
    let _ = app.update(Message::Run("terrain.analysis"));
    send(app, W::Mode(Mode::Quality));
    send(app, W::Surface("t-12".into()));
    send(app, W::Field(4, "2.5".into()));
    send(app, W::Field(5, "0.6".into()));
    send(app, W::Preview);
}

fn excluded(app: &App) -> usize {
    app.document.as_ref().and_then(|d| d.model.settings().road.as_ref()).map_or(0, |r| {
        r.surfaces.iter().find(|s| s.id == "t-12").map_or(0, |s| s.excluded.len())
    })
}

#[test]
fn the_spikes_are_left_out_after_their_difference_and_taken_back() {
    let mut app = repair_app();
    spiky(&mut app);
    let Read::Quality(q) = read(&app) else { panic!("quality") };
    assert_eq!(q.spike_count, Some(2), "{q:?}");
    send(&mut app, W::LeaveSpikes);
    let r = match &app.surface_analysis_window.as_ref().expect("open").repair {
        Some((_, Ok(r))) => r.clone(),
        other => panic!("a difference: {other:?}"),
    };
    assert!(r.cut > 0.0 && r.fill > 0.0 && r.corners_after + 2 == r.corners_before, "{r:?}");
    assert_eq!(excluded(&app), 0, "the difference writes nothing");
    send(&mut app, W::Apply);
    assert_eq!(excluded(&app), 2);
    let Read::Quality(q) = read(&app) else { panic!("quality") };
    assert_eq!(q.spike_count, Some(0), "read again without them: {q:?}");
    send(&mut app, W::RestoreAll);
    send(&mut app, W::Apply);
    assert_eq!(excluded(&app), 0);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(excluded(&app), 2);
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::SurfaceAnalysis(e));
}

fn read(app: &App) -> Read {
    match &app.surface_analysis_window.as_ref().expect("open").preview {
        Some(Ok(r)) => r.clone(),
        other => panic!("read: {other:?}"),
    }
}

#[test]
fn the_window_reads_ranges_a_difference_and_check_points() {
    let mut app = app();
    let before = app.document.as_ref().expect("a drawing").model.revision().to_string();
    let _ = app.update(Message::Run("terrain.analysis"));
    assert_eq!(app.dialog, Some(Dialog::SurfaceAnalysis));
    // The breaks are the user's: none asked, none read.
    send(&mut app, W::Preview);
    assert!(matches!(&app.surface_analysis_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("Bir şey sorun")));
    send(&mut app, W::Surface("t-6".into()));
    send(&mut app, W::Field(0, "0, 1, 12, 35".into()));
    send(&mut app, W::Roughness(true));
    send(&mut app, W::Preview);
    let Read::Analysis(a) = read(&app) else { panic!("an analysis") };
    let slopes = a.slopes.expect("slopes");
    assert_eq!(slopes.bands.len(), 3);
    assert!((slopes.bands.iter().map(|b| b.area).sum::<f64>() + slopes.outside_area - a.area).abs() < 1e-6);
    assert!(slopes.bands.iter().any(|b| !b.parts.is_empty()) && a.roughness.is_some());
    // The difference of the pad over the terrace.
    send(&mut app, W::Mode(Mode::Compare));
    send(&mut app, W::Comparison("t-9".into()));
    send(&mut app, W::Field(2, "-0.1, 0, 0.1, 0.2".into()));
    send(&mut app, W::Preview);
    let Read::Compare(c) = read(&app) else { panic!("a difference") };
    assert!(c.fill > 0.0 && c.cut > 0.0 && c.bands.len() == 3, "{c:?}");
    // The check points by class.
    send(&mut app, W::Mode(Mode::Quality));
    send(&mut app, W::Checks(Some("kontrol".into())));
    send(&mut app, W::Field(7, "Sınıf".into()));
    send(&mut app, W::Preview);
    let Read::Quality(q) = read(&app) else { panic!("quality") };
    let checks = q.checkpoints.expect("check points");
    assert_eq!(checks.classes.iter().map(|c| c.class.as_str()).collect::<Vec<_>>(), ["", "bitkili", "sert"]);
    assert_eq!(checks.all.outside, 1);
    // Spikes need both values.
    send(&mut app, W::Field(4, "1".into()));
    send(&mut app, W::Preview);
    assert!(matches!(&app.surface_analysis_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("birlikte")));
    send(&mut app, W::Close);
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision().to_string(), before);
}

/// The window, wide and narrow, light and dark, each way; `.run/gui-evidence/yol-y18-yuzey-analizi-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib surface_analysis_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y18-yuzey-analizi-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.analysis"));
        send(&mut app, W::Surface("t-3".into()));
        send(&mut app, W::Measure(0));
        send(&mut app, W::Field(0, "100, 125, 150, 175, 200, 225, 250".into()));
        send(&mut app, W::Roughness(true));
        send(&mut app, W::Field(1, "30".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "kot");
        send(&mut app, W::Surface("t-6".into()));
        send(&mut app, W::Measure(1));
        send(&mut app, W::Field(0, "0, 1, 5, 12, 25, 35".into()));
        send(&mut app, W::Field(1, String::new()));
        send(&mut app, W::Preview);
        shot(&mut app, "egim");
        send(&mut app, W::Mode(Mode::Compare));
        send(&mut app, W::Comparison("t-9".into()));
        send(&mut app, W::Field(2, "-0.1, -0.05, 0, 0.05, 0.1, 0.15, 0.2".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "fark");
        send(&mut app, W::Mode(Mode::Quality));
        send(&mut app, W::Checks(Some("kontrol".into())));
        send(&mut app, W::Field(7, "Sınıf".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "kontrol");
        let mut app = repair_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("terrain"));
        spiky(&mut app);
        send(&mut app, W::LeaveSpikes);
        shot(&mut app, "onarim");
    }
}
