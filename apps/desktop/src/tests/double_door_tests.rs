//! Çift kanatlı bina girişi (docs/adr/8093; Planlı Alanlar İmar Yönetmeliği 39/1-b) on a two-flat building's ground
//! storey, worked by hand: a 12 × 6 m plan inside 25 cm walls, the common Hol (x 0–4) and the two flats' rooms; the Hol's
//! outer door a two-leaf door. Two equal leaves in a 1.80 m hole leave each leaf alone under 1.00 m clear: 39/1-b fails;
//! a 2.40 m hole split 60 / 40 gives its first leaf over 1.00 m: it holds.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib double_door_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S};
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

fn p(x: f64, y: f64) -> Value {
    json!({"x": x, "y": y})
}

/// The building with the Hol's outer door `width` wide in two leaves, the first leaf's share `split` (none: halves).
fn building(width: f64, split: Option<f64>) -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    run(&mut app, "cad.settings.set", json!({ "arch": { "levels": [{"id": "k00", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}] } }));
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "nature": "Daire"}, {"id": "d2", "number": "2", "nature": "Daire"},
            {"id": "o", "number": "O", "name": "Ortak alan", "kind": "common"}
        ]}}),
    );
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.25, "arch": {"level": "k00"}});
    let walls = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (12.0, 0.0)), wall((12.0, 0.0), (12.0, 6.0)), wall((12.0, 6.0), (0.0, 6.0)),
            wall((0.0, 6.0), (0.0, 0.0)), wall((4.0, 0.0), (4.0, 6.0)), wall((8.0, 0.0), (8.0, 6.0))
        ]}),
    );
    let host = walls["output"]["uids"][0].as_str().expect("the south wall").to_owned();
    let mut door = json!({"host": host, "at": 2.0, "form": "door", "width": width, "leaves": 2, "head": 2.1});
    if let Some(s) = split {
        door["build"] = json!({"split": s});
    }
    run(&mut app, "cad.openings.create", json!({"layerId": layer, "openings": [door]}));
    let room = |x: f64, name: &str, unit: &str| json!({"seed": p(x, 3.0), "name": name, "unit": unit, "arch": {"level": "k00"}});
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [room(2.0, "Hol", "o"), room(6.0, "Salon", "d1"), room(10.0, "Salon", "d2")]}),
    );
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    app
}

fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

fn leaf_said(app: &App) -> (Option<O>, Vec<(Option<S>, String)>) {
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    let row = set.rules.iter().find(|r| r.rule == "PAİY 39/1").map(|r| r.state);
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code == "paiy_door_leaf_below")
        .map(|f| (f.state, f.message.clone()))
        .collect();
    (row, found)
}

#[test]
fn a_two_leaf_building_entrance_needs_one_leaf_of_a_metre() {
    let mut app = building(1.8, None);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let door = facts.doors.iter().find(|d| d.leaves.len() == 2).expect("the double door");
    let first = door.clear_first.expect("its first leaf's clear");
    assert!((door.leaves[0] - door.leaves[1]).abs() < 1e-9 && first < 1.0, "{door:#?}");
    assert!(door.clear.expect("its clear") >= 1.5, "both leaves open: {door:#?}");
    checked(&mut app);
    let (row, found) = leaf_said(&app);
    assert_eq!(row, Some(O::Failed));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].1.starts_with("Çift kanatlı bina giriş kapısının bir kanadının net genişliği")
            && found[0].1.contains("< 1.00 m"),
        "{found:#?}"
    );
    let mut wide = building(2.4, Some(0.6));
    checked(&mut wide);
    let (row, found) = leaf_said(&wide);
    assert!(found.is_empty(), "{found:#?}");
    assert_eq!(row, Some(O::Passed));
}

/// The Hol's double door and Denetim's 39/1-b finding, light and dark; `.run/gui-evidence/cift-kanat-8093/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cift-kanat-8093");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = building(1.8, None);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_door_leaf_below");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-cift-kanat-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
