//! Yapı elemanlarının yangına dayanım süreleri (docs/adr/8082; BYKHY Ek-3/B, Ek-3/C) on a two-storey house, worked by
//! hand: Zemin kat ±0.00 and 1. kat +3.00, each 3.00 m (bina yüksekliği 6.00 m, no basement); on each a 10 × 5 m plan
//! inside 25 cm walls; 1. kat's floor slab; a 40 × 40 cm column of the frame on Zemin kat. One dwelling: Ek-3/C's
//! "bir ve iki ailelik evler" row, 21,50 m'den az: 60; the second storey's floor of a two-storey house REI 30 (Ek-3/B
//! 3-a). Zemin kat's walls are declared REI 60, the slab REI 60, the column as each test says; 1. kat's walls not.

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

/// The house, its column declared `column`.
fn house(column: &str) -> App {
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
    run(&mut app, "cad.program.edit", json!({"program": {"units": [{"id": "d1", "number": "1", "nature": "Daire"}]}}));
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let mut walls: Vec<Value> = Vec::new();
    for level in ["k00", "k01"] {
        let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.25, "arch": {"level": level}});
        let made = run(&mut app, "cad.walls.create", json!({"layerId": layer, "walls": [
            wall((0.125, 0.125), (9.875, 0.125)), wall((9.875, 0.125), (9.875, 4.875)),
            wall((9.875, 4.875), (0.125, 4.875)), wall((0.125, 4.875), (0.125, 0.125))
        ]}));
        if level == "k00" {
            walls = made["output"]["uids"].as_array().cloned().unwrap_or_default();
        }
    }
    let slab = run(&mut app, "cad.slabs.create", json!({"layerId": layer, "slabs": [{
        "parts": [{"outer": {"pts": [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 5.0), p(0.0, 5.0)]}}],
        "thickness": 0.15, "arch": {"level": "k01"}
    }]}));
    let col = run(&mut app, "cad.columns.create", json!({"layerId": layer, "columns": [
        {"at": p(5.0, 2.5), "width": 0.4, "depth": 0.4, "arch": {"level": "k00"}}
    ]}));
    let declare = |app: &mut App, uids: Vec<Value>, value: &str| {
        run(
            app,
            "cad.arch.performance",
            json!({"operation": {"kind": "objects", "uids": uids, "fireRating": {"value": value, "basis": {"kind": "author"}}}}),
        );
    };
    declare(&mut app, walls, "REI 60");
    declare(&mut app, vec![slab["output"]["uids"][0].clone()], "REI 60");
    declare(&mut app, vec![col["output"]["uids"][0].clone()], column);
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    app
}

/// The facts: the column of the frame, the eight walls all on their storeys' outlines, the floor; 6.00 m high, no
/// basement.
#[test]
fn the_facts_carry_the_elements_ek_3_reads() {
    let app = house("R 30");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let count = |kind: &str| facts.elements.iter().filter(|e| e.kind == kind).count();
    assert_eq!((count("column"), count("wall"), count("slab"), count("beam")), (1, 8, 1, 0), "{:?}", facts.elements);
    assert!(facts.elements.iter().filter(|e| e.kind == "wall").all(|e| e.exterior), "{:?}", facts.elements);
    let column = facts.elements.iter().find(|e| e.kind == "column").expect("the column");
    assert!(column.frame && column.fire_rating.as_deref() == Some("R 30"), "{column:?}");
    let declared = facts.elements.iter().filter(|e| e.kind == "wall" && e.fire_rating.as_deref() == Some("REI 60")).count();
    assert_eq!(declared, 4, "Zemin kat's walls");
    assert!((facts.height - 6.0).abs() < 1e-9 && facts.basement_depth == 0.0, "{} {}", facts.height, facts.basement_depth);
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.modules.arch().arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The column's R 30 is under Ek-3/C's 60; 1. kat's walls declare nothing (eksik veri); the slab's REI 60 is over the
/// second storey's REI 30 and Zemin kat's walls' REI 60 is enough. Declared R 60, only the walls' missing classes stay.
#[test]
fn the_column_s_class_is_read_against_ek_3_c() {
    use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S};
    let mut app = house("R 30");
    checked(&mut app);
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    let found: Vec<(Option<S>, &str)> = audited
        .findings
        .iter()
        .filter(|f| f.code == "bykhy_element_resistance")
        .map(|f| (f.state, f.message.as_str()))
        .collect();
    assert!(found.iter().any(|(s, m)| s.is_none() && m.contains("kolon R 30") && m.contains("R 60")), "{found:#?}");
    assert!(found.iter().any(|(s, m)| *s == Some(S::MissingData) && m.contains("1. kat: 4 duvarın")), "{found:#?}");
    assert_eq!(found.len(), 2, "{found:#?}");
    let row = |app: &App| {
        let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
        set.rules.iter().find(|r| r.rule == "BYKHY Ek-3").map(|r| r.state)
    };
    assert_eq!(row(&app), Some(O::Failed));
    let mut fixed = house("R 60");
    checked(&mut fixed);
    assert_eq!(row(&fixed), Some(O::MissingData));
}

/// The house's ground storey and Denetim's Ek-3 finding of the column, light and dark;
/// `.run/gui-evidence/dayanim-8082/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib resistance_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dayanim-8082");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = house("R 30");
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected =
            audited.findings.iter().position(|f| f.code == "bykhy_element_resistance" && f.message.contains("kolon"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-dayanim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Zemin kat's suspended ceiling (docs/adr/8089): an Alçıpan covering of IFC type CEILING, 12.5 mm, 2.60 m over the
/// floor, under the 10 × 5 m plan's inner faces; its persistent id.
fn ceiling(app: &mut App) -> Value {
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let made = run(
        app,
        "cad.slabs.create",
        json!({"layerId": layer, "slabs": [{
            "parts": [{"outer": {"pts": [p(0.25, 0.25), p(9.75, 0.25), p(9.75, 4.75), p(0.25, 4.75)]}}],
            "thickness": 0.0125, "offset": 2.6, "material": "Alçıpan", "arch": {"level": "k00"},
            "covering": {"predefined": "CEILING", "name": "Asma tavan"}
        }]}),
    );
    made["output"]["uids"][0].clone()
}

/// The ceiling's Ek-3 findings (state, message).
fn ceiling_said(app: &App) -> Vec<(Option<kentos_commands::ArchAuditState>, String)> {
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    audited
        .findings
        .iter()
        .filter(|f| f.code == "bykhy_element_resistance" && f.message.contains("asma tavan"))
        .map(|f| (f.state, f.message.clone()))
        .collect()
}

/// A suspended ceiling is Ek-3/B 11's (docs/adr/8089): EI 30 from below, whatever Ek-3/C asks of the storey.
/// Undeclared it is eksik veri; declared EI 15 it fails; EI 30 holds.
#[test]
fn a_suspended_ceiling_is_read_by_ek_3_b_11() {
    use kentos_commands::ArchAuditState as S;
    let mut app = house("R 60");
    let uid = ceiling(&mut app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let ceilings: Vec<_> = facts.elements.iter().filter(|e| e.kind == "ceiling").collect();
    assert_eq!(ceilings.len(), 1, "{:?}", facts.elements);
    assert_eq!((ceilings[0].uid.as_str(), ceilings[0].level.as_deref()), (uid.as_str().expect("an id"), Some("k00")));
    assert_eq!(facts.elements.iter().filter(|e| e.kind == "slab").count(), 1, "the covering is no floor slab");
    checked(&mut app);
    let said = ceiling_said(&app);
    assert_eq!(said.len(), 1, "{said:#?}");
    assert_eq!(said[0].0, Some(S::MissingData));
    assert!(
        said[0].1.starts_with(
            "Zemin kat: 1 asma tavanın yangın sınıfı beyan edilmemiş; Ek-3/B 11 asma tavan (alt taraftan) en az EI 30 ister"
        ),
        "{said:#?}"
    );
    let declare = |app: &mut App, value: &str| {
        run(
            app,
            "cad.arch.performance",
            json!({"operation": {"kind": "objects", "uids": [uid.clone()], "fireRating": {"value": value, "basis": {"kind": "author"}}}}),
        );
    };
    declare(&mut app, "EI 15");
    checked(&mut app);
    let said = ceiling_said(&app);
    assert_eq!(said.len(), 1, "{said:#?}");
    assert_eq!(said[0].0, None);
    assert!(said[0].1.starts_with("Zemin kat: 1 asma tavan EI 15; Ek-3/B 11"), "{said:#?}");
    declare(&mut app, "EI 30");
    checked(&mut app);
    assert!(ceiling_said(&app).is_empty());
}

/// Zemin kat with its suspended ceiling and Denetim's Ek-3/B 11 finding, light and dark;
/// `.run/gui-evidence/asma-tavan-8089/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib resistance_tests::ceiling_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn ceiling_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/asma-tavan-8089");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = house("R 60");
        let uid = ceiling(&mut app);
        run(
            &mut app,
            "cad.arch.performance",
            json!({"operation": {"kind": "objects", "uids": [uid], "fireRating": {"value": "EI 15", "basis": {"kind": "author"}}}}),
        );
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected =
            audited.findings.iter().position(|f| f.code == "bykhy_element_resistance" && f.message.contains("asma tavan"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-asma-tavan-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Zemin kat's inner wall at x = 3 (docs/adr/8090), 20 cm, from wall face to wall face; its slot.
fn inner_wall(app: &mut App) -> kentos_domain::Slot {
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let made = run(
        app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [{"pts": [p(3.0, 0.25), p(3.0, 4.75)], "width": 0.2, "arch": {"level": "k00"}}]}),
    );
    let uid = made["output"]["uids"][0].as_str().expect("the wall").to_owned();
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.slot_of(kentos_domain::Uuid::parse_str(&uid).expect("an id")).expect("its slot")
}

fn property_rows(app: &App) -> Vec<crate::properties::Row> {
    let doc = app.document.as_ref().expect("open");
    app.properties_panel(doc).sections.into_iter().flat_map(|s| s.rows).collect()
}

/// A wall's Taşıyıcı (docs/adr/8090) chosen in Öznitelikler: Bilinmiyor until said; Evet makes the inner wall Ek-3/B 2's,
/// R as Ek-3/C asks (the house: 60), its class undeclared eksik veri; declared REI 60 it holds. Hayır: a partition,
/// nothing asked.
#[test]
fn a_load_bearing_wall_is_read_by_ek_3_b_2() {
    use crate::properties::Event;
    use kentos_commands::ArchAuditState as S;
    let mut app = house("R 60");
    let wall = inner_wall(&mut app);
    app.selection.set(vec![wall]);
    assert!(property_rows(&app).iter().any(|r| r.label == "Taşıyıcı" && r.value == "Bilinmiyor"));
    let _ = app.update(crate::app::Message::Properties(Event::LoadBearing(wall, Some(true))));
    assert!(property_rows(&app).iter().any(|r| r.label == "Taşıyıcı" && r.value == "Evet"));
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.undo_labels().first().map(String::as_str), Some("Yangın ve performans"));
    let wall_said = |app: &App| -> Vec<(Option<S>, String)> {
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        audited
            .findings
            .iter()
            .filter(|f| f.code == "bykhy_element_resistance" && f.message.contains("Ek-3/B 2"))
            .map(|f| (f.state, f.message.clone()))
            .collect()
    };
    checked(&mut app);
    let said = wall_said(&app);
    assert_eq!(said.len(), 1, "{said:#?}");
    assert_eq!(said[0].0, Some(S::MissingData));
    assert!(
        said[0].1.starts_with("Zemin kat: 1 duvarın yangın sınıfı beyan edilmemiş; Ek-3/B 2 yük taşıyıcı duvar; Ek-3/C"),
        "{said:#?}"
    );
    assert!(said[0].1.contains("en az R 60"), "{said:#?}");
    let _ = app.update(crate::app::Message::Properties(Event::Commit(
        crate::properties::Field::Arch(wall, crate::properties::ArchField::FireRating),
        "REI 60".into(),
    )));
    checked(&mut app);
    assert!(wall_said(&app).is_empty());
    // A partition: nothing asked of it.
    let _ = app.update(crate::app::Message::Properties(Event::LoadBearing(wall, Some(false))));
    let _ = app.update(crate::app::Message::Properties(Event::Commit(
        crate::properties::Field::Arch(wall, crate::properties::ArchField::FireRating),
        String::new(),
    )));
    checked(&mut app);
    assert!(wall_said(&app).is_empty());
}

/// The inner wall chosen, Öznitelikler with Taşıyıcı Evet and Denetim's Ek-3/B 2 finding, light and dark;
/// `.run/gui-evidence/tasiyici-8090/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib resistance_tests::bearing_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn bearing_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tasiyici-8090");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = house("R 60");
        let wall = inner_wall(&mut app);
        let _ = app.update(Message::Properties(crate::properties::Event::LoadBearing(wall, Some(true))));
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected =
            audited.findings.iter().position(|f| f.code == "bykhy_element_resistance" && f.message.contains("Ek-3/B 2"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.selection.set(vec![wall]);
        snapshot.settle(&mut app, App::view, &mut update);
        // Öznitelikler scrolled to Yangın dayanımı and Taşıyıcı.
        let panel = iced::Point::new(size.width - 150.0, size.height - 120.0);
        snapshot.input(&mut app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(panel, -4.0));
        let file = out.join(format!("1-tasiyici-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
