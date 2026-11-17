//! Taşıyıcı taş duvarların kalın kısmı (docs/adr/8095; PAİY 5/8, 20/8-j) on a two-storey traditional cut stone house
//! worked by hand: 10 × 6 m outside, 50 cm stone walls round it (their axes 25 cm in), Zemin kat parted by a 40 cm stone
//! wall on x = 5 into a Salon and an Oda, 1. kat one Yatak odası; each storey 3.00 m; a 200 m² parcel of emsal 1.00.
//!
//! - **Exterior walls:** each storey's band along the outer face, 50 − 25 = 25 cm deep: 10 × 6 − 9.5 × 5.5 = 7.75 m².
//!   The room contours split it at the interior wall's axis: 3.875 m² each on Zemin kat, the corner pieces of the
//!   interior wall's façade ends halved as the gross is.
//! - **Interior wall:** two bands of (40 − 12.5) / 2 = 13.75 cm along its faces, between the exterior walls' inner faces
//!   (5.00 m): 2 × 0.1375 × 5 = 1.375 m², half in each room.
//! - **Katlar alanı:** 120 − (7.75 + 1.375 + 7.75) = 103.125 m²; the 16.875 m² out apart from the 30 %. **Taban alanı:**
//!   60 − 7.75 = 52.25 m² (the interior wall's part stays, 20/8-j reads the exterior walls).
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib stone_wall_tests::screens -- --ignored --nocapture
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

/// The house, declared a traditional cut stone masonry building, applied for on 2.7.2026; its walls by persistent id
/// (the exterior ones of Zemin kat and 1. kat, then the interior one), none yet declared load-bearing.
fn stone_house() -> (App, Vec<String>) {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": {
            "levels": [
                {"id": "k00", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                {"id": "k01", "name": "1. kat", "elevation": 3.0, "height": 3.0}
            ],
            "zoning": {"parcelArea": 200.0, "far": 1.0},
            "structure": {"system": "cutStoneMasonry", "traditional": true}
        } }),
    );
    run(&mut app, "cad.program.edit", json!({"program": {"units": [{"id": "d1", "number": "1", "nature": "Daire"}]}}));
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let mut walls: Vec<String> = Vec::new();
    for level in ["k00", "k01"] {
        let wall = |a: (f64, f64), b: (f64, f64), w: f64| {
            json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}})
        };
        let mut these = vec![
            wall((0.25, 0.25), (9.75, 0.25), 0.5),
            wall((9.75, 0.25), (9.75, 5.75), 0.5),
            wall((9.75, 5.75), (0.25, 5.75), 0.5),
            wall((0.25, 5.75), (0.25, 0.25), 0.5),
        ];
        if level == "k00" {
            these.push(wall((5.0, 0.25), (5.0, 5.75), 0.4));
        }
        let made = run(&mut app, "cad.walls.create", json!({"layerId": layer, "walls": these}));
        walls.extend(made["output"]["uids"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned));
    }
    let room = |x: f64, name: &str, level: &str| {
        json!({"seed": p(x, 3.0), "name": name, "unit": "d1", "arch": {"level": level}})
    };
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [room(2.5, "Salon", "k00"), room(7.5, "Oda", "k00"), room(5.0, "Yatak odası", "k01")]}),
    );
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    (app, walls)
}

/// The walls declared load-bearing (Öznitelikler › Taşıyıcı: evet).
fn bearing(app: &mut App, walls: &[String]) {
    run(
        app,
        "cad.arch.performance",
        json!({"operation": {"kind": "objects", "uids": walls, "loadBearing": {"bearing": true, "basis": {"kind": "author"}}}}),
    );
}

fn report(app: &App) -> kentos_domain::zoning::ZoningReport {
    app.document.as_ref().expect("a drawing").model.zoning_report().expect("a zoning")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

#[test]
fn a_traditional_stone_house_s_thick_walls_are_out_of_katlar_alani_and_the_outer_ones_of_taban_alani() {
    let (mut app, walls) = stone_house();
    assert_eq!(walls.len(), 9, "four walls round each storey and the one between the rooms");
    // Not yet declared load-bearing: every wall is thicker than 12.50 cm and undeclared; their parts count in.
    let r = report(&app);
    assert!(near(r.included, 120.0) && near(r.masonry, 0.0) && near(r.footprint, 60.0), "{r:?}");
    let mut undeclared = r.masonry_undeclared.clone();
    undeclared.sort();
    let mut all = walls.clone();
    all.sort();
    assert_eq!(undeclared, all);

    bearing(&mut app, &walls);
    let r = report(&app);
    assert!(r.masonry_undeclared.is_empty() && r.masonry_walls.len() == 9, "{r:?}");
    let room = |name: &str| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let uid = doc
            .entities_with_uids()
            .find_map(|(uid, e)| match e {
                kentos_commands::Entity::Space(s) if s.space.name.as_deref() == Some(name) => Some(uid.to_string()),
                _ => None,
            })
            .expect("the room");
        r.rooms.iter().find(|x| x.uid == uid).expect("counted").clone()
    };
    for (name, gross, walls) in [("Salon", 30.0, 3.875 + 0.6875), ("Oda", 30.0, 3.875 + 0.6875), ("Yatak odası", 60.0, 7.75)] {
        let x = room(name);
        assert!(near(x.gross, gross) && near(x.walls, walls), "{name}: {x:?}");
    }
    assert!(near(r.masonry, 16.875) && near(r.outside, 16.875), "{r:?}");
    assert!(near(r.included, 103.125) && near(r.floors, 103.125) && near(r.exempt, 0.0), "{r:?}");
    assert!(near(r.masonry_footprint, 7.75) && near(r.footprint, 52.25), "{r:?}");
    assert!(near(r.construction(), 120.0), "the yapı inşaat alanı keeps the walls: {r:?}");

    // The facts and Emsal ve TAKS tablosu's rows.
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let m = facts.masonry.expect("the stone walls");
    assert!(near(m.walls, 16.875) && near(m.footprint, 7.75) && m.thick.len() == 9 && m.undeclared.is_empty(), "{m:?}");
    let table = doc.zoning_table();
    let labels: Vec<&str> = table.rows.iter().map(|row| row[0].as_str()).collect();
    assert!(labels.contains(&"Taş duvarların kalın kısmı, %30 dışında (m²)"), "{labels:?}");
    assert!(labels.contains(&"Dış taş duvarların kalın kısmı, taban alanı dışında (m²)"), "{labels:?}");

    // The model's emsal zones: each storey's counted-in zone names its walls' part, the rest is the table's.
    let (b, _, _) = kentos_native_application::arch_ifc::model(doc);
    let dahil: Vec<String> = b
        .zones
        .iter()
        .filter_map(|z| z.description.clone())
        .filter(|d| d.contains("taşıyıcı yığma taş duvarların kalın kısmı"))
        .collect();
    assert_eq!(dahil.len(), 2, "{:?}", b.zones);
    assert!(dahil.iter().any(|d| d.contains("5/8: 7.75 m²")), "{dahil:?}");

    // Denetim: 20/8-j and 5/8 hold on 2.7.2026.
    checked(&mut app);
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    let rows: Vec<(&str, Option<&str>, O)> = set
        .rules
        .iter()
        .filter(|r| r.rule == "PAİY 20/8-j" || r.rule == "PAİY 5/8")
        .map(|r| (r.rule.as_str(), r.version.as_deref(), r.state))
        .collect();
    assert_eq!(rows, [("PAİY 20/8-j", Some("9.10.2021"), O::Passed), ("PAİY 5/8", Some("31.12.2022"), O::Passed)]);
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    assert!(!audited.findings.iter().any(|f| f.code.starts_with("paiy_masonry_")), "{:?}", audited.findings);

    // Betonarme: no stone walls' part.
    let mut arch = app.document.as_ref().expect("a drawing").model.settings().arch.clone().expect("arch");
    arch.structure = Some(kentos_commands::BuildingStructure {
        system: kentos_commands::StructuralSystem::ReinforcedConcrete,
        traditional: false,
    });
    run(&mut app, "cad.settings.set", json!({ "arch": serde_json::to_value(&arch).expect("serialises") }));
    let r = report(&app);
    assert!(near(r.masonry, 0.0) && near(r.included, 120.0) && near(r.footprint, 60.0), "{r:?}");
}

/// Emsal ve TAKS tablosu beside Zemin kat's 50 cm stone walls, and Denetim's rule set with 20/8-j and 5/8, light and
/// dark; `.run/gui-evidence/tas-duvar-8095/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tas-duvar-8095");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let (mut app, walls) = stone_house();
        bearing(&mut app, &walls);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        // The window settled first: the view's extents are its own.
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("arch.zoningTable"));
        let _ = app.submit_line("12,6");
        app.selection.set(Vec::new());
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-emsal-tablosu-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        checked(&mut app);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Sivas 22/k (docs/adr/8096) counts the walls' part within the 30 %: on the 200 m² parcel of emsal 1.00 its 60 m² holds
/// the 16.875 m²; on a 50 m² parcel its 15 m² does not (1.875 m² past it), while PAİY 5/8 still counts them apart.
#[test]
fn sivas_counts_the_thick_walls_within_the_30_percent() {
    let (mut app, walls) = stone_house();
    bearing(&mut app, &walls);
    let mut arch = app.document.as_ref().expect("a drawing").model.settings().arch.clone().expect("arch");
    arch.local_rules = Some(kentos_commands::LocalRules::Sivas);
    run(&mut app, "cad.settings.set", json!({ "arch": serde_json::to_value(&arch).expect("serialises") }));
    let rows = |app: &App| -> Vec<(String, O)> {
        let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
        set.rules
            .iter()
            .filter(|r| ["PAİY 5/8", "Sivas 22/k"].contains(&r.rule.as_str()))
            .map(|r| (r.rule.clone(), r.state))
            .collect()
    };
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let cap = facts.exempt_cap.expect("the 30 %");
    assert!(near(cap.cap, 60.0) && near(cap.exempt, 0.0), "{cap:?}");
    checked(&mut app);
    assert_eq!(rows(&app), [("PAİY 5/8".to_owned(), O::Passed), ("Sivas 22/k".to_owned(), O::Passed)]);

    arch.zoning = Some(kentos_commands::Zoning { parcel_area: 50.0, far: Some(1.0), coverage: None });
    run(&mut app, "cad.settings.set", json!({ "arch": serde_json::to_value(&arch).expect("serialises") }));
    checked(&mut app);
    assert_eq!(rows(&app), [("PAİY 5/8".to_owned(), O::Passed), ("Sivas 22/k".to_owned(), O::Failed)]);
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let f = audited.findings.iter().find(|f| f.code == "local_masonry_walls_over").expect("Sivas's finding");
    assert!(
        f.message.contains("emsal harici 16.88 m², sınır 15.00 m²; aşan 1.88 m² emsale girer"),
        "{}",
        f.message
    );
    assert_eq!(f.uids.len(), 9, "the thick stone walls: {f:?}");
}
