//! Çatıların yanıcılığı (docs/adr/8134; Binaların Yangından Korunması Hakkında Yönetmelik 28/2, 28/3, Ek-2/Ç) on a
//! two-storey building's roof over 1. kat, worked by hand: kil kiremit on battens, çam kaplama tahtası, taş yünü, çam
//! mertek, each a material of the project. Undeclared, 28/2 is missing data. Malzemeler's Yanıcılık sınıfı and Dış yangın
//! sınıfı written through `cad.arch.performance`: the tiles A1 and BROOF(t1), the pine D-s2,d0, the stone wool A1. 28/2
//! holds (a hiç yanmaz covering lets its boarding be normal alevlenici); 28/3 is expert review (pine rafters in a
//! building that is not high: they fail in attached order, which the drawing does not say). Tiles of CROOF(t3) fail
//! 28/2.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib roof_fire_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S};
use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::arch;
use crate::files_testing::drive;
use crate::materials_window::Event;
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

/// The building: two storeys, the three materials, a hip roof over 1. kat in five layers; applied on 2.7.2026.
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
    for name in ["Kil kiremit", "Çam", "Taş yünü"] {
        run(&mut app, "cad.materials.edit", json!({"operation": {"kind": "define", "name": name, "change": {}}}));
    }
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    run(
        &mut app,
        "cad.roofs.create",
        json!({"layerId": layer, "roofs": [{"pts": [p(0.0, 0.0), p(8.0, 0.0), p(8.0, 5.0), p(0.0, 5.0)],
            "slopes": [33.0, 33.0, 33.0, 33.0], "overhang": 0.5, "arch": {"level": "k01"}, "layers": [
                {"kind": "covering", "thickness": 0.03, "material": "Kil kiremit"},
                {"kind": "battens", "thickness": 0.03},
                {"kind": "boarding", "thickness": 0.025, "material": "Çam"},
                {"kind": "insulation", "thickness": 0.05, "material": "Taş yünü"},
                {"kind": "rafters", "thickness": 0.125, "material": "Çam"}
            ]}]}),
    );
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    app
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// 28/2's and 28/3's rows and the roofs' findings (code, state, words).
fn said(app: &App) -> (Option<O>, Option<O>, Vec<(String, Option<S>, String)>) {
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    let row = |rule: &str| set.rules.iter().find(|r| r.rule == rule).map(|r| r.state);
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code.starts_with("bykhy_roof_"))
        .map(|f| (f.code.clone(), f.state, f.message.clone()))
        .collect();
    (row("BYKHY 28/2"), row("BYKHY 28/3"), found)
}

/// Malzemeler open, `name` chosen, its Yanıcılık sınıfı and Dış yangın sınıfı typed (none: left) and applied.
fn declared(app: &mut App, name: &str, reaction: Option<&str>, roof: Option<&str>) {
    let _ = app.update(Message::Run("arch.materials"));
    let at = app.arch.materials.as_ref().expect("the window").list.iter().position(|(m, _)| m.name == name).expect(name);
    let _ = app.update(Message::Arch(arch::Event::Materials(Event::Choose(at))));
    if let Some(r) = reaction {
        let _ = app.update(Message::Arch(arch::Event::Materials(Event::Reaction(r.to_owned()))));
    }
    if let Some(r) = roof {
        let _ = app.update(Message::Arch(arch::Event::Materials(Event::RoofClass(r.to_owned()))));
    }
    let _ = app.update(Message::Arch(arch::Event::Materials(Event::Apply)));
}

#[test]
fn a_roof_s_covering_and_layers_are_read_by_28_2_and_28_3() {
    let mut app = building();
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let layers: Vec<(&str, Option<&str>)> =
        facts.roofs.iter().flat_map(|r| r.layers.iter().map(|l| (l.kind.as_str(), l.material.as_deref()))).collect();
    assert_eq!(
        layers,
        [
            ("covering", Some("Kil kiremit")),
            ("battens", None),
            ("boarding", Some("Çam")),
            ("insulation", Some("Taş yünü")),
            ("rafters", Some("Çam"))
        ]
    );
    checked(&mut app);
    let (second, third, found) = said(&app);
    assert_eq!((second, third), (Some(O::MissingData), Some(O::OutOfScope)), "{found:#?}");
    assert!(found.iter().all(|(_, s, _)| *s == Some(S::MissingData)), "{found:#?}");
    // Declared: the tiles A1 and BROOF(t1), the pine D-s2,d0, the stone wool A1.
    declared(&mut app, "Kil kiremit", Some("A1"), Some("BROOF(t1)"));
    declared(&mut app, "Çam", Some("D-s2,d0"), None);
    declared(&mut app, "Taş yünü", Some("A1"), None);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let materials = &doc.settings().arch.as_ref().expect("Mimari").materials;
    let tiles = materials.iter().find(|m| m.name == "Kil kiremit").expect("the tiles");
    assert_eq!(tiles.roof_class.as_ref().map(|d| d.value.as_str()), Some("BROOF(t1)"));
    assert_eq!(doc.undo_labels().first().map(String::as_str), Some("Yangın ve performans"));
    checked(&mut app);
    let (second, third, found) = said(&app);
    assert_eq!((second, third), (Some(O::Passed), Some(O::ExpertReview)), "{found:#?}");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!((found[0].0.as_str(), found[0].1), ("bykhy_roof_noncombustible", Some(S::ExpertReview)));
    assert!(found[0].2.contains("mertek “Çam” D-s2,d0") && found[0].2.contains("nizam"), "{found:#?}");
    // An unreadable roof class is refused before Uygula, the window says why.
    let _ = app.update(Message::Run("arch.materials"));
    let _ = app.update(Message::Arch(arch::Event::Materials(Event::RoofClass("Broof".to_owned()))));
    let w = app.arch.materials.as_ref().expect("the window");
    assert!(w.declared.as_deref().is_some_and(|why| why.contains("TS EN 13501-5")), "{:?}", w.declared);
    let _ = app.update(Message::Arch(arch::Event::Materials(Event::Close)));
    // CROOF(t3) tiles fail 28/2.
    declared(&mut app, "Kil kiremit", None, Some("CROOF(t3)"));
    checked(&mut app);
    let (second, _, found) = said(&app);
    assert_eq!(second, Some(O::Failed), "{found:#?}");
    assert!(
        found.iter().any(|(c, s, m)| c == "bykhy_roof_covering_class"
            && s.is_none()
            && m.starts_with("1. kat: çatının kaplaması “Kil kiremit” CROOF(t3); 28/2 BROOF ister")),
        "{found:#?}"
    );
}

/// Malzemeler with the tiles' Dış yangın sınıfı, and Denetim's 28/3 finding, light and dark;
/// `.run/gui-evidence/cati-yanicilik-8134/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cati-yanicilik-8134");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = building();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        declared(&mut app, "Çam", Some("D-s2,d0"), None);
        declared(&mut app, "Taş yünü", Some("A1"), None);
        // The window open on the tiles, their classes typed, not yet applied.
        let _ = app.update(Message::Run("arch.materials"));
        let at = app.arch.materials.as_ref().expect("the window").list.iter().position(|(m, _)| m.name == "Kil kiremit");
        let _ = app.update(Message::Arch(arch::Event::Materials(Event::Choose(at.expect("the tiles")))));
        let _ = app.update(Message::Arch(arch::Event::Materials(Event::Reaction("A1".to_owned()))));
        let _ = app.update(Message::Arch(arch::Event::Materials(Event::RoofClass("BROOF(t1)".to_owned()))));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-malzemeler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::Arch(arch::Event::Materials(Event::Apply)));
        let _ = app.update(Message::Arch(arch::Event::Materials(Event::Close)));
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_roof_noncombustible");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
