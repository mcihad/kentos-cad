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
