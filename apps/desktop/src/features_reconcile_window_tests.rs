//! Adayları uzlaştır (features_reconcile_window.rs, docs/adr/1760) on the shared cases' drawing of
//! `map.features.reconcile`: the window guesses the LiDAR and orthophoto layers, its plan is the shared case's; a single's
//! Kabul and a conflict's correction make them written; Uzlaştır writes in one undo step and marks the candidates, so a
//! second run writes nothing; Harita offers it, GIS and Mimari do not. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::features_reconcile_window::Event as R;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/map.features.reconcile.json")).expect("cases")
}

fn opened(program: Option<Module>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn statuses(app: &App) -> Vec<(String, bool)> {
    let a = app.features_reconcile.answer.clone().expect("an answer").expect("reconciled");
    a.groups.iter().map(|g| (g.status.clone(), g.writes)).collect()
}

#[test]
fn the_window_plans_as_the_shared_case_and_uzlastir_writes_once() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.featuresReconcile"));
    assert_eq!(app.dialog, Some(Dialog::FeaturesReconcile));
    assert_eq!(app.features_reconcile.sources[0].0, "aday-lidar");
    assert_eq!(app.features_reconcile.sources[1].0, "aday-orto");
    let a = app.features_reconcile.answer.clone().expect("an answer").expect("reconciled");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    let got: Value = serde_json::to_value(&a.groups).expect("json");
    assert_eq!(got.as_array().map(Vec::len), want["groups"].as_array().map(Vec::len));
    for (g, w) in a.groups.iter().zip(want["groups"].as_array().expect("groups")) {
        assert_eq!(Value::from(g.status.clone()), w["status"]);
        assert_eq!(Value::from(g.writes), w["writes"]);
    }
    // The single manhole accepted and the conflict made a pole: both written then.
    let single = a.groups.iter().position(|g| g.status == "single").expect("a single");
    let conflict = a.groups.iter().position(|g| g.status == "conflict").expect("a conflict");
    let _ = app.update(Message::FeaturesReconcile(R::Accept(single)));
    let _ = app.update(Message::FeaturesReconcile(R::Correct(conflict, "direk".into())));
    let now = statuses(&app);
    assert!(now.iter().filter(|(_, w)| *w).count() >= 5, "{now:?}");
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::FeaturesReconcile(R::Run));
    let after = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    assert!(after > before, "assets written");
    // Run again: every candidate written or matched is reviewed now; nothing more is written.
    assert!(statuses(&app).iter().all(|(_, w)| !w), "{:?}", statuses(&app));
    let _ = app.update(Message::Run("edit.undo"));
    let back = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    assert_eq!(back, before, "one undo step");
}

#[test]
fn reddet_rejects_a_group_s_candidates() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.featuresReconcile"));
    let a = app.features_reconcile.answer.clone().expect("an answer").expect("reconciled");
    let single = a.groups.iter().position(|g| g.status == "single").expect("a single");
    let _ = app.update(Message::FeaturesReconcile(R::Reject(single)));
    let a = app.features_reconcile.answer.clone().expect("an answer").expect("reconciled");
    assert!(a.candidates.iter().any(|c| c.status == "rejected"));
}

#[test]
fn harita_offers_it_gis_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("map.featuresReconcile"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/aday-uzlastirma-1760/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib features_reconcile_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/aday-uzlastirma-1760");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("map.featuresReconcile"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-plan-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::FeaturesReconcile(R::Run));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-uzlastirildi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::FeaturesReconcile(R::Close));
        let _ = app.update(Message::RibbonTab("survey"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("3-serit-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
