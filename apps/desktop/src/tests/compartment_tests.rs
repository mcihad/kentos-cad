//! Yangın kompartımanları (docs/adr/8081; Binaların Yangından Korunması Hakkında Yönetmelik 4/tt, 24) on an office's
//! storey, worked by hand: a 20 × 10 m plan inside 20 cm walls, an office (x 0–12), an archive (12–16) and a boiler room
//! (16–20), each drawn to its walls' faces. The wall at x = 12 is declared REI 90 and holds a 1.00 m door; the one at
//! x = 16 is declared as each test says. The building is used as a büro (Ek-4: 8000 m²).

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
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

/// The office's storey: the boiler room's wall declared `boiler`, the office's door `door` (none: not declared).
fn storey(boiler: &str, door: Option<&str>) -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [{"id": "k00", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}] } }),
    );
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {
            "buildings": [{"id": "a", "name": "A Blok", "levels": ["k00"], "usage": "Büro"}],
            "units": [{"id": "b1", "number": "1", "nature": "Büro"}]
        }}),
    );
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": "k00"}});
    let walls = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (20.0, 0.0)), wall((20.0, 0.0), (20.0, 10.0)), wall((20.0, 10.0), (0.0, 10.0)),
            wall((0.0, 10.0), (0.0, 0.0)), wall((12.0, 0.0), (12.0, 10.0)), wall((16.0, 0.0), (16.0, 10.0))
        ]}),
    );
    let host = |i: usize| walls["output"]["uids"][i].as_str().expect("a wall").to_owned();
    let doors = run(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [{"host": host(4), "at": 5.0, "form": "door", "width": 1.0, "head": 2.1}]}),
    );
    let declare = |app: &mut App, uid: Value, value: &str| {
        run(
            app,
            "cad.arch.performance",
            json!({"operation": {"kind": "objects", "uids": [uid], "fireRating": {"value": value, "basis": {"kind": "author"}}}}),
        );
    };
    declare(&mut app, json!(host(4)), "REI 90");
    declare(&mut app, json!(host(5)), boiler);
    if let Some(d) = door {
        declare(&mut app, doors["output"]["uids"][0].clone(), d);
    }
    let rect = |x0: f64, x1: f64| vec![p(x0, 0.1), p(x1, 0.1), p(x1, 9.9), p(x0, 9.9)];
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"parts": [{"outer": {"pts": rect(0.1, 11.9)}}], "name": "Ofis", "unit": "b1", "arch": {"level": "k00"}},
            {"parts": [{"outer": {"pts": rect(12.1, 15.9)}}], "name": "Arşiv", "unit": "b1", "arch": {"level": "k00"}},
            {"parts": [{"outer": {"pts": rect(16.1, 19.9)}}], "name": "Kazan dairesi", "unit": "b1", "arch": {"level": "k00"}}
        ]}),
    );
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    app
}

/// What separates the rooms, as the facts say it: the office and the archive by the REI 90 wall and its door (the
/// boundary's 9.80 m less the corners' 0.30 m each, 9.20 m: 8.20 m of wall and the door's 1.00 m), the archive and the
/// boiler room by its wall alone.
#[test]
fn the_facts_say_what_separates_each_two_rooms() {
    let app = storey("EI 30", None);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let name = |uid: &str| facts.spaces.iter().find(|s| s.uid == uid).map(|s| s.name.clone()).unwrap_or_default();
    let mut seps: Vec<(Vec<String>, Vec<(String, Option<String>, f64)>, f64)> = facts
        .separations
        .iter()
        .map(|s| {
            let mut rooms: Vec<String> = s.rooms.iter().map(|r| name(r)).collect();
            rooms.sort();
            let elements = s
                .walls
                .iter()
                .chain(&s.openings)
                .map(|e| (e.kind.clone(), e.fire_rating.clone(), e.length))
                .collect();
            (rooms, elements, s.open)
        })
        .collect();
    seps.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(seps.len(), 2, "{seps:?}");
    assert_eq!(seps[0].0, ["Arşiv", "Kazan dairesi"]);
    assert_eq!(seps[0].1.len(), 1);
    assert_eq!((seps[0].1[0].0.as_str(), seps[0].1[0].1.as_deref()), ("wall", Some("EI 30")));
    assert!((seps[0].1[0].2 - 9.2).abs() < 0.13, "{seps:?}");
    assert_eq!(seps[1].0, ["Arşiv", "Ofis"]);
    let kinds: Vec<(&str, Option<&str>)> = seps[1].1.iter().map(|(k, r, _)| (k.as_str(), r.as_deref())).collect();
    assert_eq!(kinds, [("wall", Some("REI 90")), ("door", None)], "{seps:?}");
    let (wall, door) = (seps[1].1[0].2, seps[1].1[1].2);
    assert!((door - 1.0).abs() < 0.13 && (wall + door - 9.2).abs() < 0.13, "{seps:?}");
    assert!(seps.iter().all(|s| s.2 < 1e-9), "no open stretch: {seps:?}");
    assert_eq!(facts.spaces.len(), 3);
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

fn outcome(app: &App, rule: &str) -> Option<kentos_commands::ArchAuditOutcome> {
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    set.rules.iter().find(|r| r.rule == rule).map(|r| r.state)
}

/// 24/2: the boiler room's EI 30 wall is no compartment wall; 24/7: the REI 90 wall's door has no class. Declared REI
/// 120 and EI2 60-S200C5, both pass; the storey's one compartment (200 m²) is within Ek-4's büro 8000 m² throughout.
#[test]
fn the_boiler_room_s_wall_and_the_compartment_wall_s_door_are_read_by_bykhy_24() {
    use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S};
    let mut app = storey("EI 30", None);
    checked(&mut app);
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let hazard = audited.findings.iter().find(|f| f.code == "bykhy_hazard_room_separation").expect("24/2");
    assert!(hazard.message.contains("Kazan dairesi") && hazard.message.contains("EI 30"), "{}", hazard.message);
    assert_eq!(hazard.state, None);
    let opening = audited.findings.iter().find(|f| f.code == "bykhy_compartment_opening").expect("24/7");
    assert_eq!(opening.state, Some(S::MissingData), "{}", opening.message);
    assert_eq!(outcome(&app, "BYKHY 24/2"), Some(O::Failed));
    assert_eq!(outcome(&app, "BYKHY 24/6"), Some(O::Passed));
    assert_eq!(outcome(&app, "BYKHY 24/7"), Some(O::MissingData));
    let mut fixed = storey("REI 120", Some("EI2 60-S200C5"));
    checked(&mut fixed);
    let audited = fixed.arch.arch_audit.audited.as_ref().expect("checked");
    assert!(!audited.findings.iter().any(|f| f.code.starts_with("bykhy_compartment") || f.code.starts_with("bykhy_hazard")));
    assert_eq!(outcome(&fixed, "BYKHY 24/2"), Some(O::Passed));
    assert_eq!(outcome(&fixed, "BYKHY 24/7"), Some(O::Passed));
}

/// The office's storey and Denetim's BYKHY 24 findings, light and dark; `.run/gui-evidence/kompartiman-8081/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib compartment_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kompartiman-8081");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = storey("EI 30", None);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_hazard_room_separation");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-kompartiman-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
