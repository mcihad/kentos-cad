//! Şaftlar ve asansörler (docs/adr/8087; Binaların Yangından Korunması Hakkında Yönetmelik 4/ccc, 25/3, 41/9, 62/2, 62/4,
//! 62/6, 62/7 ve Ek-3/B 12) on an eight-storey office worked by hand: storeys of 3.00 m (bina ve yapı yüksekliği 24.00 m,
//! 21,50 m'den fazla: a high building). On Zemin kat a 20 × 10 m plan inside 20 cm walls: Ofis (x 0–12), Hol (12–17)
//! and east of the wall at x = 17 the elevator's shaft Asansör (y 6–10), the service shaft Tesisat şaftı (y 4.5–6) and
//! Merdiven evi (y 0–4.5), each drawn to its walls' faces. The elevator's landing door, sliding, 1.00 m in the wall at
//! x = 17, opens onto Hol; the shaft's 60 cm door, in the wall at y = 4.5, swings out into Merdiven evi (41/9 forbids it).
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib shaft_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::ArchAuditOutcome as O;
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
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The six articles' rows, in order (outcome, version).
fn rows(app: &App) -> Vec<(String, O, Option<String>)> {
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    ["BYKHY 25/3", "BYKHY 41/9", "BYKHY 62/2", "BYKHY 62/4", "BYKHY 62/6", "BYKHY 62/7"]
        .iter()
        .map(|rule| {
            let r = set.rules.iter().find(|r| r.rule == *rule).unwrap_or_else(|| panic!("{rule}: {:#?}", set.rules));
            (r.rule.clone(), r.state, r.version.clone())
        })
        .collect()
}

fn codes(app: &App) -> Vec<String> {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
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

/// Nothing declared: the high building's shaft and the elevator's shaft are missing data, so is the landing door's
/// class; the shaft's door onto Merdiven evi breaks 41/9; the landing door onto Hol keeps 62/4 and 62/6. Declared EI 120
/// round both shafts, the shaft's door EI2 90-S200C5 and the landing door E 30 (51,50 m'den alçak), every article holds
/// but 41/9.
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
            ("BYKHY 62/4".to_owned(), O::Passed, v("19.12.2007")),
            ("BYKHY 62/6".to_owned(), O::Passed, v("19.12.2007")),
            ("BYKHY 62/7".to_owned(), O::MissingData, v("9.7.2015")),
        ]
    );
    assert_eq!(
        codes(&app),
        [
            "bykhy_elevator_door_resistance",
            "bykhy_elevator_shaft",
            "bykhy_shaft_cover_stair",
            "bykhy_shaft_resistance"
        ]
    );
    declare(&mut app, &[&walls[1], &walls[2], &walls[5], &walls[6], &walls[7]], "EI 120");
    declare(&mut app, &[&doors[1]], "EI2 90-S200C5");
    declare(&mut app, &[&doors[0]], "E 30");
    checked(&mut app);
    let states: Vec<O> = rows(&app).into_iter().map(|r| r.1).collect();
    assert_eq!(states, [O::Passed, O::Failed, O::Passed, O::Passed, O::Passed, O::Passed]);
    assert_eq!(codes(&app), ["bykhy_shaft_cover_stair"]);
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let cover = audited.findings.iter().find(|f| f.code == "bykhy_shaft_cover_stair").expect("41/9");
    assert!(
        cover.message.starts_with("Şaft kapısı (“Tesisat şaftı”) kaçış merdiveni yuvasına açılıyor (“Merdiven evi”)"),
        "{}",
        cover.message
    );
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
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_shaft_cover_stair");
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
