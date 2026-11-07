//! Pafta eğrileri (sheet_contours_window.rs, docs/adr/1820) on the shared cases' drawing of
//! `road.terrains.sheetcontours`: the window guesses the contour layer and the detail layers, its plan is the shared
//! case's; a rule unticked changes the plan; Çiz writes the lines and heights in one undo step; Harita offers it. And
//! its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::sheet_contours_window::Event as E;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.sheetcontours.json")).expect("cases")
}

fn opened(program: Option<Module>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn the_plan_is_the_shared_case_s_and_draw_writes_in_one_step() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("terrain.sheetContours"));
    assert_eq!(app.dialog, Some(Dialog::SheetContours));
    let s = &app.sheet_contours;
    assert_eq!(s.layer, "esyukselti");
    assert_eq!(s.details, vec!["bina".to_owned(), "yol".to_owned(), "sev".to_owned()]);
    let r = s.answer.clone().expect("an answer").expect("planned");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    assert_eq!(Some(r.lines.len()), want["lines"].as_array().map(Vec::len));
    assert_eq!(Some(r.labels.len()), want["labels"].as_array().map(Vec::len));
    assert!((r.cut - want["cut"].as_f64().expect("cut")).abs() < 1e-9);
    // The auxiliary lines unticked: none of them in the plan.
    let _ = app.update(Message::SheetContours(E::Auxiliary(false)));
    let off = app.sheet_contours.answer.clone().expect("an answer").expect("planned");
    assert!(off.lines.iter().all(|l| l.kind != "auxiliary") && off.lines.len() < r.lines.len());
    let _ = app.update(Message::SheetContours(E::Auxiliary(true)));
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::SheetContours(E::Draw));
    let after = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    assert_eq!(after, before + r.lines.len() + r.labels.len());
    assert_eq!(app.dialog, None);
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Pafta eş yükselti eğrileri"));
}

#[test]
fn harita_offers_it() {
    assert!(opened(Some(Module::Map)).offers("terrain.sheetContours"));
}

/// `.run/gui-evidence/pafta-egrileri-1820/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib sheet_contours_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/pafta-egrileri-1820");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("terrain.sheetContours"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::SheetContours(E::Draw));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-cizildi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
    // Where it sits: Arazi › Yüzey.
    let mut app = opened(Some(Module::Map));
    let size = Size::new(1600.0, 900.0);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::RibbonTab("terrain"));
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("3-serit-arazi.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}
