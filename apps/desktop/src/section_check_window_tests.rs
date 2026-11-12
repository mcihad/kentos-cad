//! Docs/adr/5120 on the desktop: Pafta eğrileri's Izgara (63/2) turns and shifts the built-up grid as the shared case
//! does; Kesit kontrolü, opened from the selection (the two farthest points the ends), answers what the shared case of
//! `road.terrains.sectioncheck` answers, and a corridor puts the far point off the line. And their pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::section_check_window::Event as K;
use crate::sheet_contours_window::Event as E;

fn file(name: &str) -> Value {
    let text = match name {
        "section" => include_str!("../../../fixtures/commands/v1/road.terrains.sectioncheck.json"),
        _ => include_str!("../../../fixtures/commands/v1/road.terrains.sheetcontours.json"),
    };
    serde_json::from_str(text).expect("cases")
}

fn opened(setup: Value) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn output(name: &str, start: &str) -> Value {
    file(name)["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(start))).expect(start)
        ["steps"][0]["result"]["output"]
        .clone()
}

/// The section's slots in the shared drawing: A and B (the ends), K1–K6, K10 (3 m off the line).
const A: u32 = 10;
const B: u32 = 11;
const POINTS: [u32; 6] = [14, 15, 16, 17, 18, 19];
const OFF: u32 = 23;

fn turned(app: &mut App) {
    let _ = app.update(Message::Run("terrain.sheetContours"));
    for d in ["yol", "sev"] {
        let _ = app.update(Message::SheetContours(E::Detail(d.into(), false)));
    }
    let _ = app.update(Message::SheetContours(E::Built(true)));
    for e in [E::GridY("7".into()), E::GridX("3".into()), E::GridAngle("45".into())] {
        let _ = app.update(Message::SheetContours(e));
    }
}

/// Izgara (63/2): Y 7, X 3 and 45 (the project's degrees, counter-clockwise in a CAD project) give the shared case's
/// heights, the grid's places on the turned axes.
#[test]
fn the_turned_grid_is_the_shared_case_s() {
    let mut app = opened(file("sheet")["setup"].clone());
    turned(&mut app);
    let r = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    let want = output("sheet", "m. 63(2) ızgarası 50 g döndürülmüş");
    let spots = want["spots"].as_array().expect("spots");
    assert_eq!(r.spots.len(), spots.len());
    for (got, w) in r.spots.iter().zip(spots) {
        assert_eq!(got.kind.as_str(), w["kind"].as_str().expect("kind"));
        assert!((got.x - w["x"].as_f64().expect("x")).abs() < 1e-9 && (got.y - w["y"].as_f64().expect("y")).abs() < 1e-9);
    }
    // A typed angle that is not a number is said, nothing planned.
    let _ = app.update(Message::SheetContours(E::GridAngle("elli".into())));
    let why = app.sheet_contours.answer.clone().expect("an answer").expect_err("refused");
    assert!(why.contains("açısı “elli” bir sayı değil"), "{why}");
}

fn section(app: &mut App, extra: &[u32]) {
    app.selection.set([A, B].iter().chain(POINTS.iter()).chain(extra).map(|s| Slot(*s)));
    let _ = app.update(Message::SectionCheck(K::Open));
}

/// Kesit kontrolü from the selection: the farthest two are the ends, the contour layer guessed from its name; the answer
/// is the shared case's, the table one row a point along the section.
#[test]
fn the_section_from_the_selection_is_the_shared_case_s() {
    let mut app = opened(file("section")["setup"].clone());
    section(&mut app, &[]);
    assert_eq!(app.dialog, Some(Dialog::SectionCheck));
    assert_eq!(app.section_check.contours, "esyukselti");
    assert_eq!(app.section_check.points.len(), 6);
    let (r, warnings) = app.section_check.answer.clone().expect("an answer").expect("checked");
    let want = output("section", "Kesit: altı nokta");
    assert!(warnings.is_empty());
    assert_eq!((r.compared, r.within), (5, Some(true)));
    assert!((r.rms.expect("rms") - want["rms"].as_f64().expect("rms")).abs() < 1e-12);
    assert!((r.limit - want["limit"].as_f64().expect("limit")).abs() < 1e-12);
    let alongs: Vec<f64> = r.points.iter().map(|p| p.along).collect();
    let wanted: Vec<f64> = want["points"].as_array().expect("points").iter().map(|p| p["along"].as_f64().expect("along")).collect();
    assert_eq!(alongs, wanted);
}

/// A point 3 m off the line: compared without a corridor, off the line with 2 m.
#[test]
fn a_corridor_puts_the_far_point_off_the_line() {
    let mut app = opened(file("section")["setup"].clone());
    section(&mut app, &[OFF]);
    let state = |app: &App| {
        let (r, _) = app.section_check.answer.clone().expect("an answer").expect("checked");
        r.points.iter().find(|p| (p.offset - 3.0).abs() < 1e-9).expect("the far point").state.clone()
    };
    assert_eq!(state(&app), "compared");
    let _ = app.update(Message::SectionCheck(K::Corridor("2".into())));
    assert_eq!(state(&app), "offLine");
    let _ = app.update(Message::SectionCheck(K::Corridor("iki".into())));
    assert!(app.section_check.answer.clone().expect("an answer").is_err());
}

/// Too few points selected: said, no window.
#[test]
fn too_few_points_are_said() {
    let mut app = opened(file("section")["setup"].clone());
    app.selection.set([Slot(A), Slot(B)]);
    let _ = app.update(Message::SectionCheck(K::Open));
    assert_eq!(app.dialog, None);
}

/// `.run/gui-evidence/kesit-kontrolu-5120/`: Pafta eğrileri with the turned grid, the grid drawn, Kesit kontrolü within
/// and over its limits; light wide and dark narrow.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib section_check_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kesit-kontrolu-5120");
    std::fs::create_dir_all(&out).expect("a folder");
    type Step = fn(&mut App);
    let grid: Step = turned;
    let drawn: Step = |app| {
        turned(app);
        let _ = app.update(Message::SheetContours(E::Draw));
        let b = kentos_commands::Bounds { min_x: 175.0, min_y: 95.0, max_x: 345.0, max_y: 225.0 };
        app.navigating(|a| a.zoom_to_bounds(&b));
    };
    let within: Step = |app| section(app, &[OFF]);
    let over: Step = |app| {
        app.selection.set([A, B, 14, 24, 16, 17, 18, 19].iter().map(|s| Slot(*s)));
        let _ = app.update(Message::SectionCheck(K::Open));
    };
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (setup, step, f) in [
            (file("sheet")["setup"].clone(), grid, "1-izgara"),
            (file("sheet")["setup"].clone(), drawn, "2-donen-izgara"),
            (file("section")["setup"].clone(), within, "3-kesit"),
            (file("section")["setup"].clone(), over, "4-kesit-siniri-asiyor"),
        ] {
            let mut app = opened(setup);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            step(&mut app);
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{f}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}
