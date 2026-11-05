//! Menfez (culvert_window.rs, plan/road.md Y11.2): the window reads the culvert as it is typed, saves it and draws its
//! barrel, each one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::culvert_window::Event as W;

/// A road project with `road.culverts.edit`'s drawing: corridors and two culverts.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.culverts.edit.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Culverts(e));
}

#[test]
fn the_window_saves_and_draws_the_culvert() {
    let mut app = app();
    let _ = app.update(Message::Run("culvert.editor"));
    assert_eq!(app.dialog, Some(Dialog::Culverts));
    let w = app.culvert_window.as_ref().expect("open");
    assert_eq!(w.chosen.as_deref(), Some("m-1"));
    assert!(!w.pipe);
    let r = w.preview.report.clone().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert_eq!(r.ends.len(), 2);
    // A wider skew makes a longer barrel.
    send(&mut app, W::Field(2, "30".into()));
    let longer = app.culvert_window.as_ref().and_then(|w| w.preview.report.clone()).expect("reads");
    assert!(longer.length > r.length, "{} → {}", r.length, longer.length);
    send(&mut app, W::Field(3, "kot".into()));
    let why = app.culvert_window.as_ref().and_then(|w| w.preview.why.clone()).expect("refused");
    assert!(why.contains("Akış kotu"), "{why}");
    send(&mut app, W::Field(3, r.ends[0].invert.max(r.ends[1].invert).to_string()));
    send(&mut app, W::Save);
    let saved = app.document.as_ref().expect("a drawing").model.road_culverts()[0].clone();
    assert_eq!(saved.skew, 30.0);
    assert_eq!(saved.revision, 2);
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::Layer("korkuluk".into()));
    send(&mut app, W::Draw);
    assert!(app.culvert_window.is_none());
    let after = app.document.as_ref().expect("a drawing").model.entities().count();
    assert_eq!(after, before + 3);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.entities().count(), before);
}

#[test]
fn a_new_pipe_culvert_needs_its_sizes() {
    let mut app = app();
    let _ = app.update(Message::Run("culvert.editor"));
    send(&mut app, W::Choose(None));
    send(&mut app, W::Pipe(true));
    let why = app.culvert_window.as_ref().and_then(|w| w.preview.why.clone()).expect("no sizes yet");
    assert!(why.contains("yazılmalı"), "{why}");
    let (invert, station) = {
        let m = &app.document.as_ref().expect("a drawing").model.road_culverts()[0];
        (m.invert, m.station)
    };
    for (k, v) in [(1, station.to_string()), (3, invert.to_string()), (4, "1".into()), (8, "1".into()), (9, "0.1".into())] {
        send(&mut app, W::Field(k, v));
    }
    let w = app.culvert_window.as_ref().expect("open");
    assert!(w.preview.report.is_some(), "{:?}", w.preview.why);
    send(&mut app, W::Save);
    assert_eq!(app.document.as_ref().expect("a drawing").model.road_culverts().len(), 3);
}

/// Hidrolik reads the saved culvert as `road.culverts.hydraulics` does: the shared case “Yol aşılıyor” typed in.
#[test]
fn the_hydraulics_read_the_shared_case() {
    let mut app = app();
    let _ = app.update(Message::Run("culvert.editor"));
    assert!(app.culvert_window.as_ref().expect("open").hydraulics.is_none(), "nothing before a flow");
    send(&mut app, W::Hydraulic(0, "60".into()));
    let why = match app.culvert_window.as_ref().and_then(|w| w.hydraulics.clone()) {
        Some(Err(why)) => why,
        other => panic!("refused without ke: {other:?}"),
    };
    assert!(why.contains("ke"), "{why}");
    send(&mut app, W::Hydraulic(2, "0,4".into()));
    send(&mut app, W::Hydraulic(4, "1.66".into()));
    let why = match app.culvert_window.as_ref().and_then(|w| w.hydraulics.clone()) {
        Some(Err(why)) => why,
        other => panic!("a weir needs both: {other:?}"),
    };
    assert!(why.contains("birlikte"), "{why}");
    send(&mut app, W::Hydraulic(5, "60".into()));
    let w = app.culvert_window.as_ref().expect("open");
    assert_eq!(w.inlet, kentos_commands::CulvertInlet::BoxWingwall30To75, "a box's first inlet");
    let (read, _) = w.hydraulics.clone().expect("asked").expect("reads");
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.culverts.hydraulics.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "Yol aşılıyor").expect("the case");
    let want = &case["steps"][0]["result"]["output"]["at"];
    assert!((read.at.level - want["level"].as_f64().expect("level")).abs() < 1e-6);
    assert!((read.at.overtopping - want["overtopping"].as_f64().expect("over")).abs() < 1e-6);
    assert_eq!(read.at.over.as_ref().map(|o| o.hazard), want["over"]["hazard"].as_u64().map(|h| h as u8));
    assert_eq!(read.curve.len(), 13);
    // A pipe offers only the pipes' inlets.
    send(&mut app, W::Choose(Some("m-2".into())));
    assert_eq!(app.culvert_window.as_ref().expect("open").inlet, kentos_commands::CulvertInlet::PipeSquareHeadwall);
}

/// Kapanma reads a flood as `road.culverts.closure` does: the shared cases “Üçgen hidrograf, kutu menfez” and “Debi ve Cd
/// aralığıyla” typed in.
#[test]
fn the_closure_reads_the_shared_cases() {
    let mut app = app();
    let _ = app.update(Message::Run("culvert.editor"));
    send(&mut app, W::Page(2));
    send(&mut app, W::Flood(0, "60".into()));
    let why = match app.culvert_window.as_ref().and_then(|w| w.closure.clone()) {
        Some(Err(why)) => why,
        other => panic!("refused without its hours: {other:?}"),
    };
    assert!(why.contains("Tepeye süre"), "{why}");
    send(&mut app, W::Flood(1, "3".into()));
    send(&mut app, W::Flood(2, "12".into()));
    send(&mut app, W::Hydraulic(2, "0.4".into()));
    let why = match app.culvert_window.as_ref().and_then(|w| w.closure.clone()) {
        Some(Err(why)) => why,
        other => panic!("refused without a weir: {other:?}"),
    };
    assert!(why.contains("savak"), "{why}");
    send(&mut app, W::Hydraulic(4, "1.66".into()));
    send(&mut app, W::Hydraulic(5, "60".into()));
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.culverts.closure.json")).expect("cases");
    let expected = |name: &str| {
        let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case");
        case["steps"][0]["result"]["output"]["floods"].clone()
    };
    let check = |app: &App, name: &str| {
        let (read, _) = app.culvert_window.as_ref().and_then(|w| w.closure.clone()).expect("asked").expect("reads");
        let want = expected(name);
        assert_eq!(read.floods.len(), want.as_array().expect("floods").len(), "{name}");
        for (f, w) in read.floods.iter().zip(want.as_array().expect("floods")) {
            for (c, cw) in f.criteria.iter().zip(w["criteria"].as_array().expect("criteria")) {
                let h = cw["hours"].as_f64().expect("hours");
                assert!((c.hours - h).abs() < 1e-6, "{name} {} {}: {} vs {h}", f.case, c.criterion, c.hours);
            }
        }
    };
    check(&app, "Üçgen hidrograf, kutu menfez");
    for (k, v) in [(4, "0.8"), (5, "1.2"), (6, "1.5"), (7, "1.8")] {
        send(&mut app, W::Flood(k, v.into()));
    }
    check(&app, "Debi ve Cd aralığıyla");
    send(&mut app, W::Flood(5, String::new()));
    let why = match app.culvert_window.as_ref().and_then(|w| w.closure.clone()) {
        Some(Err(why)) => why,
        other => panic!("a range needs both factors: {other:?}"),
    };
    assert!(why.contains("birlikte"), "{why}");
}

/// The window, wide and narrow, light and dark, and the barrel drawn in plan; `.run/gui-evidence/yol-y11-menfez-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib culvert_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y11-menfez-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("culvert.editor"));
        shot(&mut app, "kutu");
        send(&mut app, W::Page(1));
        for (k, v) in [(0, "60"), (2, "0.4"), (4, "1.66"), (5, "60")] {
            send(&mut app, W::Hydraulic(k, v.into()));
        }
        shot(&mut app, "hidrolik-tasma");
        send(&mut app, W::Hydraulic(0, "12".into()));
        shot(&mut app, "hidrolik");
        send(&mut app, W::Page(2));
        for (k, v) in [(0, "60"), (1, "3"), (2, "12"), (4, "0.8"), (5, "1.2"), (6, "1.5"), (7, "1.8")] {
            send(&mut app, W::Flood(k, v.into()));
        }
        shot(&mut app, "kapanma");
        send(&mut app, W::Hydraulic(5, "10".into()));
        shot(&mut app, "kapanma-dar-cukur");
        send(&mut app, W::Page(0));
        send(&mut app, W::Choose(Some("m-2".into())));
        shot(&mut app, "boru");
        send(&mut app, W::Layer("korkuluk".into()));
        send(&mut app, W::Draw);
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "plan");
    }
}
