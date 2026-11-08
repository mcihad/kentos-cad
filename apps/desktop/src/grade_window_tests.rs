//! Eğimle kot ver (grade_window.rs, docs/adr/1290): the shared cases' polyline (a straight and an arc edge) selected,
//! the window previews 2 % from its own start elevation; Uygula writes one undo step; by end elevation the grade
//! follows; a text selected is refused; Harita and Yol offer it, GIS and Mimari do not. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::grade_window::Event as G;

fn opened(program: Option<Module>) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.entities.grade.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn zs(app: &App) -> Vec<Option<f64>> {
    match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_commands::Entity::Polyline(p)) => p.zs.clone().unwrap_or_default(),
        _ => panic!("the polyline"),
    }
}

#[test]
fn a_grade_previews_and_writes_one_step() {
    let mut app = opened(Some(Module::Map));
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("tool.grade"));
    assert_eq!(app.dialog, Some(Dialog::Grade));
    let p = app.grade.plan.clone().expect("a plan").expect("previewed");
    assert!((p.grade - 0.02).abs() < 1e-12 && (p.start - 850.0).abs() < 1e-12);
    let _ = app.update(Message::Grade(G::Apply));
    let z = zs(&app);
    assert!((z[4].expect("graded") - (850.0 + 0.02 * p.length)).abs() < 1e-9);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(zs(&app)[4], None, "one step");
    // By the end's elevation: the grade follows.
    let _ = app.update(Message::Run("tool.grade"));
    let _ = app.update(Message::Grade(G::ByEnd(true)));
    let _ = app.update(Message::Grade(G::End(851.0)));
    let p = app.grade.plan.clone().expect("a plan").expect("previewed");
    assert!((p.grade - 1.0 / p.length).abs() < 1e-12);
    // A text is no path.
    let _ = app.update(Message::Grade(G::Close));
    app.selection.set([Slot(5)]);
    let _ = app.update(Message::Run("tool.grade"));
    assert_ne!(app.dialog, Some(Dialog::Grade));
}

#[test]
fn harita_and_yol_offer_it_gis_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Road, true), (Module::Gis, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("tool.grade"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/egimle-kot-1290/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egimle-kot-1290");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.selection.set([Slot(1)]);
        app.follow_selection_tab();
        let _ = app.update(Message::Run("tool.grade"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("onizleme-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// Sahneden seç (docs/adr/2180): two clicks take the range, each the path's vertex nearest it within the pick
/// aperture; the window opens again with the range previewed; a click by no vertex is said, Esc goes back.
#[test]
fn two_clicks_on_the_drawing_take_the_range() {
    let mut app = opened(Some(Module::Map));
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("tool.grade"));
    let pts = match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_commands::Entity::Polyline(p)) => p.pts.clone(),
        _ => panic!("the polyline"),
    };
    let near = |i: usize| Some(kentos_interaction::Vec2 { x: pts[i].x + 0.01, y: pts[i].y - 0.01 });
    let _ = app.update(Message::Grade(G::Pick));
    assert_eq!((app.dialog, app.grade.picking), (None, Some(crate::grade_window::Pick::Start)));
    assert!(app.grade_picked(near(1)));
    assert_eq!((app.grade.from, app.grade.picking), (2.0, Some(crate::grade_window::Pick::End)), "the second click awaited");
    assert!(app.grade_picked(near(3)));
    assert_eq!((app.grade.from, app.grade.to, app.dialog), (2.0, 4.0, Some(Dialog::Grade)));
    let p = app.grade.plan.clone().expect("a plan").expect("previewed");
    assert_eq!(p.zs.len(), 3, "vertices 2 to 4");
    // A click by no vertex: said, the range kept; Esc: back to the window.
    let _ = app.update(Message::Grade(G::Pick));
    assert!(app.grade_picked(Some(kentos_interaction::Vec2 { x: pts[0].x + 500.0, y: pts[0].y + 500.0 })));
    assert_eq!((app.grade.from, app.dialog), (2.0, Some(Dialog::Grade)));
    let _ = app.update(Message::Grade(G::Pick));
    assert!(app.grade_picked(None));
    assert_eq!((app.grade.from, app.grade.to, app.dialog), (2.0, 4.0, Some(Dialog::Grade)));
    // Nothing awaited: another window's point.
    assert!(!app.grade_picked(near(0)));
}

/// The window after Sahneden seç took vertices 2 to 4, light wide, `.run/gui-evidence/egimle-kot-2180/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_window_tests::pick_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn pick_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egimle-kot-2180");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1440.0, 900.0);
    let mut app = opened(Some(Module::Map));
    let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from("light"))]);
    app.apply_settings();
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    app.selection.set([Slot(1)]);
    app.follow_selection_tab();
    let _ = app.update(Message::Run("tool.grade"));
    let pts = match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_commands::Entity::Polyline(p)) => p.pts.clone(),
        _ => panic!("the polyline"),
    };
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    // The first click awaited: the drawing with the prompt.
    let _ = app.update(Message::Grade(G::Pick));
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("1-baslangic-koseye-tiklayin-acik-genis.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
    let _ = app.grade_picked(Some(kentos_interaction::Vec2 { x: pts[1].x, y: pts[1].y }));
    let _ = app.grade_picked(Some(kentos_interaction::Vec2 { x: pts[3].x, y: pts[3].y }));
    // A real click ends its pick step before the point arrives; called directly here, the step is ended as it would be.
    let _ = app.with_tool(|s, cx| s.cancel(cx));
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("2-aralik-secildi-acik-genis.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}
