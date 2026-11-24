//! Kaplamaların yanıcılığı (docs/adr/8091; Binaların Yangından Korunması Hakkında Yönetmelik 26/2, 26/4, Ek-2/Ç) on a
//! four-flat building's 1. kat, worked by hand: an Alçıpan suspended ceiling (CEILING) and a Parke floor covering
//! (FLOORING), both materials of the project. Undeclared, both are missing data; Malzemeler's Yanıcılık sınıfı written
//! through `cad.arch.performance` makes the ceiling D-s2,d0 (normal alevlenici: 26/4 asks zor alevlenici, it fails),
//! then A2-s1,d0 (zor yanıcı, it holds); the parquet Dfl-s1 holds 26/2's normal alevlenici.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib covering_reaction_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S};
use kentos_launch::Module;
use serde_json::{Value, json};
use crate::arch::ArchState;

use crate::app::{App, Message};
use crate::arch;
use crate::materials_window::Event;
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

fn run(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn p(x: f64, y: f64) -> Value {
    json!({"x": x, "y": y})
}

/// The building: two storeys, four flats, the two materials, 1. kat's ceiling and floor covering; applied on 2.7.2026.
fn building() -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            {"id": "k00", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
            {"id": "k01", "name": "1. kat", "elevation": 3.0, "height": 3.0}
        ] } }),
    );
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "nature": "Daire"}, {"id": "d2", "number": "2", "nature": "Daire"},
            {"id": "d3", "number": "3", "nature": "Daire"}, {"id": "d4", "number": "4", "nature": "Daire"}
        ]}}),
    );
    for name in ["Alçıpan", "Parke"] {
        run(&mut app, "cad.materials.edit", json!({"operation": {"kind": "define", "name": name, "change": {}}}));
    }
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let rect = vec![p(0.0, 0.0), p(8.0, 0.0), p(8.0, 5.0), p(0.0, 5.0)];
    run(
        &mut app,
        "cad.slabs.create",
        json!({"layerId": layer, "slabs": [
            {"parts": [{"outer": {"pts": rect}}], "thickness": 0.0125, "offset": 2.6, "material": "Alçıpan",
             "arch": {"level": "k01"}, "covering": {"predefined": "CEILING", "name": "Asma tavan"}},
            {"parts": [{"outer": {"pts": rect}}], "thickness": 0.015, "material": "Parke",
             "arch": {"level": "k01"}, "covering": {"predefined": "FLOORING", "name": "Parke"}}
        ]}),
    );
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    app
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.modules.arch().arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// 26/2's and 26/4's rows and the coverings' findings (code, state).
#[allow(clippy::type_complexity)]
fn said(app: &App) -> (Option<O>, Option<O>, Vec<(String, Option<S>, String)>) {
    let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
    let row = |rule: &str| set.rules.iter().find(|r| r.rule == rule).map(|r| r.state);
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code == "bykhy_floor_covering_reaction" || f.code == "bykhy_ceiling_reaction")
        .map(|f| (f.code.clone(), f.state, f.message.clone()))
        .collect();
    (row("BYKHY 26/2"), row("BYKHY 26/4"), found)
}

/// Malzemeler open, `name` chosen, its Yanıcılık sınıfı typed `class` and applied.
fn declared(app: &mut App, name: &str, class: &str) {
    let _ = app.update(Message::Run("arch.materials"));
    let at = app.modules.arch().materials.as_ref().expect("the window").list.iter().position(|(m, _)| m.name == name).expect(name);
    let _ = app.update(Message::from(arch::Event::Materials(Event::Choose(at))));
    let _ = app.update(Message::from(arch::Event::Materials(Event::Reaction(class.to_owned()))));
    let _ = app.update(Message::from(arch::Event::Materials(Event::Apply)));
}

#[test]
fn the_coverings_materials_reaction_classes_are_read_by_26_2_and_26_4() {
    let mut app = building();
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let kinds: Vec<(&str, Option<&str>)> =
        facts.coverings.iter().map(|c| (c.predefined.as_str(), c.material.as_deref())).collect();
    assert_eq!(kinds.len(), 2, "{kinds:?}");
    assert!(kinds.contains(&("CEILING", Some("Alçıpan"))) && kinds.contains(&("FLOORING", Some("Parke"))), "{kinds:?}");
    checked(&mut app);
    let (floor, ceiling, found) = said(&app);
    assert_eq!((floor, ceiling), (Some(O::MissingData), Some(O::MissingData)), "{found:#?}");
    assert!(found.iter().all(|(_, s, _)| *s == Some(S::MissingData)), "{found:#?}");
    // The ceiling D-s2,d0: normal alevlenici, under 26/4's zor alevlenici; the parquet Dfl-s1.
    declared(&mut app, "Alçıpan", "D-s2,d0");
    declared(&mut app, "Parke", "Dfl-s1");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let materials = &doc.settings().arch.as_ref().expect("Mimari").materials;
    let class = |name: &str| materials.iter().find(|m| m.name == name).and_then(|m| m.reaction.clone()).map(|d| d.value);
    assert_eq!((class("Alçıpan").as_deref(), class("Parke").as_deref()), (Some("D-s2,d0"), Some("Dfl-s1")));
    assert_eq!(doc.undo_labels().first().map(String::as_str), Some("Yangın ve performans"));
    checked(&mut app);
    let (floor, ceiling, found) = said(&app);
    assert_eq!((floor, ceiling), (Some(O::Passed), Some(O::Failed)), "{found:#?}");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].2.starts_with("1. kat: tavan kaplaması: malzemesi “Alçıpan” D-s2,d0, normal alevlenici; 26/4 en az zor alevlenici ister"),
        "{found:#?}"
    );
    // An unreadable class is refused before Uygula, the window says why.
    let _ = app.update(Message::Run("arch.materials"));
    let _ = app.update(Message::from(arch::Event::Materials(Event::Reaction("A3".to_owned()))));
    let w = app.modules.arch().materials.as_ref().expect("the window");
    assert!(w.declared.as_deref().is_some_and(|why| why.contains("TS EN 13501-1")), "{:?}", w.declared);
    assert!(app.materials_unapplied(), "a class typed is a change Uygula writes");
    let _ = app.update(Message::from(arch::Event::Materials(Event::Close)));
    // A2-s1,d0: zor yanıcı.
    declared(&mut app, "Alçıpan", "A2-s1,d0");
    checked(&mut app);
    let (_, ceiling, found) = said(&app);
    assert_eq!(ceiling, Some(O::Passed), "{found:#?}");
}

/// Malzemeler with Alçıpan's Yanıcılık sınıfı, and Denetim's 26/4 finding, light and dark;
/// `.run/gui-evidence/yanicilik-8091/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yanicilik-8091");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = building();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        declared(&mut app, "Parke", "Dfl-s1");
        // The window open on Alçıpan, its class typed, not yet applied.
        let _ = app.update(Message::Run("arch.materials"));
        let at = app.modules.arch().materials.as_ref().expect("the window").list.iter().position(|(m, _)| m.name == "Alçıpan");
        let _ = app.update(Message::from(arch::Event::Materials(Event::Choose(at.expect("Alçıpan")))));
        let _ = app.update(Message::from(arch::Event::Materials(Event::Reaction("D-s2,d0".to_owned()))));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-malzemeler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::from(arch::Event::Materials(Event::Apply)));
        let _ = app.update(Message::from(arch::Event::Materials(Event::Close)));
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_ceiling_reaction");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
