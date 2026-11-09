//! Duvar tepeleri (docs/adr/3040 §1; M40, ADV-18's first acceptance): the two-storey building, its first storey's floor
//! slab drawn, 15 cm: the ground storey's walls stop under it (2.85 of 3.00) in Metraj; the ground storey made 3.24 in
//! Proje ayarları › Mimari, the storey above moves up with its slab and the walls follow (3.09). The storey above has no
//! slab over it: its walls stay floor to floor.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::project::{SettingsEvent, settings_message};

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

/// Metraj's walls: their level, height and net face area, metres.
fn walls(app: &mut App) -> Vec<(String, f64, f64, f64)> {
    let answer = run(app, "cad.spaces.quantities", json!({}));
    answer["output"]["walls"]
        .as_array()
        .expect("walls")
        .iter()
        .map(|w| {
            (
                w["level"].as_str().unwrap_or_default().to_owned(),
                w["length"].as_f64().unwrap_or_default(),
                w["height"].as_f64().unwrap_or(f64::NAN),
                w["netArea"].as_f64().unwrap_or(f64::NAN),
            )
        })
        .collect()
}

/// Each storey's walls' heights, each once.
fn heights(rows: &[(String, f64, f64, f64)], level: &str) -> Vec<f64> {
    let mut out: Vec<f64> = rows
        .iter()
        .filter(|r| r.0 == level)
        .map(|r| (r.2 * 1000.0).round() / 1000.0)
        .collect();
    out.dedup();
    out
}

#[test]
fn the_walls_stop_under_the_slab_over_them_and_follow_their_storey_s_height() {
    let mut app = crate::permit_set_tests::building();
    let before = walls(&mut app);
    assert_eq!(
        heights(&before, "zemin"),
        [3.0],
        "floor to floor without a slab"
    );
    // The first storey's floor slab over the whole building.
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    run(
        &mut app,
        "cad.slabs.create",
        json!({"layerId": layer, "slabs": [{
            "parts": [{"outer": {"pts": [p(0.0, 0.0), p(15.0, 0.0), p(15.0, 6.0), p(0.0, 6.0)]}}],
            "thickness": 0.15, "arch": {"level": "k1"}
        }]}),
    );
    let under = walls(&mut app);
    assert_eq!(heights(&under, "zemin"), [2.85], "under the slab over them");
    assert_eq!(
        heights(&under, "k1"),
        [3.0],
        "no slab over the storey above"
    );
    // Metraj: the wall as built, the slab's 15 cm off each ground storey wall's face.
    for (b, u) in before.iter().zip(&under).filter(|(b, _)| b.0 == "zemin") {
        assert!(
            (b.3 - u.3 - b.1 * 0.15).abs() < 1e-6,
            "{b:?} → {u:?}: the slab's thickness × its length"
        );
    }
    // Proje ayarları › Mimari: the ground storey 3.00 → 3.24; the storey above and its slab move up.
    let _ = app.update(Message::Run("file.settings"));
    let _ = app.update(settings_message(SettingsEvent::Section(
        crate::project::settings::Section::Arch,
    )));
    let _ = app.update(settings_message(SettingsEvent::Level(
        0,
        crate::project::arch::Edit::Height("3.24".into()),
    )));
    let _ = app.update(settings_message(SettingsEvent::Save));
    assert!(app.project.is_none(), "saved and closed");
    let levels = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .arch
        .clone()
        .expect("levels")
        .levels;
    let k1 = levels.iter().find(|l| l.id == "k1").expect("1. kat");
    assert!((k1.elevation - 3.24).abs() < 1e-9, "{k1:?}");
    let after = walls(&mut app);
    assert_eq!(heights(&after, "zemin"), [3.09], "the walls follow");
    assert_eq!(heights(&after, "k1"), [3.0]);
    // Döşeme katmanları (docs/adr/3040 §2): the slab over them a storey's floor (kaplama 2 + şap 5 over its concrete,
    // sıva 2 under): the walls stop under its concrete, 3.24 − 0.07 − 0.15 = 3.02; the ground storey's floor slab
    // drawn too, the same build-up: they stand on its concrete, 7 cm lower: 3.09 again, concrete to concrete.
    let layers = kentos_commands::slab_build_ups()
        .into_iter()
        .find(|(n, _)| *n == "Ara kat döşemesi")
        .expect("Ara kat döşemesi")
        .1;
    let slab = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .find(|e| matches!(e, kentos_commands::Entity::Slab(_)))
        .map(|e| e.base().id)
        .expect("the slab");
    let uid = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .uid(kentos_domain::Slot(slab))
        .expect("its uid")
        .to_string();
    run(
        &mut app,
        "cad.slabs.edit",
        json!({"uids": [uid], "operation": {"kind": "set", "layers": layers}}),
    );
    assert_eq!(
        heights(&walls(&mut app), "zemin"),
        [3.02],
        "under its concrete"
    );
    run(
        &mut app,
        "cad.slabs.create",
        json!({"layerId": layer, "slabs": [{
            "parts": [{"outer": {"pts": [p(0.0, 0.0), p(15.0, 0.0), p(15.0, 6.0), p(0.0, 6.0)]}}],
            "thickness": 0.15, "arch": {"level": "zemin"}, "layers": layers
        }]}),
    );
    assert_eq!(
        heights(&walls(&mut app), "zemin"),
        [3.09],
        "concrete to concrete"
    );
}

/// A room's interior elevation (docs/adr/3040 §3): the two-storey building, the 1. kat's floor slab Ara kat (kaplama 2,
/// şap 5 over 15 cm of concrete, sıva 2 under): the ground storey's room found from its walls is drawn to its ceiling's
/// plaster, 2.76 (its walls run 2.78 to the concrete over its finished floor); the room over it, on the slab's finish,
/// has no ceiling known and is drawn to its walls' tops, the next floor: 3.00.
#[test]
fn a_room_s_interior_elevation_is_its_inside_height() {
    let mut app = crate::permit_set_tests::building();
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    run(
        &mut app,
        "cad.slabs.create",
        json!({"layerId": layer, "slabs": [{
            "parts": [{"outer": {"pts": [p(0.0, 0.0), p(15.0, 0.0), p(15.0, 6.0), p(0.0, 6.0)]}}],
            "thickness": 0.15, "arch": {"level": "k1"},
            "layers": {"over": [{"kind": "finish", "thickness": 0.02}, {"kind": "screed", "thickness": 0.05}],
                       "under": [{"kind": "plaster", "thickness": 0.02}]}
        }]}),
    );
    let made = run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"seed": p(2.5, 3.0), "name": "Salon", "arch": {"level": "zemin"}},
            {"seed": p(2.5, 3.0), "name": "Oda", "arch": {"level": "k1"}}
        ]}),
    );
    let uids: Vec<String> = made["output"]["uids"]
        .as_array()
        .expect("uids")
        .iter()
        .map(|u| u.as_str().expect("a uid").to_owned())
        .collect();
    let heights = |app: &mut App, uid: &str| -> Vec<f64> {
        let drawn = run(
            app,
            "cad.elevations.edit",
            json!({"operation": {"kind": "derive", "space": uid}}),
        );
        drawn["output"]["sides"]
            .as_array()
            .expect("sides")
            .iter()
            .map(|s| (s["height"].as_f64().expect("a height") * 1000.0).round() / 1000.0)
            .collect()
    };
    assert_eq!(
        heights(&mut app, &uids[0]),
        [2.76; 4],
        "to the ceiling's plaster"
    );
    assert_eq!(heights(&mut app, &uids[1]), [3.0; 4], "to its walls' tops");
}
