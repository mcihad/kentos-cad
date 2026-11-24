//! Şaftlar ve asansörler (docs/adr/8087, 8135; Binaların Yangından Korunması Hakkında Yönetmelik 4/ccc, 25/3, 41/9, 62/2,
//! 62/4, 62/6, 62/7 ve Ek-3/B 12) on an eight-storey office worked by hand: storeys of 3.00 m (bina ve yapı yüksekliği 24.00 m,
//! 21,50 m'den fazla: a high building). On Zemin kat a 20 × 10 m plan inside 20 cm walls: Ofis (x 0–12), Hol (12–17)
//! and east of the wall at x = 17 the elevator's shaft Asansör (y 6–10), the service shaft Tesisat şaftı (y 4.5–6) and
//! Merdiven evi (y 0–4.5), each drawn to its walls' faces. The elevator's landing door, sliding, 1.00 m in the wall at
//! x = 17, opens onto Hol; the shaft's 60 cm door, in the wall at y = 4.5, swings out into Merdiven evi (41/9 forbids it).
//! The lift shaft is 2.80 × 3.80 = 10.64 m²: 62/4 asks a vent of 0.025 × 10.64 = 0.266 m² (more than 0.1 m²) or a
//! pressurised shaft.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib shaft_tests::screens -- --ignored --nocapture
//! ```

use crate::arch_ext::*;
use kentos_commands::ArchAuditOutcome as O;
use kentos_launch::Module;
use serde_json::{Value, json};
use crate::arch::ArchState;

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

/// The office, nothing declared; applied for on 2.7.2026. Its walls' and doors' persistent ids, in the order drawn.
fn office() -> (App, Vec<String>, Vec<String>) {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    let levels: Vec<Value> = (0..8)
        .map(|i| {
            let name = if i == 0 { "Zemin kat".to_owned() } else { format!("{i}. kat") };
            json!({"id": format!("k{i:02}"), "name": name, "elevation": 3.0 * i as f64, "height": 3.0})
        })
        .collect();
    let ids: Vec<String> = (0..8).map(|i| format!("k{i:02}")).collect();
    run(&mut app, "cad.settings.set", json!({ "arch": { "levels": levels } }));
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {
            "buildings": [{"id": "a", "name": "A Blok", "levels": ids, "usage": "Büro"}],
            "units": [{"id": "b1", "number": "1", "nature": "Büro"}]
        }}),
    );
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": "k00"}});
    let walls = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (20.0, 0.0)), wall((20.0, 0.0), (20.0, 10.0)), wall((20.0, 10.0), (0.0, 10.0)),
            wall((0.0, 10.0), (0.0, 0.0)), wall((12.0, 0.0), (12.0, 10.0)), wall((17.0, 0.0), (17.0, 10.0)),
            wall((17.0, 6.0), (20.0, 6.0)), wall((17.0, 4.5), (20.0, 4.5))
        ]}),
    );
    let walls: Vec<String> =
        walls["output"]["uids"].as_array().expect("walls").iter().map(|u| u.as_str().expect("a wall").to_owned()).collect();
    let door = |host: &str, at: f64, width: f64| json!({"host": host, "at": at, "form": "door", "width": width, "head": 2.1});
    // The landing door slides; the shaft's door swings out to the path's right, into Merdiven evi.
    let mut landing = door(&walls[5], 8.0, 1.0);
    landing["operation"] = json!("slide");
    let mut cover = door(&walls[7], 1.5, 0.6);
    cover["side"] = json!("right");
    let doors = run(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [landing, cover, door(&walls[4], 5.0, 1.0), door(&walls[5], 2.0, 1.0)]}),
    );
    let doors: Vec<String> =
        doors["output"]["uids"].as_array().expect("doors").iter().map(|u| u.as_str().expect("a door").to_owned()).collect();
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)];
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"parts": [{"outer": {"pts": rect(0.1, 0.1, 11.9, 9.9)}}], "name": "Ofis", "unit": "b1", "arch": {"level": "k00"}},
            {"parts": [{"outer": {"pts": rect(12.1, 0.1, 16.9, 9.9)}}], "name": "Hol", "arch": {"level": "k00"}},
            {"parts": [{"outer": {"pts": rect(17.1, 6.1, 19.9, 9.9)}}], "name": "Asansör", "void": true, "arch": {"level": "k00"}},
            {"parts": [{"outer": {"pts": rect(17.1, 4.6, 19.9, 5.9)}}], "name": "Tesisat şaftı", "void": true, "arch": {"level": "k00"}},
            {"parts": [{"outer": {"pts": rect(17.1, 0.1, 19.9, 4.4)}}], "name": "Merdiven evi", "arch": {"level": "k00"}}
        ]}),
    );
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    (app, walls, doors)
}

/// Asansör's room on `level` (the same outline as Zemin kat's) and its persistent id.
fn lift_room(app: &mut App, level: &str) -> String {
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let pts = [p(17.1, 6.1), p(19.9, 6.1), p(19.9, 9.9), p(17.1, 9.9)];
    let made = run(
        app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"parts": [{"outer": {"pts": pts}}], "name": "Asansör", "void": true, "arch": {"level": level}}
        ]}),
    );
    made["output"]["uids"][0].as_str().expect("the room").to_owned()
}

/// The persistent id of the room named `name` on `level`.
fn room(app: &App, name: &str, level: &str) -> String {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities_with_uids()
        .find_map(|(u, e)| match e {
            kentos_commands::Entity::Space(s)
                if s.space.name.as_deref() == Some(name) && s.space.arch.level.as_deref() == Some(level) =>
            {
                Some(u.to_string())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} on {level}"))
}

fn vent(app: &mut App, uid: &str, vent: Value) {
    run(app, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": [uid], "shaftVent": vent}}));
}

fn declare(app: &mut App, uids: &[&String], value: &str) {
    run(
        app,
        "cad.arch.performance",
        json!({"operation": {"kind": "objects", "uids": uids, "fireRating": {"value": value, "basis": {"kind": "author"}}}}),
    );
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.modules.arch().arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The six articles' rows, in order (outcome, version).
fn rows(app: &App) -> Vec<(String, O, Option<String>)> {
    let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
    ["BYKHY 25/3", "BYKHY 41/9", "BYKHY 62/2", "BYKHY 62/4", "BYKHY 62/6", "BYKHY 62/7"]
        .iter()
        .map(|rule| {
            let r = set.rules.iter().find(|r| r.rule == *rule).unwrap_or_else(|| panic!("{rule}: {:#?}", set.rules));
            (r.rule.clone(), r.state, r.version.clone())
        })
        .collect()
}

fn codes(app: &App) -> Vec<String> {
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    let mut out: Vec<String> = audited
        .findings
        .iter()
        .filter(|f| f.code.starts_with("bykhy_shaft_") || f.code.starts_with("bykhy_elevator_"))
        .map(|f| f.code.clone())
        .collect();
    out.sort();
    out
}

/// The facts: the elevator's shaft enclosed by the east and north outer walls, the wall at x = 17 with the landing door
/// and the wall at y = 6; the service shaft by the east wall, the wall at x = 17, the wall at y = 6 and the wall at
/// y = 4.5 with its door. The landing door is an elevator's and opens onto a passage; the shaft's door opens onto the
/// stair's room. The structure is 24.00 m high.
#[test]
fn the_facts_carry_the_shafts_and_the_elevator_s_door() {
    let (app, walls, doors) = office();
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    assert!((facts.height - 24.0).abs() < 1e-9 && (facts.structure_height - 24.0).abs() < 1e-9, "{}", facts.height);
    assert_eq!(facts.shafts.len(), 2, "{:#?}", facts.shafts);
    let shaft = |kind: &str| facts.shafts.iter().find(|s| s.kind == kind).expect(kind);
    let set = |v: &[kentos_extensions::audit_facts::building::SeparatingElement]| {
        let mut u: Vec<String> = v.iter().map(|e| e.uid.clone()).collect();
        u.sort();
        u
    };
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    let lift = shaft("elevator");
    assert_eq!(lift.name, "Asansör");
    // Its outline's area, the shaft itself through the storeys (one storey here) and no vent declared (docs/adr/8135).
    assert!((lift.area - 10.64).abs() < 1e-9, "{}", lift.area);
    assert_eq!(lift.stack, lift.uid);
    assert_eq!(lift.vent, None);
    assert_eq!(set(&lift.walls), sorted(vec![walls[1].clone(), walls[2].clone(), walls[5].clone(), walls[6].clone()]));
    assert_eq!(set(&lift.openings), vec![doors[0].clone()]);
    assert!(lift.open < 1e-9, "{lift:#?}");
    let service = shaft("service");
    assert_eq!(service.name, "Tesisat şaftı");
    assert_eq!(set(&service.walls), sorted(vec![walls[1].clone(), walls[5].clone(), walls[6].clone(), walls[7].clone()]));
    assert_eq!(set(&service.openings), vec![doors[1].clone()]);
    assert!(service.open < 1e-9, "{service:#?}");
    let door = |uid: &str| facts.doors.iter().find(|d| d.uid == uid).expect("the door");
    let (landing, cover) = (door(&doors[0]), door(&doors[1]));
    assert!(landing.elevator && landing.passage && !landing.stair, "{landing:#?}");
    assert!(!cover.elevator && cover.stair, "{cover:#?}");
    assert!(!door(&doors[2]).elevator, "Ofis–Hol");
}

/// Nothing declared: the high building's shaft and the elevator's shaft are missing data, so are the landing door's
/// class and the lift shaft's vent (62/4, docs/adr/8135); the shaft's door onto Merdiven evi breaks 41/9; the landing
/// door onto Hol keeps 62/6. Declared EI 120 round both shafts, the shaft's door EI2 90-S200C5, the landing door E 30
/// (51,50 m'den alçak) and a 0.27 m² vent, every article holds but 41/9.
#[test]
fn the_shafts_and_the_elevator_are_read_by_25_3_41_9_and_62() {
    let (mut app, walls, doors) = office();
    checked(&mut app);
    let v = |s: &str| Some(s.to_owned());
    assert_eq!(
        rows(&app),
        [
            ("BYKHY 25/3".to_owned(), O::MissingData, v("9.9.2009")),
            ("BYKHY 41/9".to_owned(), O::Failed, v("9.9.2009")),
            ("BYKHY 62/2".to_owned(), O::MissingData, v("19.12.2007")),
            ("BYKHY 62/4".to_owned(), O::MissingData, v("19.12.2007")),
            ("BYKHY 62/6".to_owned(), O::Passed, v("19.12.2007")),
            ("BYKHY 62/7".to_owned(), O::MissingData, v("9.7.2015")),
        ]
    );
    assert_eq!(
        codes(&app),
        [
            "bykhy_elevator_door_resistance",
            "bykhy_elevator_shaft",
            "bykhy_elevator_shaft_vent",
            "bykhy_shaft_cover_stair",
            "bykhy_shaft_resistance"
        ]
    );
    declare(&mut app, &[&walls[1], &walls[2], &walls[5], &walls[6], &walls[7]], "EI 120");
    declare(&mut app, &[&doors[1]], "EI2 90-S200C5");
    declare(&mut app, &[&doors[0]], "E 30");
    let lift = room(&app, "Asansör", "k00");
    vent(&mut app, &lift, json!({"area": 0.27, "basis": {"kind": "author"}}));
    checked(&mut app);
    let states: Vec<O> = rows(&app).into_iter().map(|r| r.1).collect();
    assert_eq!(states, [O::Passed, O::Failed, O::Passed, O::Passed, O::Passed, O::Passed]);
    assert_eq!(codes(&app), ["bykhy_shaft_cover_stair"]);
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    let cover = audited.findings.iter().find(|f| f.code == "bykhy_shaft_cover_stair").expect("41/9");
    assert!(
        cover.message.starts_with("Şaft kapısı (“Tesisat şaftı”) kaçış merdiveni yuvasına açılıyor (“Merdiven evi”)"),
        "{}",
        cover.message
    );
}

/// 62/4's first sentence (docs/adr/8135): the lift shaft through Zemin kat, 1. kat and 2. kat is one shaft; its vent,
/// typed in Öznitelikler on 1. kat's room, is read for the whole shaft. 0.2 m² is less than 0.266 m²; pressurised it
/// holds; the area taken away it stays pressurised; not pressurised either, the declaration is gone.
#[test]
fn a_lift_shaft_s_vent_is_declared_once_for_the_shaft_in_oznitelikler() {
    use crate::properties::{ArchField, Event, Field};
    let (mut app, _, _) = office();
    let (one, two) = (lift_room(&mut app, "k01"), lift_room(&mut app, "k02"));
    let ground = room(&app, "Asansör", "k00");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let lifts: Vec<_> = facts.shafts.iter().filter(|s| s.kind == "elevator").collect();
    assert_eq!(lifts.len(), 3, "{lifts:#?}");
    assert!(lifts.iter().all(|s| s.stack == ground), "{lifts:#?}");
    let slot = doc.slot_of(one.parse().expect("a uid")).expect("1. kat's room");
    app.selection.set(vec![slot]);
    let panel = |app: &App| -> Vec<(String, String)> {
        let doc = app.document.as_ref().expect("open");
        app.properties_panel(doc).sections.into_iter().flat_map(|s| s.rows).map(|r| (r.label.to_string(), r.value)).collect()
    };
    let row = |app: &App, label: &str| {
        let rows = panel(app);
        let at = rows.iter().position(|r| r.0 == label).unwrap_or_else(|| panic!("no {label} row: {rows:#?}"));
        (rows[at].1.clone(), rows.get(at + 1).map(|r| r.1.clone()).unwrap_or_default())
    };
    assert_eq!(row(&app, "Baca alanı").0, "");
    let (said, note) = row(&app, "Basınçlandırılır");
    assert_eq!(said, "Hayır");
    assert_eq!(note, "Kuyu bu katta 10.64 m²; bir katta beyan yeter.");
    let commit = |app: &mut App, text: &str| {
        let _ = app.update(Message::Properties(Event::Commit(Field::Arch(slot, ArchField::ShaftVent), text.into())));
    };
    let own = |app: &App| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.get(slot).and_then(|e| e.arch_info()).and_then(|a| a.shaft_vent.clone())
    };
    let row_62_4 = |app: &App| rows(app).into_iter().find(|r| r.0 == "BYKHY 62/4").map(|r| r.1);
    // A word is no area: nothing written, said why.
    commit(&mut app, "yok");
    assert_eq!(own(&app), None);
    commit(&mut app, "0.2");
    assert_eq!(own(&app).and_then(|v| v.area), Some(0.2));
    assert_eq!(row(&app, "Baca alanı").0, "0.2");
    checked(&mut app);
    assert_eq!(row_62_4(&app), Some(O::Failed));
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    let found: Vec<&kentos_commands::ArchAuditFinding> =
        audited.findings.iter().filter(|f| f.code == "bykhy_elevator_shaft_vent").collect();
    assert_eq!(found.len(), 1, "one finding for the shaft through three storeys: {found:#?}");
    assert!(
        found[0].message.starts_with(
            "“Asansör”: havalandırma bacası 0.2 m² (kuyu alanı 10.64 m², en az 0.266 m² baca); 62/4 asansör kuyusunda"
        ),
        "{}",
        found[0].message
    );
    let mut uids = found[0].uids.clone();
    uids.sort();
    let mut three = vec![ground.clone(), one.clone(), two.clone()];
    three.sort();
    assert_eq!(uids, three);
    // Pressurised: the area stays, 62/4 holds.
    let _ = app.update(Message::Properties(Event::ShaftPressurized(slot, true)));
    let v = own(&app).expect("declared");
    assert!(v.pressurized && v.area == Some(0.2), "{v:?}");
    checked(&mut app);
    assert_eq!(row_62_4(&app), Some(O::Passed));
    // The area taken away: still pressurised; then not pressurised either: no declaration, 62/4 lacks it.
    commit(&mut app, "");
    assert_eq!(own(&app).map(|v| (v.area, v.pressurized)), Some((None, true)));
    let _ = app.update(Message::Properties(Event::ShaftPressurized(slot, false)));
    assert_eq!(own(&app), None);
    checked(&mut app);
    assert_eq!(row_62_4(&app), Some(O::MissingData));
    // Hol is no lift shaft: no row.
    let hol = room(&app, "Hol", "k00");
    let doc = &app.document.as_ref().expect("a drawing").model;
    app.selection.set(vec![doc.slot_of(hol.parse().expect("a uid")).expect("Hol")]);
    assert!(panel(&app).iter().all(|r| r.0 != "Baca alanı"));
}

/// Ek-3/B 8 and 10 (docs/adr/8136): Tesisat şaftı declared protected in Öznitelikler asks REI 120 of its four walls
/// (missing data while they declare nothing; held when they are REI 120); the wall at x = 12 between Ofis and Hol
/// declared a fire stop asks EI 30, and E 30 is not enough. Hol is no shaft: no Korunumlu row; a column has no
/// Yangın kesici row.
#[test]
fn a_protected_shaft_and_a_fire_stop_are_declared_in_oznitelikler() {
    use crate::properties::Event;
    let (mut app, walls, _) = office();
    let service = room(&app, "Tesisat şaftı", "k00");
    let slot_of = |app: &App, uid: &str| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.slot_of(uid.parse().expect("a uid")).expect("its slot")
    };
    let panel = |app: &App| -> Vec<(String, String)> {
        let doc = app.document.as_ref().expect("open");
        app.properties_panel(doc).sections.into_iter().flat_map(|s| s.rows).map(|r| (r.label.to_string(), r.value)).collect()
    };
    let row = |app: &App, label: &str| {
        let rows = panel(app);
        let at = rows.iter().position(|r| r.0 == label).unwrap_or_else(|| panic!("no {label} row: {rows:#?}"));
        (rows[at].1.clone(), rows.get(at + 1).map(|r| r.1.clone()).unwrap_or_default())
    };
    let shaft = slot_of(&app, &service);
    app.selection.set(vec![shaft]);
    assert_eq!(row(&app, "Korunumlu").0, "Bilinmiyor");
    let _ = app.update(Message::Properties(Event::Protected(shaft, Some(true))));
    let (said, note) = row(&app, "Korunumlu");
    assert_eq!((said.as_str(), note.as_str()), ("Evet", "Ek-3/B 8: duvarları en az REI 120; bir katta beyan yeter."));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let measured = facts.shafts.iter().find(|s| s.uid == service).expect("the service shaft");
    assert_eq!(measured.protected, Some(true));
    let ek3 = |app: &App, words: &str| -> Vec<(Option<kentos_commands::ArchAuditState>, String)> {
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        audited
            .findings
            .iter()
            .filter(|f| f.code == "bykhy_element_resistance" && f.message.contains(words))
            .map(|f| (f.state, f.message.clone()))
            .collect()
    };
    checked(&mut app);
    let found = ek3(&app, "Ek-3/B 8");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, Some(kentos_commands::ArchAuditState::MissingData));
    assert!(
        found[0]
            .1
            .starts_with("Zemin kat: 4 duvarın yangın sınıfı beyan edilmemiş; Ek-3/B 8 korunumlu şaft (her yüzü) en az REI 120"),
        "{}",
        found[0].1
    );
    declare(&mut app, &[&walls[1], &walls[5], &walls[6], &walls[7]], "REI 120");
    checked(&mut app);
    assert!(ek3(&app, "Ek-3/B 8").is_empty());
    // The wall between Ofis and Hol, a fire stop: E 30 lacks I.
    let partition = slot_of(&app, &walls[4]);
    app.selection.set(vec![partition]);
    assert_eq!(row(&app, "Yangın kesici").0, "Bilinmiyor");
    let _ = app.update(Message::Properties(Event::FireStop(partition, Some(true))));
    assert_eq!(row(&app, "Yangın kesici"), ("Evet".to_owned(), "Ek-3/B 10: en az EI 30, her yüzü.".to_owned()));
    declare(&mut app, &[&walls[4]], "E 30");
    checked(&mut app);
    let found = ek3(&app, "Ek-3/B 10");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].1.contains("1 duvar E 30; Ek-3/B 10 yangın kesici (her yüzü) en az EI 30 ister"), "{}", found[0].1);
    declare(&mut app, &[&walls[4]], "EI 30");
    checked(&mut app);
    assert!(ek3(&app, "Ek-3/B 10").is_empty());
    // Not declared again: the row says so, the facts carry nothing.
    let _ = app.update(Message::Properties(Event::FireStop(partition, None)));
    assert_eq!(row(&app, "Yangın kesici").0, "Bilinmiyor");
    let hol = slot_of(&app, &room(&app, "Hol", "k00"));
    app.selection.set(vec![hol]);
    assert!(panel(&app).iter().all(|r| r.0 != "Korunumlu"));
}

/// Öznitelikler of Tesisat şaftı declared protected and of the partition declared a fire stop, and Denetim with Ek-3/B
/// 8's finding chosen, light and dark; `.run/gui-evidence/korunumlu-saft-8136/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib shaft_tests::roles_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn roles_screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    use crate::properties::Event;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/korunumlu-saft-8136");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 1180.0), "acik"), ("dark", Size::new(1280.0, 1100.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, walls, _) = office();
        let slot_of = |app: &App, uid: &str| {
            let doc = &app.document.as_ref().expect("a drawing").model;
            doc.slot_of(uid.parse().expect("a uid")).expect("its slot")
        };
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::RibbonTab("arch"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        // `scroll`: the properties panel scrolled down this many steps (the wall's roles are under its geometry).
        let mut shot = |app: &mut App, file: &str, scroll: Option<f32>| {
            snapshot.settle(app, App::view, &mut update);
            if let Some(down) = scroll {
                let at = iced::Point::new(size.width - 150.0, size.height - 250.0);
                snapshot.input(app, App::view, &mut update, Input::Scroll(at, -down));
                snapshot.settle(app, App::view, &mut update);
            }
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let shaft = slot_of(&app, &room(&app, "Tesisat şaftı", "k00"));
        app.selection.set(vec![shaft]);
        let _ = app.update(Message::Properties(Event::Protected(shaft, Some(true))));
        shot(&mut app, "1-korunumlu-saft", None);
        let partition = slot_of(&app, &walls[4]);
        app.selection.set(vec![partition]);
        let _ = app.update(Message::Properties(Event::FireStop(partition, Some(true))));
        declare(&mut app, &[&walls[4]], "E 30");
        shot(&mut app, "2-yangin-kesici", Some(3.0));
        app.selection.set(Vec::new());
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected =
            audited.findings.iter().position(|f| f.code == "bykhy_element_resistance" && f.message.contains("Ek-3/B 8"));
        shot(&mut app, "3-denetim", None);
    }
}

/// Öznitelikler of 1. kat's lift shaft room with its vent typed, and Denetim with 62/4's finding chosen, light and dark;
/// `.run/gui-evidence/kuyu-havalandirmasi-8135/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib shaft_tests::vent_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn vent_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    use crate::properties::{ArchField, Event, Field};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kuyu-havalandirmasi-8135");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 1180.0), "acik"), ("dark", Size::new(1280.0, 1100.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, walls, doors) = office();
        declare(&mut app, &[&walls[1], &walls[2], &walls[5], &walls[6], &walls[7]], "EI 120");
        declare(&mut app, &[&doors[1]], "EI2 90-S200C5");
        declare(&mut app, &[&doors[0]], "E 30");
        let one = lift_room(&mut app, "k01");
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::RibbonTab("arch"));
        let slot = {
            let doc = &app.document.as_ref().expect("a drawing").model;
            doc.slot_of(one.parse().expect("a uid")).expect("1. kat's room")
        };
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.selection.set(vec![slot]);
        let _ = app.update(Message::Properties(Event::Commit(Field::Arch(slot, ArchField::ShaftVent), "0.2".into())));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-oznitelikler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_elevator_shaft_vent");
        app.selection.set(Vec::new());
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Zemin kat with its shafts and Denetim with 41/9's finding chosen, light and dark; `.run/gui-evidence/saft-8087/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/saft-8087");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, walls, doors) = office();
        declare(&mut app, &[&walls[1], &walls[2], &walls[5], &walls[6], &walls[7]], "EI 120");
        declare(&mut app, &[&doors[1]], "EI2 90-S200C5");
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_shaft_cover_stair");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-saft-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// A building over a basement (docs/adr/8092): Bodrum kat −3.00 and Zemin kat ±0.00, four flats; on Bodrum kat a 10 × 6 m
/// plan inside 20 cm walls, Depo (x 0–4), Koridor (4–8) and the lift's shaft Asansör (8–10), its sliding door in the
/// wall at x = 8 onto Koridor. The wall at x = 4 separates Koridor from Depo, declared `between`. Its slot.
fn basement(between: &str) -> (App, String) {
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
            {"id": "b01", "name": "Bodrum kat", "elevation": -3.0, "height": 3.0},
            {"id": "k00", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}
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
    let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": "b01"}});
    let walls = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (10.0, 0.0)), wall((10.0, 0.0), (10.0, 6.0)), wall((10.0, 6.0), (0.0, 6.0)),
            wall((0.0, 6.0), (0.0, 0.0)), wall((4.0, 0.0), (4.0, 6.0)), wall((8.0, 0.0), (8.0, 6.0))
        ]}),
    );
    let host = |i: usize| walls["output"]["uids"][i].as_str().expect("a wall").to_owned();
    let mut landing = json!({"host": host(5), "at": 3.0, "form": "door", "width": 1.0, "head": 2.1});
    landing["operation"] = json!("slide");
    let doors = run(&mut app, "cad.openings.create", json!({"layerId": layer, "openings": [landing]}));
    run(
        &mut app,
        "cad.arch.performance",
        json!({"operation": {"kind": "objects", "uids": [doors["output"]["uids"][0].clone()],
            "fireRating": {"value": "E 30", "basis": {"kind": "author"}}}}),
    );
    run(
        &mut app,
        "cad.arch.performance",
        json!({"operation": {"kind": "objects", "uids": [host(4)], "fireRating": {"value": between, "basis": {"kind": "author"}}}}),
    );
    let rect = |x0: f64, x1: f64| vec![p(x0, 0.1), p(x1, 0.1), p(x1, 5.9), p(x0, 5.9)];
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"parts": [{"outer": {"pts": rect(0.1, 3.9)}}], "name": "Depo", "arch": {"level": "b01"}},
            {"parts": [{"outer": {"pts": rect(4.1, 7.9)}}], "name": "Koridor", "arch": {"level": "b01"}},
            {"parts": [{"outer": {"pts": rect(8.1, 9.9)}}], "name": "Asansör", "void": true, "arch": {"level": "b01"}}
        ]}),
    );
    // The lift's shaft pressurised (62/4's first sentence, docs/adr/8135): these pictures and checks are its second's.
    let lift = room(&app, "Asansör", "b01");
    vent(&mut app, &lift, json!({"pressurized": true, "basis": {"kind": "author"}}));
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    (app, host(4))
}

/// 62/4's second sentence (docs/adr/8092): the lift serving the basement reached there through Koridor, separated from
/// Depo by EI 30, is not a protected corridor (Ek-3/C apartman, bodrum 10 m'den az: 60; 36/1-a); declared EI 60 it is.
#[test]
fn a_lift_serving_the_basement_is_reached_through_a_protected_corridor() {
    let (mut app, wall) = basement("EI 30");
    checked(&mut app);
    let row = |app: &App| rows(app).into_iter().find(|r| r.0 == "BYKHY 62/4").map(|r| r.1);
    assert_eq!(row(&app), Some(O::Failed));
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    let found: Vec<&String> =
        audited.findings.iter().filter(|f| f.code == "bykhy_elevator_basement_lobby").map(|f| &f.message).collect();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].starts_with("Bodrum kat: asansör kapısı korunmamış koridora açılıyor (“Koridor”, ayrımı en az EI 60 değil)"),
        "{found:#?}"
    );
    declare(&mut app, &[&wall], "EI 60");
    checked(&mut app);
    assert_eq!(row(&app), Some(O::Passed));
}

/// Bodrum kat and Denetim's 62/4 finding, light and dark; `.run/gui-evidence/bodrum-asansoru-8092/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib shaft_tests::basement_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn basement_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bodrum-asansoru-8092");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, _) = basement("EI 30");
        let _ = app.update(Message::ActiveLevel(Some("b01".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_elevator_basement_lobby");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-bodrum-asansoru-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
