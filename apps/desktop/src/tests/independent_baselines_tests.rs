//! A session's independent baselines on the desktop (survey_network_window.rs; plan/map-cad.md MAP-03, docs/adr/5320),
//! on the shared case of a three-receiver session (bo-1: bz-1, bz-3, bz-5 close a triangle): Bazlar says which are
//! independent and which dependent, Oturumlar lists the session's blocks, Bağımsız bazları seç takes the dependent one
//! (bz-3, the longest) out of the adjustment in one undo step. And the pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::survey_network_window::{Event as N, View};

const TRIANGLE: &str = "Bağımsız bazlar: üç alıcılı bo-1";

fn opened() -> App {
    crate::survey_epoch_tests::grids();
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.edit.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(TRIANGLE)))
        .expect("the case")
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    assert_eq!(app.dialog, Some(Dialog::SurveyNetwork));
    app
}

fn send(app: &mut App, e: N) {
    let _ = app.update(Message::SurveyNetwork(e));
}

fn adjust_of(app: &App, id: &str) -> bool {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model.settings().map_survey.as_ref().and_then(|s| s.baseline(id)).expect("the baseline").adjust
}

#[test]
fn the_dependent_baseline_goes_out_in_one_step() {
    let mut app = opened();
    let sv = app.document.as_ref().and_then(|d| d.model.settings().map_survey.clone()).expect("an archive");
    let (free, tied) = kentos_native_application::map_survey::independent_baselines(&sv, "bo-1").expect("bo-1");
    assert_eq!((free, tied.clone()), (vec!["bz-5".to_owned(), "bz-1".to_owned()], vec!["bz-3".to_owned()]));
    assert!(adjust_of(&app, "bz-3"));
    send(&mut app, N::View(View::Sessions));
    send(&mut app, N::Independent);
    assert!(!adjust_of(&app, "bz-3"), "the dependent one is out");
    assert!(adjust_of(&app, "bz-1") && adjust_of(&app, "bz-5"));
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(doc.model.undo().as_deref(), Some("Bağımsız bazlar"));
    assert!(adjust_of(&app, "bz-3"), "undone");
}

/// `.run/gui-evidence/bagimsiz-baz-5320/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib independent_baselines_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bagimsiz-baz-5320");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, w, h, name) in [("light", 1440.0, 900.0, "acik-genis"), ("dark", 1100.0, 760.0, "koyu-dar")] {
        for (shot, view, apply) in [("oturumlar", View::Sessions, false), ("bazlar", View::Baselines, true)] {
            let mut app = opened();
            let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
            app.apply_settings();
            if apply {
                send(&mut app, N::Independent);
            }
            send(&mut app, N::View(view));
            app.follow.flash = None;
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            snapshot.operate(app.view(), Box::new(crate::files_testing::SnapAll));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{shot}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
