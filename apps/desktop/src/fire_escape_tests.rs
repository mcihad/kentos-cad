//! Yangın kaçışı (M35, docs/adr/3035): Binaların Yangından Korunması Hakkında Yönetmelik read on the drawing's escape
//! routes — an office storey's distances, people, exits and corridor; a tall dwelling building's walks inside a flat
//! and from its door, and its stairs; the sprinkler system's longer distances.

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

fn blank() -> App {
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
    app
}

fn layer(app: &App) -> String {
    app.document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned()
}

fn p(x: f64, y: f64) -> Value {
    json!({"x": x, "y": y})
}

/// The fire regulation's findings: code and message.
fn fire(app: &mut App) -> Vec<(String, String)> {
    let answer = run(app, "cad.arch.audit", json!({ "checks": ["regulation"] }));
    answer["output"]["findings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|f| f["code"].as_str().is_some_and(|c| c.starts_with("bykhy_")))
        .map(|f| {
            (
                f["code"].as_str().unwrap_or_default().to_owned(),
                f["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn set_arch(app: &mut App, edit: impl FnOnce(&mut kentos_commands::ArchSettings)) {
    let mut arch = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .arch
        .clone()
        .unwrap_or_default();
    edit(&mut arch);
    run(
        app,
        "cad.settings.set",
        json!({ "arch": serde_json::to_value(&arch).expect("serialises") }),
    );
}

/// Walls on a level: (a, b, width).
fn walls(app: &mut App, level: &str, list: &[((f64, f64), (f64, f64), f64)]) -> Vec<String> {
    let layer = layer(app);
    let walls: Vec<Value> = list
        .iter()
        .map(|(a, b, w)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}}))
        .collect();
    let answer = run(
        app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": walls}),
    );
    answer["output"]["uids"]
        .as_array()
        .expect("uids")
        .iter()
        .map(|u| u.as_str().expect("a uid").to_owned())
        .collect()
}

/// Seeded rooms on a level: (seed, name, unit).
fn rooms(app: &mut App, level: &str, list: &[((f64, f64), &str, &str)]) {
    let layer = layer(app);
    let spaces: Vec<Value> = list
        .iter()
        .map(|(s, name, unit)| json!({"seed": p(s.0, s.1), "name": name, "unit": unit, "arch": {"level": level}}))
        .collect();
    run(
        app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": spaces}),
    );
}

/// Doors: (host wall's uid, distance along it, rough width).
fn doors(app: &mut App, list: &[(&str, f64, f64)]) {
    let layer = layer(app);
    let openings: Vec<Value> = list
        .iter()
        .map(|(host, at, w)| json!({"host": host, "at": at, "form": "door", "width": w, "head": 2.1}))
        .collect();
    run(
        app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": openings}),
    );
}

/// An office storey (the first floor of two): a 1.00 m corridor with its escape stair at the west end only, four
/// offices off it, a meeting room of 63 people with one door, an office with no door. The farthest offices are more
/// than 15 m from the stair's door (one way: Ek-5/B's office), the corridor is narrower than 1.10 m (33/1), the stair
/// 0.90 m (1.00 m for 50 people or more, 33/1), the storey of 76 people has one exit (two asked, 39/2), the meeting
/// room one door (two asked), the doorless office reaches no exit (33/6-b).
#[test]
fn an_office_storey_s_distances_people_exits_and_corridor_are_said() {
    let mut app = blank();
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
            {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0}
        ] } }),
    );
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "b1", "number": "1", "nature": "Büro"},
            {"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common"}
        ]}}),
    );
    let w = walls(
        &mut app,
        "k1",
        &[
            ((0.0, 0.0), (30.0, 0.0), 0.2),
            ((30.0, 0.0), (30.0, 9.0), 0.2),
            ((30.0, 9.0), (0.0, 9.0), 0.2),
            ((0.0, 9.0), (0.0, 0.0), 0.2),
            ((0.0, 3.0), (30.0, 3.0), 0.1),
            ((0.0, 4.1), (30.0, 4.1), 0.1),
            ((6.0, 0.0), (6.0, 3.0), 0.1),
            ((12.0, 0.0), (12.0, 3.0), 0.1),
            ((18.0, 0.0), (18.0, 3.0), 0.1),
            ((24.0, 0.0), (24.0, 3.0), 0.1),
            ((20.0, 4.1), (20.0, 9.0), 0.1),
        ],
    );
    rooms(
        &mut app,
        "k1",
        &[
            ((3.0, 1.5), "Merdiven evi", "ortak"),
            ((15.0, 3.5), "Koridor", "ortak"),
            ((9.0, 1.5), "Ofis 1", "b1"),
            ((15.0, 1.5), "Ofis 2", "b1"),
            ((21.0, 1.5), "Ofis 3", "b1"),
            ((27.0, 1.5), "Ofis 4", "b1"),
            ((10.0, 6.5), "Toplantı salonu", "b1"),
            ((25.0, 6.5), "Ofis 5", "b1"),
        ],
    );
    // The south partition (y 3.0): the stair room's door at x 3, the offices' at 9, 15, 21, 27; the meeting room's on
    // the north partition at x 10.
    doors(
        &mut app,
        &[
            (&w[4], 3.0, 1.0),
            (&w[4], 9.0, 0.9),
            (&w[4], 15.0, 0.9),
            (&w[4], 21.0, 0.9),
            (&w[4], 27.0, 0.9),
            (&w[5], 10.0, 1.0),
        ],
    );
    // The stair from Zemin kat arrives in the first floor's stair room.
    let layer = layer(&app);
    run(
        &mut app,
        "cad.stairs.create",
        json!({"layerId": layer, "stairs": [{"pts": [p(0.6, 0.6), p(0.6, 2.4), p(5.4, 2.4)], "width": 0.9,
            "risers": 18, "riserHeight": 0.1667, "tread": 0.27, "railings": "both", "arch": {"level": "zemin"}}]}),
    );
    let found = fire(&mut app);
    let codes: Vec<&str> = found.iter().map(|(c, _)| c.as_str()).collect();
    let messages: Vec<&str> = found.iter().map(|(_, m)| m.as_str()).collect();
    let far = |room: &str| {
        messages.iter().any(|m| {
            m.starts_with(&format!("{room} en uzak noktasından kat çıkışına "))
                && m.ends_with("> 15.00 m (büro, tek yön) — BYKHY 32/4, Ek-5/B (9.7.2015)")
        })
    };
    assert!(
        far("Ofis 4") && far("Ofis 3") && far("Ofis 2") && far("Toplantı salonu"),
        "{messages:#?}"
    );
    assert!(!far("Ofis 1"), "{messages:#?}");
    assert!(
        messages.contains(&"Ofis 4 en uzak noktasından kat çıkışına 28.07 m > 15.00 m (büro, tek yön) — BYKHY 32/4, Ek-5/B (9.7.2015)"),
        "{messages:#?}"
    );
    // The corridor's own walk is a dead end's (4/i): its east end 26.80 m from the stair's door.
    assert!(
        messages.contains(&"Koridor çıkmaz koridor uzunluğu 26.80 m > 15.00 m (büro) — BYKHY 4/i, Ek-5/B (9.7.2015)"),
        "{messages:#?}"
    );
    assert!(
        messages.contains(&"Ofis 5 kat çıkışına ulaşmıyor — BYKHY 33/6-b"),
        "{messages:#?}"
    );
    assert!(
        messages.contains(
            &"Toplantı salonu kişi yükü 63: en az 2 çıkış gerekir, 1 var — BYKHY 39/2 (9.9.2009)"
        ),
        "{messages:#?}"
    );
    assert!(
        messages.iter().any(|m| m.starts_with("“1. kat” kişi yükü ")
            && m.ends_with(": en az 2 çıkış gerekir, 1 var — BYKHY 39/2 (9.9.2009)")),
        "{messages:#?}"
    );
    assert!(
        messages.iter().any(|m| m
            .contains(": her kaçış yolu en az 1.00 m; merdiven 0.90 m — BYKHY 33/1 (9.7.2015)")),
        "{messages:#?}"
    );
    assert!(
        messages.contains(&"Koridor net eni 1.00 m < 1.10 m (kaçış yolu) — BYKHY 33/1 (9.7.2015)"),
        "{messages:#?}"
    );
    assert!(
        !codes.contains(&"bykhy_stair_count_below"),
        "an office building: 48/5 is a dwelling's"
    );
    // The real window's drawing (fixtures/interaction/real/yangin-kacisi.txt): the office storey.
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("buro-yangin.kcad"), bytes)
            .expect("written");
    }
    // With a sprinkler system an office's one-way walk may be 30 m, a dead end 20 m: only the corridor stays.
    set_arch(&mut app, |a| a.sprinklers = true);
    let found = fire(&mut app);
    let far: Vec<&str> = found
        .iter()
        .filter(|(c, _)| c == "bykhy_escape_distance_over")
        .map(|(_, m)| m.as_str())
        .collect();
    assert_eq!(
        far,
        [
            "Koridor çıkmaz koridor uzunluğu 26.80 m > 20.00 m (büro, yağmurlamalı) — BYKHY 4/i, Ek-5/B (9.7.2015)"
        ],
        "{found:#?}"
    );
}

/// A dwelling building of eight 3 m storeys (structure 24 m): its ground storey's flat “1” is a row of three rooms, its
/// farthest point some 22 m from the flat's door (20 m asked, 48/2); the door is some 26 m from the stair's room along
/// a common corridor with one exit (15 m one way, 48/4 and Ek-5/B); one stair where two are asked (48/5-b); a high
/// building's corridor narrower than 1.20 m (33/2). With a sprinkler system the walks may be 30 m.
#[test]
fn a_tall_dwelling_building_s_walks_and_stairs_are_said() {
    let mut app = blank();
    let levels: Vec<Value> = (0..8)
        .map(|i| {
            let id = if i == 0 {
                "zemin".to_owned()
            } else {
                format!("k{i}")
            };
            let name = if i == 0 {
                "Zemin kat".to_owned()
            } else {
                format!("{i}. kat")
            };
            json!({"id": id, "name": name, "elevation": 3.0 * i as f64, "height": 3.0})
        })
        .collect();
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": levels } }),
    );
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "nature": "Daire"},
            {"id": "d2", "number": "2", "nature": "Daire"},
            {"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common"}
        ]}}),
    );
    let w = walls(
        &mut app,
        "zemin",
        &[
            ((0.0, 0.0), (32.0, 0.0), 0.2),
            ((32.0, 0.0), (32.0, 5.3), 0.2),
            ((32.0, 5.3), (0.0, 5.3), 0.2),
            ((0.0, 5.3), (0.0, 0.0), 0.2),
            ((0.0, 4.0), (28.0, 4.0), 0.1),
            ((28.0, 0.0), (28.0, 5.3), 0.1),
            ((8.0, 0.0), (8.0, 4.0), 0.1),
            ((16.0, 0.0), (16.0, 4.0), 0.1),
            ((24.0, 0.0), (24.0, 4.0), 0.1),
        ],
    );
    rooms(
        &mut app,
        "zemin",
        &[
            ((4.0, 2.0), "Salon", "d1"),
            ((12.0, 2.0), "Yatak odası 1", "d1"),
            ((20.0, 2.0), "Yatak odası 2", "d1"),
            ((26.0, 2.0), "Salon", "d2"),
            ((14.0, 4.6), "Koridor", "ortak"),
            ((30.0, 2.5), "Merdiven evi", "ortak"),
        ],
    );
    doors(
        &mut app,
        &[
            (&w[4], 2.0, 1.0),
            (&w[4], 26.0, 1.0),
            (&w[6], 2.0, 0.9),
            (&w[7], 2.0, 0.9),
            (&w[5], 4.6, 1.0),
        ],
    );
    let layer = layer(&app);
    run(
        &mut app,
        "cad.stairs.create",
        json!({"layerId": layer, "stairs": [{"pts": [p(28.7, 0.6), p(28.7, 4.7), p(31.4, 4.7)], "width": 1.2,
            "risers": 18, "riserHeight": 0.1667, "tread": 0.27, "railings": "both", "arch": {"level": "zemin"}}]}),
    );
    let found = fire(&mut app);
    let messages: Vec<&str> = found.iter().map(|(_, m)| m.as_str()).collect();
    assert!(
        messages.iter().any(
            |m| m.starts_with("“1” bölümünde en uzak noktadan daire kapısına ")
                && m.ends_with("> 20.00 m — BYKHY 48/2 (9.9.2009)")
        ),
        "{messages:#?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("“1” bölümünün kapısından kat çıkışına ")
                && m.ends_with("> 15.00 m (apartman, tek yön) — BYKHY 48/4, Ek-5/B (9.7.2015)")),
        "{messages:#?}"
    );
    assert!(
        !messages
            .iter()
            .any(|m| m.starts_with("“2” bölümünün kapısından")),
        "flat 2's door is by the stair: {messages:#?}"
    );
    assert!(
        messages.contains(&"Yapı yüksekliği 24.00 m: en az 2 merdiven, biri korunumlu ve her daireden ikisine de ulaşılan gerekir; zemin katta 1 ortak merdiven var — BYKHY 48/5-b (9.9.2009)"),
        "{messages:#?}"
    );
    assert!(
        messages.contains(&"Koridor net eni 1.15 m < 1.20 m (yüksek bina) — BYKHY 33/2"),
        "{messages:#?}"
    );
    // The walks' lengths as measured: 48/2 about 22 m, 48/4 about 26 m.
    let number = |prefix: &str| -> f64 {
        let m = messages
            .iter()
            .find(|m| m.starts_with(prefix))
            .expect("the finding");
        m[prefix.len()..]
            .split(' ')
            .next()
            .unwrap_or_default()
            .parse()
            .unwrap_or(0.0)
    };
    assert_eq!(
        number("“1” bölümünde en uzak noktadan daire kapısına "),
        22.0
    );
    assert_eq!(number("“1” bölümünün kapısından kat çıkışına "), 26.15);
    // With a sprinkler system: 30 m inside the flat and one way; the stairs and the corridor stay.
    set_arch(&mut app, |a| a.sprinklers = true);
    let found = fire(&mut app);
    let codes: Vec<&str> = found.iter().map(|(c, _)| c.as_str()).collect();
    assert_eq!(
        codes,
        ["bykhy_corridor_narrow", "bykhy_stair_count_below"],
        "{found:#?}"
    );
    // The real window's drawing (fixtures/interaction/real/yangin-kacisi.txt): the sprinkler system off again.
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        set_arch(&mut app, |a| a.sprinklers = false);
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("konut-yangin.kcad"), bytes)
            .expect("written");
    }
}
