//! Mevzuat denetimi (M33, docs/adr/3033): Planlı Alanlar İmar Yönetmeliği's rooms (29/1), corridors (29/3), storey
//! heights (28/1-c) and doors (39/1) read on the drawing; each finding names its article and says what it measured.

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

/// The regulation group's findings: code, message and rule.
fn regulation(app: &mut App, checks: Value) -> Vec<(String, String, String)> {
    let answer = run(app, "cad.arch.audit", json!({ "checks": checks }));
    answer["output"]["findings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|f| f["check"] == "regulation")
        .map(|f| {
            (
                f["code"].as_str().unwrap_or_default().to_owned(),
                f["message"].as_str().unwrap_or_default().to_owned(),
                f["rule"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// The acceptance apartment: its rooms keep 29/1, its doors do not keep 39/1 as amended on 13.8.2025: the four inner
/// doors of a 90 cm rough opening leave 0.76 m clear (0.90 asked), the entrance 0.86 m (1.00 asked at a unit's door).
/// A plain call does not read the regulation: it is asked for by name.
#[cfg(unix)]
#[test]
fn the_apartment_s_doors_are_narrower_than_the_regulation_asks() {
    let (mut app, _dir) = crate::acceptance_flat_tests::opened("mevzuat");
    let _ = crate::acceptance_flat_tests::drawn(&mut app);
    let found = regulation(&mut app, json!(["regulation"]));
    let messages: Vec<&str> = found.iter().map(|(_, m, _)| m.as_str()).collect();
    assert_eq!(found.len(), 5, "{messages:?}");
    assert!(
        found.iter().all(|(c, ..)| c == "paiy_door_width_below"),
        "{found:?}"
    );
    assert_eq!(
        messages
            .iter()
            .filter(|m| **m == "Kapının net genişliği 0.76 m < 0.90 m — PAİY 39/1-c (13.8.2025)")
            .count(),
        4,
        "{messages:?}"
    );
    assert!(
        messages.contains(&"Bağımsız bölüm giriş kapısının net genişliği 0.86 m < 1.00 m — PAİY 39/1-c (13.8.2025)"),
        "{messages:?}"
    );
    assert!(
        found
            .iter()
            .all(|(.., r)| r == "PAİY 39/1-c · RG 13.8.2025/32985 değişikliğiyle"),
        "{found:?}"
    );
    // Asked by name only, as the doors and the fixtures: an older call's findings stay as they were; Denetim's panel asks
    // for every group by name.
    assert!(
        regulation(&mut app, json!([])).is_empty(),
        "a plain call reads no regulation"
    );
}

/// A small dwelling of one 4.20 m storey: its living room 11.11 m² (12 asked), its bedroom 2.35 m wide and 8.23 m²
/// (2.50 and 9 asked), its corridor 0.85 m (1.20 asked), no kitchen (its bathroom stands for the toilet in two rooms,
/// 29/2), the storey 4.20 m high (4.00 at most in a building of one storey).
#[test]
fn a_small_dwelling_s_rooms_corridor_kitchen_and_storey_are_said() {
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
        json!({ "arch": { "levels": [{ "id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 4.2 }] } }),
    );
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [{"id": "d1", "number": "1", "nature": "Daire"}]}}),
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
    let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": "zemin"}});
    run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.125, 0.125), (11.875, 0.125), 0.25), wall((11.875, 0.125), (11.875, 3.875), 0.25),
            wall((11.875, 3.875), (0.125, 3.875), 0.25), wall((0.125, 3.875), (0.125, 0.125), 0.25),
            wall((3.5, 0.125), (3.5, 3.875), 0.15), wall((6.0, 0.125), (6.0, 3.875), 0.15),
            wall((7.0, 0.125), (7.0, 3.875), 0.15)
        ]}),
    );
    let room = |x: f64, name: &str| json!({"seed": p(x, 2.0), "arch": {"level": "zemin"}, "name": name, "unit": "d1"});
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [room(1.75, "Salon"), room(4.75, "Yatak odası"), room(6.5, "Koridor"), room(9.5, "Banyo")]}),
    );
    let found = regulation(&mut app, json!(["regulation"]));
    let messages: Vec<&str> = found.iter().map(|(_, m, _)| m.as_str()).collect();
    let codes: Vec<&str> = found.iter().map(|(c, ..)| c.as_str()).collect();
    assert_eq!(
        codes,
        [
            "paiy_room_missing",
            "paiy_room_area_below",
            "paiy_room_area_below",
            "paiy_room_width_below",
            "paiy_corridor_narrow",
            "paiy_storey_height_over"
        ],
        "{messages:?}"
    );
    assert!(
        messages.contains(&"“1” bölümünde mutfak yok — PAİY 29/1"),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"Salon net alanı 11.11 m² < 12.00 m² — PAİY 29/1"),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("Yatak odası net alanı 8.2")
                && m.ends_with("< 9.00 m² — PAİY 29/1")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("Yatak odası dar kenarı 2.3")
                && m.ends_with("< 2.50 m — PAİY 29/1")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("Koridor net eni 0.8") && m.ends_with("< 1.20 m — PAİY 29/3")),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"“Zemin kat” kat yüksekliği 4.20 m > 4.00 m (konut bölgesi, 1 kat) — PAİY 28/1-c (12.8.2023)"),
        "{messages:?}"
    );
    let rules: Vec<&str> = found.iter().map(|(.., r)| r.as_str()).collect();
    assert!(
        rules.contains(&"PAİY 29/1 · RG 3.7.2017/30113"),
        "{rules:?}"
    );
    assert!(
        rules.contains(&"PAİY 28/1-c · RG 12.8.2023/32277 değişikliğiyle"),
        "{rules:?}"
    );
    // The real window's drawing (fixtures/interaction/real/mevzuat-denetimi.txt): the small dwelling.
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("konut.kcad"), bytes).expect("written");
    }
    // Not a dwelling: an office keeps none of 29/1's rooms, nor 28/1-c's storey.
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [{"id": "d1", "number": "1", "nature": "Büro"}]}}),
    );
    let found = regulation(&mut app, json!(["regulation"]));
    assert!(found.is_empty(), "{found:?}");
}
