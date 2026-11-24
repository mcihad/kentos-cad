//! Mahallerin kaplamaları (docs/adr/8138; Binaların Yangından Korunması Hakkında Yönetmelik 20/3, 26/2, 26/4, Ek-2/Ç) on
//! a four-flat building's 1. kat, worked by hand: Salon and Oda, their wall finish typed "Duvar kağıdı" and their floor
//! finish "Seramik" in Öznitelikler. With no material of those names both are missing data; Malzemeler's Duvar kağıdı
//! with Yanıcılık sınıfı F (kolay alevlenici) fails 20/3's normal alevlenici, one finding for both rooms; B-s1,d0 (zor
//! alevlenici) holds; Seramik A1fl holds 26/2.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib finish_reaction_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S};
use kentos_launch::Module;
use serde_json::{Value, json};
use crate::arch::ArchState;

use crate::app::{App, Message};
use crate::arch;
use crate::files_testing::drive;
use crate::materials_window::Event;
use crate::properties::{Event as Properties, Field};
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

/// The building: two storeys, four flats, 1. kat's Salon and Oda; applied on 2.7.2026. The rooms' slots.
fn building() -> (App, Vec<kentos_domain::Slot>) {
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
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let rect = |x0: f64, x1: f64| vec![p(x0, 0.0), p(x1, 0.0), p(x1, 4.0), p(x0, 4.0)];
    let made = run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"parts": [{"outer": {"pts": rect(0.0, 5.0)}}], "name": "Salon", "unit": "d1", "arch": {"level": "k01"}},
            {"parts": [{"outer": {"pts": rect(5.0, 9.0)}}], "name": "Oda", "unit": "d1", "arch": {"level": "k01"}}
        ]}),
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let slots = made["output"]["uids"]
        .as_array()
        .expect("the rooms")
        .iter()
        .map(|u| doc.slot_of(u.as_str().expect("a uid").parse().expect("a uid")).expect("its slot"))
        .collect();
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    (app, slots)
}

/// Each room's wall finish Duvar kağıdı and floor finish Seramik, typed in Öznitelikler.
fn finished(app: &mut App, rooms: &[kentos_domain::Slot]) {
    for &room in rooms {
        app.selection.set(vec![room]);
        let _ = app.update(Message::Properties(Properties::Commit(Field::SpaceWall(room), "Duvar kağıdı".into())));
        let _ = app.update(Message::Properties(Properties::Commit(Field::SpaceFloor(room), "Seramik".into())));
    }
    app.selection.set(Vec::new());
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.modules.arch().arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// 20/3's and 26/2's rows and the finishes' findings (code, state, message, objects).
type Found = Vec<(String, Option<S>, String, usize)>;
fn said(app: &App) -> (Option<O>, Option<O>, Found) {
    let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
    let row = |rule: &str| set.rules.iter().find(|r| r.rule == rule).map(|r| r.state);
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code == "bykhy_wall_lining_reaction" || f.code == "bykhy_floor_covering_reaction")
        .map(|f| (f.code.clone(), f.state, f.message.clone(), f.uids.len()))
        .collect();
    (row("BYKHY 20/3"), row("BYKHY 26/2"), found)
}

/// Malzemeler open, `name` chosen, its Yanıcılık sınıfı typed `class` and applied.
pub(crate) fn declared(app: &mut App, name: &str, class: &str) {
    let _ = app.update(Message::Run("arch.materials"));
    let at = app.modules.arch().materials.as_ref().expect("the window").list.iter().position(|(m, _)| m.name == name).expect(name);
    let _ = app.update(Message::from(arch::Event::Materials(Event::Choose(at))));
    let _ = app.update(Message::from(arch::Event::Materials(Event::Reaction(class.to_owned()))));
    let _ = app.update(Message::from(arch::Event::Materials(Event::Apply)));
}

/// A row's note in Öznitelikler: the row after the one labelled `label`.
fn note_under(app: &App, label: &str) -> String {
    let doc = app.document.as_ref().expect("open");
    let rows: Vec<(String, String)> =
        app.properties_panel(doc).sections.into_iter().flat_map(|s| s.rows).map(|r| (r.label.to_string(), r.value)).collect();
    let at = rows.iter().position(|r| r.0 == label).unwrap_or_else(|| panic!("no {label} row: {rows:#?}"));
    rows.get(at + 1).map(|r| r.1.clone()).unwrap_or_default()
}

#[test]
fn a_room_s_finishes_are_read_by_their_materials_classes() {
    let (mut app, rooms) = building();
    finished(&mut app, &rooms);
    // The facts: each room's floor and wall finish by name, no material of those names yet.
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let salon = facts.spaces.iter().find(|s| s.name == "Salon").expect("Salon");
    let parts: Vec<(&str, &str, Option<&str>)> =
        salon.finishes.iter().map(|f| (f.part.as_str(), f.name.as_str(), f.material.as_deref())).collect();
    assert_eq!(parts, [("floor", "Seramik", None), ("wall", "Duvar kağıdı", None)]);
    app.selection.set(vec![rooms[0]]);
    assert_eq!(note_under(&app, "Duvar kaplaması"), "Malzemeler'de bu adla malzeme yok.");
    checked(&mut app);
    let (wall, floor, found) = said(&app);
    assert_eq!((wall, floor), (Some(O::MissingData), Some(O::MissingData)));
    assert_eq!(found.len(), 2, "{found:#?}");
    let wall_found = found.iter().find(|f| f.0 == "bykhy_wall_lining_reaction").expect("20/3's");
    assert_eq!((wall_found.1, wall_found.3), (Some(S::MissingData), 2), "one finding for both rooms");
    assert!(wall_found.2.starts_with("“Duvar kağıdı” duvar kaplaması (2 mahal): Malzemeler'de “Duvar kağıdı” adlı malzeme yok"));
    // The materials named so, Duvar kağıdı F: 20/3 fails; Seramik A1fl: 26/2 holds.
    for name in ["Duvar kağıdı", "Seramik"] {
        run(&mut app, "cad.materials.edit", json!({"operation": {"kind": "define", "name": name, "change": {}}}));
    }
    declared(&mut app, "Duvar kağıdı", "F");
    declared(&mut app, "Seramik", "A1fl");
    app.dialog = None;
    app.modules.arch_mut().materials = None;
    app.selection.set(vec![rooms[0]]);
    assert_eq!(note_under(&app, "Duvar kaplaması"), "Malzemelerden: F.");
    assert_eq!(note_under(&app, "Döşeme kaplaması"), "Malzemelerden: A1fl.");
    checked(&mut app);
    let (wall, floor, found) = said(&app);
    assert_eq!((wall, floor), (Some(O::Failed), Some(O::Passed)));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].2.starts_with(
            "“Duvar kağıdı” duvar kaplaması (2 mahal): malzemesi “Duvar kağıdı” F, kolay alevlenici; 20/3 en az normal \
             alevlenici ister"
        ),
        "{}",
        found[0].2
    );
    // B-s1,d0, zor alevlenici: it holds.
    declared(&mut app, "Duvar kağıdı", "B-s1,d0");
    app.dialog = None;
    app.modules.arch_mut().materials = None;
    checked(&mut app);
    let (wall, _, found) = said(&app);
    assert_eq!(wall, Some(O::Passed));
    assert!(found.is_empty(), "{found:#?}");
}

/// Öznitelikler of Salon with its finishes' notes, and Denetim with 20/3's finding chosen, light and dark;
/// `.run/gui-evidence/mahal-kaplamasi-8138/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/mahal-kaplamasi-8138");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 1180.0), "acik"), ("dark", Size::new(1280.0, 1100.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, rooms) = building();
        finished(&mut app, &rooms);
        for name in ["Duvar kağıdı", "Seramik"] {
            run(&mut app, "cad.materials.edit", json!({"operation": {"kind": "define", "name": name, "change": {}}}));
        }
        declared(&mut app, "Duvar kağıdı", "F");
        declared(&mut app, "Seramik", "A1fl");
        app.dialog = None;
        app.modules.arch_mut().materials = None;
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::RibbonTab("arch"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.selection.set(vec![rooms[0]]);
        snapshot.settle(&mut app, App::view, &mut update);
        // The finishes are under the room's measures: the properties panel scrolled down.
        let at = iced::Point::new(size.width - 150.0, size.height - 250.0);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -8.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-oznitelikler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        app.selection.set(Vec::new());
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_wall_lining_reaction");
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
