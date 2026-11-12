//! Kavşak (junction_window.rs, plan/road.md Y12.2): the window reads the junction as it is typed, saves it and draws its
//! curb returns, each one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::junction_window::Event as W;
use kentos_commands::JunctionKind;

/// A road project with `road.junctions.edit`'s drawing: alignments and two junctions.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.junctions.edit.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Junctions(e));
}

#[test]
fn the_window_saves_and_draws_the_junction() {
    let mut app = app();
    let _ = app.update(Message::Run("junction.editor"));
    assert_eq!(app.dialog, Some(Dialog::Junctions));
    let w = app.junction_window.as_ref().expect("open");
    assert_eq!(w.chosen.as_deref(), Some("j-1"));
    let r = w.preview.report.clone().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert_eq!(r.returns.len(), 4);
    send(&mut app, W::Field(2, "25".into()));
    let wider = app.junction_window.as_ref().and_then(|w| w.preview.report.clone()).expect("reads");
    assert!(wider.returns[0].length > r.returns[0].length);
    send(&mut app, W::Field(7, "10".into()));
    let why = app.junction_window.as_ref().and_then(|w| w.preview.why.clone()).expect("three quadrants missing");
    assert!(why.contains("yazılmalı"), "{why}");
    for k in 8..11 {
        send(&mut app, W::Field(k, "18".into()));
    }
    send(&mut app, W::Save);
    let saved = app.document.as_ref().expect("a drawing").model.road_junctions()[0].clone();
    assert_eq!(saved.radius, 25.0);
    assert_eq!(saved.radii, Some(vec![10.0, 18.0, 18.0, 18.0]));
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::Layer("korkuluk".into()));
    send(&mut app, W::Draw);
    assert!(app.junction_window.is_none());
    assert_eq!(app.document.as_ref().expect("a drawing").model.entities().count(), before + 5);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.entities().count(), before);
}

#[test]
fn a_separated_junction_joins_nothing() {
    let mut app = app();
    let _ = app.update(Message::Run("junction.editor"));
    send(&mut app, W::Choose(None));
    send(&mut app, W::Kind(JunctionKind::Separated));
    for (k, v) in [(3, "5"), (4, "5"), (5, "3.5"), (6, "3.5")] {
        send(&mut app, W::Field(k, v.into()));
    }
    let r = app.junction_window.as_ref().and_then(|w| w.preview.report.clone()).expect("reads");
    assert!(!r.connected && r.returns.is_empty());
    send(&mut app, W::Save);
    assert_eq!(app.document.as_ref().expect("a drawing").model.road_junctions().len(), 3);
}

/// The window, wide and narrow, light and dark, and the returns drawn in plan; `.run/gui-evidence/yol-y12-kavsak-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib junction_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y12-kavsak-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("junction.editor"));
        send(&mut app, W::Field(2, "18".into()));
        shot(&mut app, "hemzemin");
        send(&mut app, W::Choose(Some("j-2".into())));
        shot(&mut app, "katli");
        send(&mut app, W::Choose(Some("j-1".into())));
        send(&mut app, W::Layer("korkuluk".into()));
        send(&mut app, W::Draw);
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "plan");
    }
}

/// `road.junctions.clearance`'s drawing: straight roads, their profiles and separated junctions.
fn clearance_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.junctions.clearance.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn cleared(app: &App) -> Result<kentos_commands::JunctionsCleared, String> {
    match app.junction_window.as_ref().and_then(|w| w.clearance.clone()) {
        Some(Ok((c, _))) => Ok(c),
        Some(Err(why)) => Err(why),
        None => Err("not read".into()),
    }
}

#[test]
fn a_separated_junction_reads_its_vertical_clearance() {
    let mut app = clearance_app();
    let before = app.document.as_ref().expect("a drawing").model.revision().to_string();
    let _ = app.update(Message::Run("junction.editor"));
    let w = app.junction_window.as_ref().expect("open");
    assert_eq!(w.chosen.as_deref(), Some("j-1"));
    // The depth is the user's: none typed, none read.
    assert!(cleared(&app).is_err_and(|why| why.contains("yazılmalı")));
    send(&mut app, W::ClearField(0, "1.2".into()));
    send(&mut app, W::ClearField(1, "1".into()));
    let c = cleared(&app).expect("reads");
    assert_eq!((c.upper, c.samples), (0, 99));
    assert!((c.least.clearance - 5.978523749999994).abs() < 1e-6 && !c.short);
    send(&mut app, W::ClearField(2, "6".into()));
    assert!(cleared(&app).expect("reads").short);
    // One profile chosen is not enough; both are.
    send(&mut app, W::ClearProfile(0, Some("b-2".into())));
    assert!(cleared(&app).is_err_and(|why| why.contains("birlikte")));
    send(&mut app, W::ClearProfile(1, Some("b-1".into())));
    assert!(cleared(&app).is_ok());
    // An at-grade junction has none.
    send(&mut app, W::Choose(Some("j-4".into())));
    assert!(app.junction_window.as_ref().expect("open").clearance.is_none());
    send(&mut app, W::Close);
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision().to_string(), before);
}

/// The clearance under the overpass and the deck too deep for it, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y17-dusey-aciklik-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib junction_window_tests::clearance_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn clearance_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = clearance_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y17-dusey-aciklik-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("junction.editor"));
        send(&mut app, W::ClearField(0, "1.2".into()));
        send(&mut app, W::ClearField(1, "0.5".into()));
        send(&mut app, W::ClearField(2, "5.5".into()));
        shot(&mut app, "ust-gecit");
        send(&mut app, W::Choose(Some("j-2".into())));
        send(&mut app, W::ClearProfile(0, Some("b-1".into())));
        send(&mut app, W::ClearProfile(1, Some("b-3".into())));
        send(&mut app, W::ClearField(2, "5".into()));
        shot(&mut app, "egik-kisa");
    }
}

/// `road.junctions.tie`'s drawing: straight roads, their profiles, superelevations and at-grade junctions.
fn tie_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.junctions.tie.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn tied(app: &App) -> Result<kentos_commands::JunctionsTied, String> {
    match app.junction_window.as_ref().and_then(|w| w.tie.clone()) {
        Some(Ok((t, _))) => Ok(t),
        Some(Err(why)) => Err(why),
        None => Err("not read".into()),
    }
}

fn pvis(app: &App, id: &str) -> Vec<String> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.road_profiles().iter().find(|p| p.id == id).map(|p| p.pvis.iter().map(|v| v.id.clone()).collect()).unwrap_or_default()
}

#[test]
fn an_at_grade_junction_ties_the_secondary_profile_to_the_crown() {
    let mut app = tie_app();
    let _ = app.update(Message::Run("junction.editor"));
    assert_eq!(app.junction_window.as_ref().expect("open").chosen.as_deref(), Some("j-1"));
    // The slopes are the user's: none typed, none read; and the secondary alignment has three profiles.
    send(&mut app, W::ClearProfile(0, Some("b-1".into())));
    send(&mut app, W::ClearProfile(1, Some("b-2".into())));
    assert!(tied(&app).is_err_and(|why| why.contains("yazılmalı")));
    send(&mut app, W::TieField(0, "-2.5".into()));
    send(&mut app, W::TieField(1, "-2.5".into()));
    let t = tied(&app).expect("reads");
    assert_eq!(t.ties.iter().map(|x| x.pvi.as_str()).collect::<Vec<_>>(), ["j-1.a", "j-1.x", "j-1.b"]);
    assert!((t.changes[1] - 0.05).abs() < 1e-9, "{t:?}");
    // The primary road's superelevation instead of the typed slopes.
    send(&mut app, W::TieSuper(Some("d-1".into())));
    assert!((tied(&app).expect("reads").ties[2].crossfall - 0.03).abs() < 1e-9);
    assert_eq!(pvis(&app, "b-2"), ["a", "v1", "v2", "v3", "b"]);
    send(&mut app, W::Tie);
    assert_eq!(pvis(&app, "b-2"), ["a", "v1", "j-1.a", "j-1.x", "j-1.b", "v3", "b"]);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(pvis(&app, "b-2"), ["a", "v1", "v2", "v3", "b"]);
}

/// The tie at the crossroads, typed slopes and the superelevation, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y17-kavsak-bagi-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib junction_window_tests::tie_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn tie_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = tie_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y17-kavsak-bagi-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("junction.editor"));
        send(&mut app, W::ClearProfile(0, Some("b-1".into())));
        send(&mut app, W::ClearProfile(1, Some("b-2".into())));
        send(&mut app, W::TieField(0, "-2.5".into()));
        send(&mut app, W::TieField(1, "-2.5".into()));
        send(&mut app, W::TieField(2, "4".into()));
        shot(&mut app, "egim");
        send(&mut app, W::TieSuper(Some("d-1".into())));
        shot(&mut app, "dever");
    }
}

/// `road.junctions.overlap`'s drawing: the junctions' drawing with corridors across the main junction.
fn overlap_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.junctions.overlap.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn the_window_reads_the_corridors_overlap() {
    let mut app = overlap_app();
    let before = app.document.as_ref().expect("a drawing").model.revision().to_string();
    let _ = app.update(Message::Run("junction.editor"));
    send(&mut app, W::OverlapMode(true));
    let read = |app: &App| app.junction_window.as_ref().and_then(|w| w.overlap.clone()).expect("asked");
    // The cell is the user's; the main alignment has three corridors.
    assert!(read(&app).is_err_and(|why| why.contains("yazılmalı")));
    send(&mut app, W::OverlapCell("1".into()));
    assert!(read(&app).is_err_and(|why| why.contains("3 koridoru")));
    send(&mut app, W::OverlapCorridor(0, Some("c-1".into())));
    send(&mut app, W::OverlapCorridor(1, Some("c-5".into())));
    let (o, warnings) = read(&app).expect("reads");
    assert_eq!((o.cells, o.primary_corridor.as_str(), o.secondary_corridor.as_str()), (346, "c-1", "c-5"));
    assert!(o.cut > 0.0 && warnings.iter().any(|w| w.code == "junction_corridors_overlap"));
    send(&mut app, W::OverlapPrimary(1));
    let (o, _) = read(&app).expect("reads");
    assert_eq!(o.secondary_corridor, "c-1");
    send(&mut app, W::OverlapMode(false));
    assert!(app.junction_window.as_ref().expect("open").overlap.is_none());
    send(&mut app, W::Close);
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision().to_string(), before);
}

/// The overlap of the main corridor and the junction's corridor, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y17-koridor-ortusme-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib junction_window_tests::overlap_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn overlap_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = overlap_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y17-koridor-ortusme-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("junction.editor"));
        send(&mut app, W::OverlapMode(true));
        send(&mut app, W::OverlapCorridor(0, Some("c-1".into())));
        send(&mut app, W::OverlapCorridor(1, Some("c-5".into())));
        send(&mut app, W::OverlapCell("1".into()));
        shot(&mut app, "ana-sahip");
    }
}
