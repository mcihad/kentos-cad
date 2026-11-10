//! Otopark ve sığınak hesapları (M36, docs/adr/8036): Otopark Yönetmeliği Ek-1 and Sığınak Yönetmeliği 7–8 read from the
//! building program's units, live tables placed from Metraj ▾, and the Mevzuat group's findings.

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

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Value {
    json!({"outer": {"pts": [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]}})
}

/// `flats` flats of 100 m² (a living room and a bedroom each) and, with `shop`, a 60 m² shop on Zemin kat; a 100 m²
/// Otopark and a 25 m² Sığınak below them.
fn building(flats: usize, shop: bool) -> App {
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
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}] } }),
    );
    let mut units: Vec<Value> = (1..=flats)
        .map(|i| json!({"id": format!("d{i}"), "number": i.to_string(), "nature": "Daire"}))
        .collect();
    if shop {
        units.push(json!({"id": "s1", "number": "11", "nature": "Dükkân"}));
    }
    units.push(json!({"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common"}));
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": units}}),
    );
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let space = |r: Value, name: &str, unit: &str| json!({"parts": [r], "method": "net", "name": name, "unit": unit, "arch": {"level": "zemin"}});
    let mut spaces: Vec<Value> = Vec::new();
    for i in 0..flats {
        let x = 10.0 * i as f64;
        let unit = format!("d{}", i + 1);
        spaces.push(space(rect(x, 0.0, x + 6.0, 10.0), "Salon", &unit));
        spaces.push(space(
            rect(x + 6.0, 0.0, x + 10.0, 10.0),
            "Yatak odası",
            &unit,
        ));
    }
    if shop {
        spaces.push(space(rect(100.0, 0.0, 106.0, 10.0), "Dükkân", "s1"));
    }
    spaces.push(space(rect(0.0, -12.0, 10.0, -2.0), "Otopark", "ortak"));
    spaces.push(space(rect(12.0, -12.0, 17.0, -7.0), "Sığınak", "ortak"));
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": spaces}),
    );
    app
}

fn rows(t: &kentos_commands::TableEntity) -> Vec<Vec<&str>> {
    t.cells
        .iter()
        .map(|r| r.iter().map(String::as_str).collect())
        .collect()
}

fn table(app: &App, source: &kentos_commands::TableSource) -> kentos_commands::TableEntity {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Table(t) if t.source.as_ref() == Some(source) => {
                Some(t.clone())
            }
            _ => None,
        })
        .expect("the table")
}

/// Ek-1: ten flats of 100 m² one place for two (5.00), the shop 60 m² one for 50 (1.20): 6.20, seven places; the
/// Otopark's 100 m² gives five — red, and Mevzuat says it. Sığınak 8/a: each flat three people (two rooms), the shop
/// three (60 ÷ 20): 33 people, 33 m²; the drawn 25 m² red, and said. The tables follow the rooms: a larger Otopark
/// rewrites its count in the same step.
#[test]
fn the_parking_and_shelter_tables_count_the_units_against_the_drawing() {
    let mut app = building(10, true);
    let _ = app.update(Message::Run("arch.parkingTable"));
    assert_eq!(
        app.session.tool_id(),
        "tablePlace",
        "placed from the cursor"
    );
    let _ = app.submit_line("0,30");
    let parking = table(&app, &kentos_commands::TableSource::Parking {});
    let r = rows(&parking);
    assert_eq!(r[0], ["Otopark hesabı", "Brüt alan", "Ek-1", "Gereken"]);
    assert_eq!(
        r[1],
        ["“1” Daire", "100.00", "80–120 m²: 2 daireye 1", "0.50"]
    );
    assert_eq!(
        r[11],
        [
            "“11” Dükkân",
            "60.00",
            "50 m²'ye 1, bölüm başına en az 1",
            "1.20"
        ]
    );
    assert_eq!(r[12], ["Toplam (kesir bir üste, 8/4)", "", "", "7"]);
    assert_eq!(
        r[13],
        ["Çizimdeki otopark", "100.00", "20 m²'ye 1 (4/1-ç)", "5"]
    );
    assert!(
        parking
            .styles
            .iter()
            .any(|s| s.range.row == 13 && s.color.as_deref() == Some("#d32f2f")),
        "{:?}",
        parking.styles
    );
    let _ = app.update(Message::Run("arch.shelterTable"));
    let _ = app.submit_line("40,30");
    let shelter = table(&app, &kentos_commands::TableSource::Shelter {});
    let r = rows(&shelter);
    assert_eq!(r[0], ["Sığınak hesabı", "Esas", "Kişi", "Net alan"]);
    assert_eq!(r[1], ["“1” Daire", "2 oda", "3", ""]);
    assert_eq!(r[11], ["“11” Dükkân", "60.00 m² ÷ 20", "3", ""]);
    assert_eq!(r[12], ["Toplam (kişi başına 1 m², 8/a)", "", "33", "33.00"]);
    assert_eq!(r[13], ["Çizimdeki sığınak", "1 mahal", "", "25.00"]);
    assert_eq!(
        r[14],
        [
            "Tuvalet ve lavabo (100 kişiye)",
            "",
            "kadın 0 · erkek 0",
            ""
        ]
    );
    // Mevzuat.
    let answer = run(
        &mut app,
        "cad.arch.audit",
        json!({ "checks": ["regulation"] }),
    );
    let found: Vec<(String, String)> = answer["output"]["findings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|f| {
            f["code"]
                .as_str()
                .is_some_and(|c| c.starts_with("parking_") || c.starts_with("shelter_"))
        })
        .map(|f| {
            (
                f["code"].as_str().unwrap_or_default().to_owned(),
                f["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            (
                "parking_short".to_owned(),
                "Otopark: gereken 7 yer, çizimde 5 (100.00 m² ÷ 20 m²) — Otopark Yön. 8, Ek-1 (16.6.2022)".to_owned()
            ),
            (
                "shelter_short".to_owned(),
                "Sığınak: 33 kişi için en az 33.00 m² net, çizimde 25.00 m² — Sığınak Yön. 8/a (7.11.2025)".to_owned()
            ),
        ]
    );
    // A second Otopark of 50 m²: 150 m², seven places; the table rewritten in the same step.
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [{"parts": [rect(0.0, -17.0, 10.0, -12.0)], "method": "net",
            "name": "Otopark 2", "unit": "ortak", "arch": {"level": "zemin"}}]}),
    );
    let parking = table(&app, &kentos_commands::TableSource::Parking {});
    assert_eq!(
        rows(&parking)[13],
        ["Çizimdeki otopark", "150.00", "20 m²'ye 1 (4/1-ç)", "7"]
    );
    // The real window's drawing (fixtures/interaction/real/otopark-siginak.txt): the building before its tables.
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        let fresh = building(10, true);
        let doc = &fresh.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(
            std::path::Path::new(&out).join("otopark-siginak.kcad"),
            bytes,
        )
        .expect("written");
    }
}

/// Sığınak 7/a: nine flats only ask none; the table says why and Mevzuat is quiet.
#[test]
fn a_building_of_fewer_than_ten_flats_asks_no_shelter() {
    let app = building(9, false);
    let report = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .shelter_report();
    assert_eq!(
        report.exempt,
        Some("10'dan az bağımsız bölümlü, yalnız konut kullanımlı yapı (7/a)")
    );
    assert!(!report.short());
}
