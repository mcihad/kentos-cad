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
