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
