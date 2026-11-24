//! Kullanım sınıfına özel kaçış hükümleri (docs/adr/8107; BYKHY 49/1-a, 50/1-a, 50/1-b, 52) worked by hand through the
//! commands, checked in Denetim with the standards package:
//!
//! - a three-storey office (Büro in the building programme, storeys of 3.00 m): each storey an Ofis of 12 × 10 m (12
//!   people) and a Merdiven evi. One stair on the upper storeys: 52's (a) 9 m and (b) 12 people hold, (c)–(d) are not
//!   drawn: expert review; a second stair on each: passed.
//! - a two-storey hotel (Otel): on 1. kat Oda 101, Koridor and two stair rooms in 20 cm walls, the room's door in the
//!   wall to the corridor. Nothing declared: 50/1-a and 50/1-b missing data; the wall EI 30: failed; EI 60 and the door
//!   EI2 30-C5: both passed.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib use_class_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::ArchAuditOutcome as O;
use kentos_launch::Module;
use serde_json::{Value, json};
use crate::arch::ArchState;

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

pub(crate) fn run(app: &mut App, name: &str, input: Value) -> Value {
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

pub(crate) fn p(x: f64, y: f64) -> Value {
    json!({"x": x, "y": y})
}

pub(crate) fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Value {
    json!([{"outer": {"pts": [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]}}])
}

/// A new drawing with `n` storeys of 3.00 m and one building used as `usage`; applied for on 2.7.2026.
pub(crate) fn building(n: usize, usage: &str) -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    let levels: Vec<Value> = (0..n)
        .map(|i| {
            let name = if i == 0 { "Zemin kat".to_owned() } else { format!("{i}. kat") };
            json!({"id": format!("k{i:02}"), "name": name, "elevation": 3.0 * i as f64, "height": 3.0})
        })
        .collect();
    let ids: Vec<String> = (0..n).map(|i| format!("k{i:02}")).collect();
    run(&mut app, "cad.settings.set", json!({ "arch": { "levels": levels } }));
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {
            "buildings": [{"id": "a", "name": "A Blok", "levels": ids, "usage": usage}],
            "units": [{"id": "b1", "number": "1", "nature": usage}]
        }}),
    );
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    app
}

fn layer(app: &App) -> String {
    app.document.as_ref().expect("a drawing").model.layers().active().to_owned()
}

pub(crate) fn spaces(app: &mut App, list: Vec<Value>) {
    let layer = layer(app);
    run(app, "cad.spaces.create", json!({"layerId": layer, "spaces": list}));
}

/// The office: on each storey an Ofis and as many stair rooms as `stairs` says for it.
fn office(stairs: &[usize]) -> App {
    let mut app = building(stairs.len(), "Büro");
    let mut list = Vec::new();
    for (i, &n) in stairs.iter().enumerate() {
        let level = format!("k{i:02}");
        list.push(json!({"parts": rect(0.0, 0.0, 12.0, 10.0), "name": "Ofis", "unit": "b1", "arch": {"level": level}}));
        for j in 0..n {
            let x = 12.0 + 3.0 * j as f64;
            let name = if j == 0 { "Merdiven evi".to_owned() } else { format!("Merdiven evi {}", j + 1) };
            list.push(json!({"parts": rect(x, 0.0, x + 3.0, 4.5), "name": name, "arch": {"level": level}}));
        }
    }
    spaces(&mut app, list);
    app
}

/// The hotel's 1. kat: Oda 101 (x 0–5), Koridor (5–8) and two stair rooms (8–10, 10–12) inside 20 cm walls; the room's
/// door in the wall at x = 5. Its walls' and its door's persistent ids.
fn hotel() -> (App, Vec<String>, String) {
    let mut app = building(2, "Otel");
    let layer = layer(&app);
    let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": "k01"}});
    let walls = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (12.0, 0.0)), wall((12.0, 0.0), (12.0, 5.0)), wall((12.0, 5.0), (0.0, 5.0)),
            wall((0.0, 5.0), (0.0, 0.0)), wall((5.0, 0.0), (5.0, 5.0)), wall((8.0, 0.0), (8.0, 5.0)),
            wall((10.0, 0.0), (10.0, 5.0))
        ]}),
    );
    let walls: Vec<String> =
        walls["output"]["uids"].as_array().expect("walls").iter().map(|u| u.as_str().expect("a wall").to_owned()).collect();
    let doors = run(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [{"host": walls[4], "at": 2.5, "form": "door", "width": 1.0, "head": 2.1}]}),
    );
    let door = doors["output"]["uids"][0].as_str().expect("the door").to_owned();
    spaces(
        &mut app,
        vec![
            json!({"parts": rect(0.1, 0.1, 4.9, 4.9), "name": "Oda 101", "unit": "b1", "arch": {"level": "k01"}}),
            json!({"parts": rect(5.1, 0.1, 7.9, 4.9), "name": "Koridor", "arch": {"level": "k01"}}),
            json!({"parts": rect(8.1, 0.1, 9.9, 4.9), "name": "Merdiven evi", "arch": {"level": "k01"}}),
            json!({"parts": rect(10.1, 0.1, 11.9, 4.9), "name": "Merdiven evi 2", "arch": {"level": "k01"}}),
        ],
    );
    (app, walls, door)
}

pub(crate) fn declare(app: &mut App, uids: &[&String], value: &str) {
    run(
        app,
        "cad.arch.performance",
        json!({"operation": {"kind": "objects", "uids": uids, "fireRating": {"value": value, "basis": {"kind": "author"}}}}),
    );
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
pub(crate) fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.modules.arch().arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The four articles' outcomes, in order.
fn rows(app: &App) -> Vec<O> {
    let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
    ["BYKHY 49/1-a", "BYKHY 50/1-a", "BYKHY 50/1-b", "BYKHY 52"]
        .iter()
        .map(|rule| set.rules.iter().find(|r| r.rule == *rule).unwrap_or_else(|| panic!("{rule}: {:#?}", set.rules)).state)
        .collect()
}

/// The new rules' findings: code and message.
fn findings(app: &App) -> Vec<(String, String)> {
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    audited
        .findings
        .iter()
        .filter(|f| f.code.starts_with("bykhy_single_") || f.code.starts_with("bykhy_hotel_") || f.code.starts_with("bykhy_patient_"))
        .map(|f| (f.code.clone(), f.message.clone()))
        .collect()
}

#[test]
fn an_office_with_one_stair_is_for_the_reviewer_and_with_two_it_passes() {
    let mut app = office(&[1, 1, 1]);
    checked(&mut app);
    assert_eq!(rows(&app), [O::OutOfScope, O::OutOfScope, O::OutOfScope, O::ExpertReview]);
    let said = findings(&app);
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said.iter().all(|(c, m)| c == "bykhy_single_stair" && m.contains("çizimden kanıtlanmaz")), "{said:?}");
    let mut app = office(&[1, 2, 2]);
    checked(&mut app);
    assert_eq!(rows(&app)[3], O::Passed);
    assert!(findings(&app).is_empty());
}

#[test]
fn a_hotel_room_s_wall_and_door_to_the_corridor_are_read_from_their_declarations() {
    let (mut app, walls, door) = hotel();
    checked(&mut app);
    assert_eq!(rows(&app), [O::OutOfScope, O::MissingData, O::MissingData, O::OutOfScope]);
    let said = findings(&app);
    assert!(said.iter().any(|(c, m)| c == "bykhy_hotel_room_wall" && m.contains("Oda 101")), "{said:?}");
    assert!(said.iter().any(|(c, _)| c == "bykhy_hotel_room_door"), "{said:?}");
    // The wall between the room and the corridor EI 30: 50/1-a fails.
    declare(&mut app, &[&walls[4]], "EI 30");
    checked(&mut app);
    assert_eq!(rows(&app)[1], O::Failed);
    assert!(findings(&app).iter().any(|(_, m)| m.contains("30 dakika < 60 dakika")));
    // EI 60, and the door EI2 30-C5: both pass.
    declare(&mut app, &[&walls[4]], "EI 60");
    declare(&mut app, &[&door], "EI2 30-C5");
    checked(&mut app);
    assert_eq!(rows(&app), [O::OutOfScope, O::Passed, O::Passed, O::OutOfScope]);
}

/// BYKHY 96/2-a (docs/adr/8108): an eleven-storey office, 33 m, asks for sprinklers; declared, it passes.
#[test]
fn a_33_metre_office_asks_for_sprinklers_until_they_are_declared() {
    let mut app = office(&[1; 11]);
    checked(&mut app);
    let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
    let row = |set: &kentos_commands::ArchAuditRuleSet| set.rules.iter().find(|r| r.rule == "BYKHY 96/2").expect("96/2").state;
    assert_eq!(row(set), O::Failed);
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    assert!(
        audited.findings.iter().any(|f| f.code == "bykhy_sprinklers_required" && f.message.contains("33,00 m")),
        "{:?}",
        audited.findings.iter().map(|f| &f.code).collect::<Vec<_>>()
    );
    // The whole architectural settings with sprinklers on (cad.settings.set writes `arch` whole).
    let mut arch = app.document.as_ref().expect("a drawing").model.settings().arch.clone().unwrap_or_default();
    arch.sprinklers = true;
    run(&mut app, "cad.settings.set", json!({ "arch": arch }));
    checked(&mut app);
    assert_eq!(row(app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set")), O::Passed);
}

/// Denetim's pictures (`.run/gui-evidence/kullanim-sinifi-8107/`): the office's expert review, the hotel's failing wall.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-sinifi-8107");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let shot = |app: &mut App, file: &str| {
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            app.toasts.clear();
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        // The Mevzuat group, the new rule's finding chosen: its words and its row in sight.
        let choose = |app: &mut App, code: &str| {
            let group = crate::arch::Event::ArchAudit(crate::arch_audit::Event::Group(Some(
                kentos_commands::ArchAuditCheck::Regulation,
            )));
            let _ = app.update(Message::from(group));
            let shown = app.arch_audit_shown();
            let findings = &app.modules.arch().arch_audit.audited.as_ref().expect("checked").findings;
            let row = shown.iter().position(|&i| findings[i].code == code).expect("the finding shown");
            let _ = app.update(Message::from(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Press(row))));
        };
        let mut app = office(&[1, 1, 1]);
        checked(&mut app);
        choose(&mut app, "bykhy_single_stair");
        shot(&mut app, "1-buro-tek-merdiven");
        let (mut app, walls, _) = hotel();
        declare(&mut app, &[&walls[4]], "EI 30");
        checked(&mut app);
        choose(&mut app, "bykhy_hotel_room_wall");
        shot(&mut app, "2-otel-duvar");
    }
}
