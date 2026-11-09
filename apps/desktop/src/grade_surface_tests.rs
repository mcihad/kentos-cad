//! Eğimle kot ver's Hedef Yüzeyden (grade_surface.rs, docs/adr/4180) on the shared cases' drawing
//! (`fixtures/commands/v1/cad.entities.drape.json`: the surface “Zemin” of four corners, its diagonal AC): the area with a
//! hole and Ara kırık noktaları preview the shared case's counts and Uygula writes the case's vertices in one undo
//! step; without break points only the heights change; a drawing with no surface says so and Uygula stays off. And the
//! pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::grade_window::{Event as G, Target};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.entities.drape.json")).expect("cases")
}

fn opened(setup: &Value) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The object in `slot` selected, Eğimle kot ver open on Yüzeyden.
fn on_surface(slot: u32) -> App {
    let mut app = opened(&cases()["setup"]);
    app.selection.set([Slot(slot)]);
    let _ = app.update(Message::Run("tool.grade"));
    let _ = app.update(Message::Grade(G::Target(Target::Surface)));
    app
}

fn case_step(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name))).expect("the case")["steps"][0].clone()
}

/// The area with a hole (slot 9): with Ara kırık noktaları the preview says the case's 8 vertices and 4 break points;
/// Uygula writes the case's rings in one undo step.
#[test]
fn yuzeyden_previews_and_writes_the_shared_case() {
    let mut app = on_surface(9);
    assert_eq!(app.dialog, Some(Dialog::Grade));
    assert_eq!(app.grade.surface, "t-1", "the project's first surface");
    let _ = app.update(Message::Grade(G::Breaks(true)));
    let plan = app.grade.draped.clone().expect("a plan").expect("planned");
    assert_eq!((plan.set, plan.outside, plan.inserted), (8, 0, 4));
    let step = case_step("Ara kırık noktalarıyla");
    let want = &step["expect"]["fields"]["9"];
    let _ = app.update(Message::Grade(G::Apply));
    assert_eq!(app.dialog, None);
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Polygon(p)) = doc.model.get(Slot(9)) else { panic!("an area") };
    let got = serde_json::to_value(p).expect("json");
    for key in ["pts", "zs", "holes"] {
        let (a, b) = (&got[key], &want[key]);
        assert_eq!(a.to_string().len() > 2, true, "{key}");
        let near = |a: &Value, b: &Value| -> bool {
            match (a, b) {
                (Value::Number(x), Value::Number(y)) => (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 1e-6,
                (Value::Array(x), Value::Array(y)) => x.len() == y.len(),
                (Value::Object(_), Value::Object(_)) => true,
                _ => a == b,
            }
        };
        assert!(near(a, b), "{key}: {a} ≠ {b}");
    }
    assert_eq!(p.pts.len(), 6);
    let said = app.log.lines().last().map(|l| l.text.clone()).unwrap_or_default();
    assert_eq!(said, "Yüzeyden kot verildi: 8 köşe yüzeyden kot alır; 4 ara kırık noktası eklenir (tek adımda geri alınır).");
    let _ = app.update(Message::Run("edit.undo"));
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Polygon(p)) = doc.model.get(Slot(9)) else { panic!("an area") };
    assert_eq!((p.pts.len(), p.zs.is_none()), (4, true), "one undo step");
}

/// Without break points only the heights change; the polyline's three vertices take the surface's.
#[test]
fn without_break_points_only_the_heights_change() {
    let mut app = on_surface(8);
    let plan = app.grade.draped.clone().expect("a plan").expect("planned");
    assert_eq!((plan.set, plan.inserted), (3, 0));
    let _ = app.update(Message::Grade(G::Apply));
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Polyline(p)) = doc.model.get(Slot(8)) else { panic!("a polyline") };
    assert_eq!(p.pts.len(), 3);
    let zs: Vec<f64> = p.zs.clone().expect("heights").into_iter().flatten().collect();
    // ACD at (10, 50), ABC at (60, 30) and (90, 10).
    for (got, want) in zs.iter().zip([100.0 + 0.072 * 10.0 + 0.06 * 50.0, 100.0 + 0.04 * 60.0 + 0.1 * 30.0, 100.0 + 0.04 * 90.0 + 0.1 * 10.0]) {
        assert!((got - want).abs() < 1e-9, "{zs:?}");
    }
}

/// A drawing with no surface: the window says so and Uygula stays off.
#[test]
fn no_surface_is_said() {
    let mut setup = cases()["setup"].clone();
    setup["settings"]["road"] = serde_json::json!({"surfaces": []});
    let mut app = opened(&setup);
    app.selection.set([Slot(8)]);
    let _ = app.update(Message::Run("tool.grade"));
    let _ = app.update(Message::Grade(G::Target(Target::Surface)));
    assert!(app.grade.surface.is_empty());
    assert!(app.grade.draped.is_none());
}

/// The window on Yüzeyden with break points, light wide and dark narrow, `.run/gui-evidence/yuzeyden-kot-4180/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_surface_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yuzeyden-kot-4180");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened(&cases()["setup"]);
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.selection.set([Slot(9)]);
        let _ = app.update(Message::Run("tool.grade"));
        let _ = app.update(Message::Grade(G::Target(Target::Surface)));
        let _ = app.update(Message::Grade(G::Breaks(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
