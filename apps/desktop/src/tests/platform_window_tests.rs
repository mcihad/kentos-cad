//! Platform (platform_window.rs, plan/road.md Y14.3): the window grades the pad as it is typed, balances it, saves and
//! draws it, each one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::platform_window::{Event as W, corners_of, corners_text};
use crate::road;

/// A road project with `road.platforms.edit`'s drawing: a sloping plane terrain and two pads.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.platforms.edit.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::Platforms(e)));
}

#[test]
fn corners_read_as_typed() {
    assert_eq!(corners_of("0,0; 40,0\n40,30;0,30").expect("reads"), vec![[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]]);
    assert!(corners_of("0,0; 40").expect_err("one number").contains("Y,X"));
    assert!(corners_of("0,0; a,1").expect_err("not a number").contains("sayı"));
    assert_eq!(corners_text(&[[1.5, 2.0], [3.25, -4.0]]), "1.5,2; 3.25,-4");
}

#[test]
fn the_window_grades_balances_saves_and_draws_the_pad() {
    let mut app = app();
    let _ = app.update(Message::Run("platform.editor"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Platforms))
    );
    let w = app.road.platform_window.as_ref().expect("open");
    assert_eq!(w.chosen.as_deref(), Some("p-1"));
    let r = w
        .preview
        .report
        .clone()
        .unwrap_or_else(|| panic!("grades: {:?}", w.preview.why));
    let fill = r.fill.expect("closed");
    // A flatter fill slope runs further out and fills more.
    send(&mut app, W::Field(8, "3".into()));
    let more = app
        .road
        .platform_window
        .as_ref()
        .and_then(|w| w.preview.report.clone())
        .expect("grades");
    assert!(
        more.fill.expect("closed") > fill,
        "{fill} → {:?}",
        more.fill
    );
    // A balance in the bracket typed, put in the base's height.
    for (k, v) in [(12, "-6"), (13, "2"), (14, "0.5")] {
        send(&mut app, W::Field(k, v.into()));
    }
    send(&mut app, W::Balance);
    let b = match app
        .road
        .platform_window
        .as_ref()
        .and_then(|w| w.preview.balance.clone())
    {
        Some(Ok(b)) => b,
        other => panic!("balanced: {other:?}"),
    };
    assert!(b.converged && b.net.abs() <= 0.5, "{b:?}");
    send(&mut app, W::Apply);
    let w = app.road.platform_window.as_ref().expect("open");
    let balanced = w.preview.report.clone().expect("grades");
    let (c, f) = (
        balanced.cut.expect("closed"),
        balanced.fill.expect("closed"),
    );
    assert!((c - f).abs() <= 1.0, "{c} {f}");
    // Nothing written until Kaydet.
    assert_eq!(app.document.as_ref().expect("a drawing").model.road_platforms()[0].base[2], 104.0);
    send(&mut app, W::Save);
    let saved = app.document.as_ref().expect("a drawing").model.road_platforms()[0].clone();
    assert_eq!(saved.fill, 3.0);
    assert!((saved.base[2] - b.elevation).abs() < 1e-3);
    assert_eq!(saved.revision, 2);
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::Layer("platform".into()));
    send(&mut app, W::Draw);
    assert!(app.road.platform_window.is_none());
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
fn a_new_pad_needs_its_slopes_and_takes_its_corners_from_a_closed_area() {
    let mut app = app();
    let _ = app.update(Message::Run("platform.editor"));
    send(&mut app, W::Choose(None));
    let why = app
        .road
        .platform_window
        .as_ref()
        .and_then(|w| w.preview.why.clone())
        .expect("no corners yet");
    assert!(why.contains("Köşeler"), "{why}");
    send(&mut app, W::Field(1, "200,0; 230,0; 230,20; 200,20".into()));
    let why = app
        .road
        .platform_window
        .as_ref()
        .and_then(|w| w.preview.why.clone())
        .expect("no base yet");
    assert!(why.contains("yazılmalı"), "{why}");
    for (k, v) in [
        (2, "200"),
        (3, "0"),
        (4, "110"),
        (7, "1.5"),
        (8, "2"),
        (9, "5"),
        (10, "20"),
        (11, "100"),
    ] {
        send(&mut app, W::Field(k, v.into()));
    }
    let w = app.road.platform_window.as_ref().expect("open");
    assert!(w.preview.report.is_some(), "{:?}", w.preview.why);
    send(&mut app, W::Save);
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .road_platforms()
            .len(),
        3
    );
    // A closed area selected gives the corners, and the base's place while none is typed.
    send(&mut app, W::Choose(None));
    let input: kentos_commands::PolygonCreate = serde_json::from_value(serde_json::json!({
        "layerId": "platform",
        "pts": [{"x": 300.0, "y": 0.0}, {"x": 320.0, "y": 0.0}, {"x": 320.0, "y": 15.0}, {"x": 300.0, "y": 15.0}],
    }))
    .expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    let made = kentos_native_application::polygon::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        input,
    );
    let slot = match made {
        kentos_commands::CommandResult::Completed { output, .. } => kentos_domain::Slot(output.id),
        other => panic!("drawn: {other:?}"),
    };
    app.selection.set([slot]);
    send(&mut app, W::FromSelection);
    let w = app.road.platform_window.as_ref().expect("open");
    assert_eq!(w.fields[1], "300,0; 320,0; 320,15; 300,15");
    assert_eq!((w.fields[2].as_str(), w.fields[3].as_str()), ("300", "0"));
}

/// The window, wide and narrow, light and dark, and the pad drawn in plan; `.run/gui-evidence/yol-y14-platform-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib platform_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y14-platform-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("platform.editor"));
        shot(&mut app, "dolgu");
        for (k, v) in [(12, "-6"), (13, "2"), (14, "0.5")] {
            send(&mut app, W::Field(k, v.into()));
        }
        send(&mut app, W::Balance);
        shot(&mut app, "dengele");
        send(&mut app, W::Apply);
        shot(&mut app, "dengeli");
        send(&mut app, W::Save);
        send(&mut app, W::Layer("platform".into()));
        send(&mut app, W::Draw);
        // The pad drawn, framed: the terrain's points reach far beyond it.
        let drawn: Vec<kentos_domain::Slot> = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .filter(|e| e.base().layer_id == "platform")
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        app.selection.set(drawn);
        let _ = app.update(Message::Run("view.zoomSelection"));
        let _ = app.update(Message::Run("edit.deselect"));
        shot(&mut app, "plan");
    }
}

/// The same drawing as a KentOS Harita project (its road settings: the surfaces and the pads only, docs/adr/1580).
fn map_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.platforms.edit.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    setup["settings"]["cadWork"] = serde_json::json!("map");
    let road = setup["settings"]["road"].clone();
    setup["settings"]["road"] = serde_json::json!({ "surfaces": road["surfaces"], "platforms": road["platforms"] });
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.program = Some(kentos_launch::Module::Map);
    app
}

/// docs/adr/1580: a Harita project grades, balances and saves a pad as a Yol project does (a square, a car park, a
/// building's platform), and its drawing saves and opens again as a .kcad of schema 197.
#[test]
fn a_map_project_grades_its_pads() {
    let mut app = map_app();
    assert!(app.offers("platform.editor") && app.offers("drainage.editor"));
    let _ = app.update(Message::Run("platform.editor"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Platforms))
    );
    let w = app.road.platform_window.as_ref().expect("open");
    let r = w
        .preview
        .report
        .clone()
        .unwrap_or_else(|| panic!("grades: {:?}", w.preview.why));
    assert!(r.fill.is_some() || r.cut.is_some(), "{r:?}");
    send(&mut app, W::Field(8, "3".into()));
    send(&mut app, W::Save);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.road_platforms()[0].fill, 3.0);
    let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("a map project with pads writes");
    let back = kentos_kcad::decode(&bytes).expect("and reads again");
    assert_eq!(back.settings.road.expect("road settings").platforms[0].fill, 3.0);
    // Drenaj reads its surface too.
    let _ = app.update(Message::Run("drainage.editor"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Drainage))
    );
}

/// Tesviye and Drenaj in KentOS Harita: the Arazi tab and the pad window; `.run/gui-evidence/harita-tesviye-1580/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib platform_window_tests::screens_map -- --ignored --nocapture
/// ```
#[test]
#[ignore = "writes pictures"]
fn screens_map() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/harita-tesviye-1580");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = map_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-arazi-sekmesi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::Run("platform.editor"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-platform-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
