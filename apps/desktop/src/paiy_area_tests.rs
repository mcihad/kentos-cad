//! Planlı Alanlar İmar Yönetmeliği 4's areas (M29, docs/adr/3029) on a building a hand can check: 15 × 6 m outside, its
//! exterior walls 25 cm, two 15 cm walls at x = 5 and 10; the west and east rooms two independent units, the middle one
//! the common stair hall, shared equally.
//!
//! - BB brüt alanı: the contour, the façade's outer face and the shared walls' axis: 5 × 6 = 30 m² each.
//! - BB net alanı: between the plastered faces: (4.925 − 0.25) × (5.75 − 0.25) = 25.71 m² for a unit.
//! - Ortak alan payı: the hall's gross 30 m² halved, 15 m²; BB genel brüt alanı 45 m².

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

fn command(app: &mut App, name: &str, input: Value) -> Value {
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

#[test]
fn a_units_schedule_writes_the_regulation_s_net_gross_share_and_general_gross() {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(
        Some(Module::Architecture),
        None,
        Settings::memory(),
        Recovery::off(),
    );
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    let (layer, level) = {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let level = doc
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.levels.first())
            .map(|l| l.id.clone())
            .expect("a storey");
        (doc.layers().active().to_owned(), level)
    };
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}});
    command(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.125, 0.125), (14.875, 0.125), 0.25), wall((14.875, 0.125), (14.875, 5.875), 0.25),
            wall((14.875, 5.875), (0.125, 5.875), 0.25), wall((0.125, 5.875), (0.125, 0.125), 0.25),
            wall((5.0, 0.125), (5.0, 5.875), 0.15), wall((10.0, 0.125), (10.0, 5.875), 0.15)
        ]}),
    );
    command(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "kind": "unit"},
            {"id": "ortak", "number": "O", "kind": "common"},
            {"id": "d2", "number": "2", "kind": "unit"}
        ], "shareRule": "equal"}}),
    );
    let room =
        |x: f64, unit: &str| json!({"seed": p(x, 3.0), "arch": {"level": level}, "unit": unit});
    command(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [room(2.5, "d1"), room(7.5, "ortak"), room(12.5, "d2")]}),
    );
    let answer = command(
        &mut app,
        "cad.schedules.create",
        json!({"layerId": layer, "at": p(20.0, 0.0), "of": "units", "columns": [
            {"field": "number"}, {"field": "paiyNet"}, {"field": "paiyGross"}, {"field": "paiyShare"}, {"field": "generalGross"}
        ]}),
    );
    let cells: Vec<Vec<String>> =
        serde_json::from_value(answer["output"]["table"]["cells"].clone())
            .or_else(|_| {
                let doc = &app.document.as_ref().expect("a drawing").model;
                let t = doc
                    .entities()
                    .find_map(|e| match e {
                        kentos_commands::Entity::Table(t) => Some(t.cells.clone()),
                        _ => None,
                    })
                    .expect("the schedule");
                Ok::<_, serde_json::Error>(t)
            })
            .expect("cells");
    assert_eq!(
        cells[0],
        [
            "No",
            "BB net alanı (m²)",
            "BB brüt alanı (m²)",
            "Ortak alan payı (m²)",
            "BB genel brüt alanı (m²)"
        ],
        "{cells:?}"
    );
    assert_eq!(
        cells[1],
        ["1", "25.71", "30.00", "15.00", "45.00"],
        "{cells:?}"
    );
    assert_eq!(cells[2], ["O", "26.68", "30.00", "", ""], "{cells:?}");
    assert_eq!(
        cells[3],
        ["2", "25.71", "30.00", "15.00", "45.00"],
        "{cells:?}"
    );
    // The real window's drawing (fixtures/interaction/real/yonetmelik-alanlari.txt): the building without its schedule.
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        let _ = app.update(Message::Run("edit.undo"));
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("alan.kcad"), bytes).expect("written");
    }
}

/// Metrekare cetveli (M29, docs/adr/3029; PAİY 57/2-a): two storeys of the same plan, the stair hall one common area on
/// both; the units schedule storey by storey: each storey's units and its part of the hall, 30 m² each, the total the
/// building's 180 m².
#[test]
fn a_metrekare_cetveli_lists_each_storey_s_units_and_common_areas() {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(
        Some(Module::Architecture),
        None,
        Settings::memory(),
        Recovery::off(),
    );
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    command(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            { "id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0 },
            { "id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0 } ] } }),
    );
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    for level in ["zemin", "k1"] {
        let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}});
        command(
            &mut app,
            "cad.walls.create",
            json!({"layerId": layer, "walls": [
                wall((0.125, 0.125), (14.875, 0.125), 0.25), wall((14.875, 0.125), (14.875, 5.875), 0.25),
                wall((14.875, 5.875), (0.125, 5.875), 0.25), wall((0.125, 5.875), (0.125, 0.125), 0.25),
                wall((5.0, 0.125), (5.0, 5.875), 0.15), wall((10.0, 0.125), (10.0, 5.875), 0.15)
            ]}),
        );
    }
    command(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "kind": "unit"}, {"id": "d2", "number": "2", "kind": "unit"},
            {"id": "d3", "number": "3", "kind": "unit"}, {"id": "d4", "number": "4", "kind": "unit"},
            {"id": "ortak", "number": "O", "name": "Merdiven evi", "kind": "common"}
        ], "shareRule": "equal"}}),
    );
    let room = |x: f64, level: &str, unit: &str| json!({"seed": p(x, 3.0), "arch": {"level": level}, "unit": unit});
    command(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            room(2.5, "zemin", "d1"), room(7.5, "zemin", "ortak"), room(12.5, "zemin", "d2"),
            room(2.5, "k1", "d3"), room(7.5, "k1", "ortak"), room(12.5, "k1", "d4")
        ]}),
    );
    command(
        &mut app,
        "cad.schedules.create",
        json!({"layerId": layer, "at": p(20.0, 0.0), "of": "units", "byLevel": true, "total": true, "columns": [
            {"field": "level"}, {"field": "number"}, {"field": "unitKind"}, {"field": "paiyGross"}
        ]}),
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let cells = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Table(t) => Some(t.cells.clone()),
            _ => None,
        })
        .expect("the schedule");
    let rows: Vec<Vec<&str>> = cells
        .iter()
        .map(|r| r.iter().map(String::as_str).collect())
        .collect();
    assert_eq!(
        rows,
        vec![
            vec!["Kat", "No", "Bölüm türü", "BB brüt alanı (m²)"],
            vec!["Zemin kat", "1", "Bağımsız bölüm", "30.00"],
            vec!["Zemin kat", "2", "Bağımsız bölüm", "30.00"],
            vec!["Zemin kat", "O", "Ortak alan", "30.00"],
            vec!["1. kat", "3", "Bağımsız bölüm", "30.00"],
            vec!["1. kat", "4", "Bağımsız bölüm", "30.00"],
            vec!["1. kat", "O", "Ortak alan", "30.00"],
            vec!["Toplam", "", "", "180.00"],
        ],
        "{cells:?}"
    );
    // The real window's drawing (fixtures/interaction/real/metrekare-cetveli.txt): the building without its schedule.
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        let _ = app.update(Message::Run("edit.undo"));
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("kat.kcad"), bytes).expect("written");
    }
}

/// The ground storey of the first test, its middle room an Otopark (22/1-h: emsal harici by its name): the app, the
/// project's Mimari settings, the walls' ids (the four exterior ones, then x = 5 and x = 10).
fn parking_storey() -> (App, kentos_commands::ArchSettings, Vec<String>) {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(
        Some(Module::Architecture),
        None,
        Settings::memory(),
        Recovery::off(),
    );
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    let (layer, level, arch) = {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let arch = doc.settings().arch.clone().expect("Mimari's settings");
        let level = arch.levels.first().map(|l| l.id.clone()).expect("a storey");
        (doc.layers().active().to_owned(), level, arch)
    };
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}});
    let walls = command(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.125, 0.125), (14.875, 0.125), 0.25), wall((14.875, 0.125), (14.875, 5.875), 0.25),
            wall((14.875, 5.875), (0.125, 5.875), 0.25), wall((0.125, 5.875), (0.125, 0.125), 0.25),
            wall((5.0, 0.125), (5.0, 5.875), 0.15), wall((10.0, 0.125), (10.0, 5.875), 0.15)
        ]}),
    );
    let walls: Vec<String> = walls["output"]["uids"]
        .as_array()
        .expect("the walls")
        .iter()
        .filter_map(|w| w.as_str().map(str::to_owned))
        .collect();
    let room =
        |x: f64, name: &str| json!({"seed": p(x, 3.0), "arch": {"level": level}, "name": name});
    command(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [room(2.5, "Salon"), room(7.5, "Otopark"), room(12.5, "Salon")]}),
    );
    (app, arch, walls)
}

/// `arch` with the parcel's zoning, set as Proje ayarları sets it.
fn zone(app: &mut App, arch: &kentos_commands::ArchSettings, parcel: f64, far: f64, coverage: f64) {
    let mut a = serde_json::to_value(arch).expect("the settings");
    a["zoning"] = json!({"parcelArea": parcel, "far": far, "coverage": coverage});
    command(app, "cad.settings.set", json!({"arch": a}));
}

/// Emsal and TAKS (M30, docs/adr/3030): the same ground storey, its middle room an Otopark (22/1-h: emsal harici by its
/// name). On a 300 m² parcel, emsal 1.50, TAKS 0.40: katlar alanı 60 (the two flats), harici 30 within the 30 % cap
/// of 135, emsal 0.20; taban alanı 90, TAKS 0.30. On an 80 m² parcel with emsal 1.00 the cap is 24: the 6 past it
/// count in, emsal (60 + 6) / 80 = 0.825, and TAKS 90 / 80 passes 0.40.
#[test]
fn emsal_and_taks_read_the_rooms_against_the_parcel() {
    let (mut app, arch, _) = parking_storey();
    let mut zoned = |parcel: f64, far: f64, coverage: f64| {
        zone(&mut app, &arch, parcel, far, coverage);
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .zoning_report()
            .expect("a report")
    };
    let r = zoned(300.0, 1.5, 0.4);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
    assert!(
        near(r.included, 60.0) && near(r.exempt, 30.0) && near(r.outside, 0.0),
        "{r:?}"
    );
    assert!(
        near(r.cap.unwrap_or_default(), 135.0) && near(r.over, 0.0),
        "{r:?}"
    );
    assert!(
        near(r.far(), 0.2) && near(r.footprint, 90.0) && near(r.coverage(), 0.3),
        "{r:?}"
    );
    assert!(!r.far_over() && !r.coverage_over());
    assert_eq!(r.levels.len(), 1, "{r:?}");
    assert!(
        near(r.levels[0].included, 60.0)
            && near(r.levels[0].exempt, 30.0)
            && near(r.construction(), 90.0)
    );
    let r = zoned(80.0, 1.0, 0.4);
    assert!(
        near(r.cap.unwrap_or_default(), 24.0) && near(r.over, 6.0) && near(r.floors, 66.0),
        "{r:?}"
    );
    assert!(near(r.far(), 0.825) && !r.far_over(), "{r:?}");
    assert!(near(r.coverage(), 1.125) && r.coverage_over(), "{r:?}");
}

/// Emsal ve TAKS tablosu (M30, docs/adr/3030): placed from Metraj ▾, it writes PAİY 57/3's values against the zoning
/// and each storey's areas; a smaller parcel rewrites it at once (TAKS 0.45 past 0.40, its value and what is left in
/// the warning colour), a wall moved rewrites it in the same step and one undo takes both back.
#[test]
fn an_emsal_and_taks_table_follows_the_zoning_and_the_rooms() {
    let (mut app, arch, walls) = parking_storey();
    // Without a zoning it says where the parcel is typed and opens Proje ayarları on Mimari.
    let _ = app.update(Message::Run("arch.zoningTable"));
    assert!(
        crate::files_testing::last_said(&app).starts_with("Projede imar durumu yok"),
        "{}",
        crate::files_testing::last_said(&app)
    );
    assert!(app.project.is_some(), "Proje ayarları open");
    app.project = None;
    app.dialog = None;
    zone(&mut app, &arch, 300.0, 1.5, 0.4);
    let _ = app.update(Message::Run("arch.zoningTable"));
    assert_eq!(
        app.session.tool_id(),
        "tablePlace",
        "placed from the cursor"
    );
    let _ = app.submit_line("20,6");
    let table = |app: &App| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities()
            .find_map(|e| match e {
                kentos_commands::Entity::Table(t)
                    if matches!(t.source, Some(kentos_commands::TableSource::Zoning {})) =>
                {
                    Some(t.clone())
                }
                _ => None,
            })
            .expect("the Emsal ve TAKS table")
    };
    fn rows(t: &kentos_commands::TableEntity) -> Vec<Vec<&str>> {
        t.cells
            .iter()
            .map(|r| r.iter().map(String::as_str).collect())
            .collect()
    }
    let t = table(&app);
    assert_eq!(
        rows(&t),
        [
            vec!["Emsal ve TAKS", "İmar durumu", "Proje", "Kalan"],
            vec!["Parsel alanı (m²)", "300.00", "", ""],
            vec!["Emsal (KAKS)", "1.50", "0.20", ""],
            vec!["Emsale konu alan (m²)", "450.00", "60.00", "390.00"],
            vec!["Emsal harici, %30 içinde (m²)", "135.00", "30.00", "105.00"],
            vec!["TAKS", "0.40", "0.30", ""],
            vec!["Taban alanı (m²)", "120.00", "90.00", "30.00"],
            vec!["Yapı inşaat alanı (m²)", "", "90.00", ""],
            vec![
                "Kat",
                "Emsale dahil (m²)",
                "Emsal harici (m²)",
                "Kat alanı (m²)"
            ],
            vec!["Zemin kat", "60.00", "30.00", "90.00"],
            vec!["Toplam", "60.00", "30.00", "90.00"],
        ]
    );
    let over = |t: &kentos_commands::TableEntity| -> Vec<(u32, u32)> {
        t.styles
            .iter()
            .filter(|s| s.color.as_deref() == Some(kentos_domain::zoning::OVER_COLOR))
            .map(|s| (s.range.row, s.range.col))
            .collect()
    };
    assert!(over(&t).is_empty(), "nothing past its limit");
    // A 200 m² parcel: TAKS 90 / 200 = 0.45 past 0.40, the footprint 10 m² past the 80 allowed; written at once.
    zone(&mut app, &arch, 200.0, 1.5, 0.4);
    let t = table(&app);
    assert_eq!(rows(&t)[5], ["TAKS", "0.40", "0.45", ""]);
    assert_eq!(
        rows(&t)[6],
        ["Taban alanı (m²)", "80.00", "90.00", "-10.00"]
    );
    assert_eq!(over(&t), [(5, 2), (6, 2)], "{:?}", t.styles);
    // The wall at x = 5 a metre east: the west flat 6 m wide, the Otopark 4 m; the table in the same step.
    let before = rows(&table(&app))
        .iter()
        .map(|r| r.join("|"))
        .collect::<Vec<_>>();
    command(
        &mut app,
        "cad.entities.transform",
        json!({"uids": [walls[4]], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}),
    );
    let t = table(&app);
    assert_eq!(
        rows(&t)[9],
        ["Zemin kat", "66.00", "24.00", "90.00"],
        "{:?}",
        rows(&t)
    );
    let _ = app.update(Message::Run("edit.undo"));
    let after = rows(&table(&app))
        .iter()
        .map(|r| r.join("|"))
        .collect::<Vec<_>>();
    assert_eq!(after, before, "one undo takes the wall and the table back");
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("imar.kcad"), bytes).expect("written");
    }
}

/// Kat irtifakı tablosu (M31, docs/adr/3031; Kat Mülkiyeti Kanunu 12/a, PAİY 57/3): the two storeys of the metrekare
/// cetveli, the east room upstairs flat 1's annex (Depo), flat 3 worth 1.2 for its place, the hall one common area
/// shared equally, a 450 m² parcel. Flat 1's total gross is its 30 m² and the Depo's 30, its general gross 80 with its
/// 20 of the hall; the land shares are 60 : 30 : 36 of 450: 214, 107 and 129 (the one left to the largest remainder).
#[test]
fn a_kat_irtifaki_table_lists_the_units_with_their_annexes_and_land_shares() {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(
        Some(Module::Architecture),
        None,
        Settings::memory(),
        Recovery::off(),
    );
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    command(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            { "id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0 },
            { "id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0 } ],
            "zoning": { "parcelArea": 450.0, "far": 1.5, "coverage": 0.4 } } }),
    );
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    for level in ["zemin", "k1"] {
        let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}});
        command(
            &mut app,
            "cad.walls.create",
            json!({"layerId": layer, "walls": [
                wall((0.125, 0.125), (14.875, 0.125), 0.25), wall((14.875, 0.125), (14.875, 5.875), 0.25),
                wall((14.875, 5.875), (0.125, 5.875), 0.25), wall((0.125, 5.875), (0.125, 0.125), 0.25),
                wall((5.0, 0.125), (5.0, 5.875), 0.15), wall((10.0, 0.125), (10.0, 5.875), 0.15)
            ]}),
        );
    }
    // An annex without its unit is refused by the program's own rule.
    let doc = app.document.as_mut().expect("a drawing");
    let refused = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.program.edit",
        kentos_commands::CallOp::Execute,
        json!({"program": {"units": [{"id": "e1", "number": "E1", "kind": "annex"}]}}),
    )
    .expect("answers");
    assert_ne!(refused["status"], "completed", "{refused}");
    assert!(
        refused
            .to_string()
            .contains("hangi bağımsız bölüme ait olduğu seçilmeli"),
        "{refused}"
    );
    command(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "nature": "Daire"}, {"id": "d2", "number": "2", "nature": "Daire"},
            {"id": "d3", "number": "3", "nature": "Daire", "landFactor": "1.2"},
            {"id": "e1", "number": "E1", "kind": "annex", "nature": "Depo", "annexOf": "d1"},
            {"id": "ortak", "number": "O", "name": "Merdiven evi", "kind": "common"}
        ], "shareRule": "equal"}}),
    );
    let room = |x: f64, level: &str, unit: &str| json!({"seed": p(x, 3.0), "arch": {"level": level}, "unit": unit});
    command(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            room(2.5, "zemin", "d1"), room(7.5, "zemin", "ortak"), room(12.5, "zemin", "d2"),
            room(2.5, "k1", "d3"), room(7.5, "k1", "ortak"), room(12.5, "k1", "e1")
        ]}),
    );
    let _ = app.update(Message::Run("tool.scheduleLandShares"));
    assert_eq!(app.session.tool_id(), "scheduleLandShares");
    let _ = app.submit_line("20,0");
    let said = crate::files_testing::last_said(&app);
    assert!(
        said.starts_with("Kat irtifakı tablosu:") && said.contains("Arsa payı paydası 450."),
        "{said}"
    );
    assert!(
        !said.contains("sırayla gitmiyor") && !said.contains("niteliği yok"),
        "{said}"
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let cells = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Table(t) => Some(t.cells.clone()),
            _ => None,
        })
        .expect("the table");
    let rows: Vec<Vec<&str>> = cells
        .iter()
        .map(|r| r.iter().map(String::as_str).collect())
        .collect();
    assert_eq!(
        rows,
        vec![
            vec![
                "No",
                "Kat",
                "Niteliği",
                "BB net alanı (m²)",
                "BB brüt alanı (m²)",
                "Eklentiler",
                "Eklenti net alanı (m²)",
                "BB toplam brüt alanı (m²)",
                "BB genel brüt alanı (m²)",
                "Arsa payı",
            ],
            vec![
                "1",
                "Zemin kat",
                "Daire",
                "25.71",
                "30.00",
                "Depo 30.00 m²",
                "25.71",
                "60.00",
                "80.00",
                "214/450"
            ],
            vec![
                "2",
                "Zemin kat",
                "Daire",
                "25.71",
                "30.00",
                "",
                "",
                "30.00",
                "50.00",
                "107/450"
            ],
            vec![
                "3", "1. kat", "Daire", "25.71", "30.00", "", "", "30.00", "50.00", "129/450"
            ],
            vec![
                "Toplam", "", "", "77.14", "90.00", "", "25.71", "120.00", "180.00", ""
            ],
        ],
        "{cells:?}"
    );
    // Numbers other than 1, 2, 3 … and a unit without its nature are said (KMK 12/a).
    command(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "D1", "nature": "Daire"}, {"id": "d2", "number": "D2"},
            {"id": "d3", "number": "D3", "nature": "Daire", "landFactor": "1.2"},
            {"id": "e1", "number": "E1", "kind": "annex", "nature": "Depo", "annexOf": "d1"},
            {"id": "ortak", "number": "O", "name": "Merdiven evi", "kind": "common"}
        ], "shareRule": "equal"}}),
    );
    let notes = kentos_interaction::tag_tools::land_share_notes(
        &app.document.as_ref().expect("a drawing").model,
    );
    assert!(
        notes.contains("Bölüm numaraları (D1, D2, D3) 1'den başlayıp sırayla gitmiyor"),
        "{notes}"
    );
    assert!(notes.contains("“D2” bölümünün niteliği yok"), "{notes}");
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        // The real window's drawing (fixtures/interaction/real/kat-irtifaki.txt): the program back as the table had it,
        // the table taken away.
        let _ = app.update(Message::Run("edit.undo"));
        let _ = app.update(Message::Run("edit.undo"));
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("irtifak.kcad"), bytes).expect("written");
    }
}
