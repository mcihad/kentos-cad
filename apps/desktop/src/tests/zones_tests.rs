//! Mahal, zon ve emsal (docs/adr/8069; plan/bim-plan.md BIM-08) on the building paiy_area_tests checks by hand: 15 × 6 m,
//! exterior walls 25 cm, two 15 cm walls at x = 5 and 10; the west and east rooms two independent units (net 4.675 ×
//! 5.5 = 25.7125 m² each), the middle one the common stair hall. A balcony 5 × 1.5 m (7.5 m²) drawn on unit 1's south
//! façade, one 4 × 1 m (4 m²) on unit 2's.
//!
//! PAİY 22/1-l (as amended RG 11/7/2021-31538): open balconies are out up to 20 % of their unit's net area. Unit 1:
//! 0.20 × 25.7125 = 5.1425, so 7.5 − 5.1425 = 2.3575 m² of its balcony counts in; unit 2's 4 m² is within its 5.1425.

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

/// The two-unit building with its balconies and the parcel's zoning (400 m², emsal 1.50, TAKS 0.40).
fn building(balcony_one: f64) -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    let (layer, level) = {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let level = doc.settings().arch.as_ref().and_then(|a| a.levels.first()).map(|l| l.id.clone()).expect("a storey");
        (doc.layers().active().to_owned(), level)
    };
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}});
    command(&mut app, "cad.walls.create", json!({"layerId": layer, "walls": [
        wall((0.125, 0.125), (14.875, 0.125), 0.25), wall((14.875, 0.125), (14.875, 5.875), 0.25),
        wall((14.875, 5.875), (0.125, 5.875), 0.25), wall((0.125, 5.875), (0.125, 0.125), 0.25),
        wall((5.0, 0.125), (5.0, 5.875), 0.15), wall((10.0, 0.125), (10.0, 5.875), 0.15)
    ]}));
    command(&mut app, "cad.program.edit", json!({"program": {
        "units": [
            {"id": "d1", "number": "1", "kind": "unit"},
            {"id": "ortak", "number": "O", "kind": "common"},
            {"id": "d2", "number": "2", "kind": "unit"}
        ],
        "briefs": [{"usage": "Balkon", "outdoor": true}],
        "shareRule": "equal"
    }}));
    let room = |x: f64, unit: &str| json!({"seed": p(x, 3.0), "arch": {"level": level}, "unit": unit});
    command(&mut app, "cad.spaces.create", json!({"layerId": layer, "spaces": [room(2.5, "d1"), room(7.5, "ortak"), room(12.5, "d2")]}));
    let balcony = |x0: f64, x1: f64, depth: f64, unit: &str| json!({
        "parts": [{"outer": {"pts": [p(x0, -depth), p(x1, -depth), p(x1, 0.0), p(x0, 0.0)]}}],
        "arch": {"level": level}, "unit": unit, "name": "Balkon", "usage": "Balkon"
    });
    command(&mut app, "cad.spaces.create", json!({"layerId": layer, "spaces": [
        balcony(0.0, 5.0, balcony_one, "d1"), balcony(10.5, 14.5, 1.0, "d2")
    ]}));
    let arch = app.document.as_ref().expect("a drawing").model.settings().arch.clone().expect("arch");
    let mut a = serde_json::to_value(&arch).expect("the settings");
    a["zoning"] = json!({"parcelArea": 400.0, "far": 1.5, "coverage": 0.4});
    command(&mut app, "cad.settings.set", json!({"arch": a}));
    app
}

fn report(app: &App) -> kentos_domain::zoning::ZoningReport {
    app.document.as_ref().expect("a drawing").model.zoning_report().expect("a zoning")
}

#[test]
fn a_unit_s_balconies_past_20_percent_of_its_net_area_count_in() {
    let app = building(1.5);
    let r = report(&app);
    // Unit 1's balcony 7.5 m² against 0.20 × 25.7125: 2.3575 m² in; unit 2's 4 m² within its share.
    assert!((r.balcony_over - 2.3575).abs() < 1e-6, "{r:?}");
    // Katlar alanı: the two units' and the hall's gross (3 × 30) and the balcony's part past the 20 %.
    assert!((r.included - (90.0 + 2.3575)).abs() < 1e-6, "{r:?}");
    assert!((r.exempt - (7.5 - 2.3575 + 4.0)).abs() < 1e-6, "{r:?}");
    // Emsal ve TAKS tablosu says the part past the 20 % on its own row.
    let t = app.document.as_ref().expect("a drawing").model.zoning_table();
    let row = t.rows.iter().find(|r| r[0].starts_with("Balkonların %20'yi aşan kısmı")).expect("the balconies' row");
    assert_eq!(row[2], "2.36", "{row:?}");
    // A balcony within its share stays out, and the table has no such row.
    let small = building(1.0);
    assert!(!small.document.as_ref().expect("a drawing").model.zoning_table().rows.iter().any(|r| r[0].starts_with("Balkonların")));
    let r = report(&small);
    assert!(r.balcony_over.abs() < 1e-9, "5 m² is within 5.1425: {r:?}");
    assert!((r.included - 90.0).abs() < 1e-6, "{r:?}");
}
