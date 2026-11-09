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
    // Not a dwelling: an office keeps none of 29/1's rooms, nor 28/1-c's storey; its 0.85 m corridor is the fire
    // regulation's escape route, 1.10 m asked (BYKHY 33/1, M35).
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [{"id": "d1", "number": "1", "nature": "Büro"}]}}),
    );
    let found = regulation(&mut app, json!(["regulation"]));
    let messages: Vec<&str> = found.iter().map(|(_, m, _)| m.as_str()).collect();
    assert_eq!(messages.len(), 1, "{found:?}");
    assert!(
        messages[0].starts_with("Koridor net eni 0.8")
            && messages[0].ends_with("< 1.10 m (kaçış yolu) — BYKHY 33/1 (9.7.2015)"),
        "{found:?}"
    );
}

/// A blank Mimari drawing.
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

/// The project's Mimari settings changed by `edit` and written back whole (the program, the levels and the rest kept).
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

/// The regulation group's findings whole.
fn regulation_findings(app: &mut App) -> Vec<Value> {
    let answer = run(app, "cad.arch.audit", json!({ "checks": ["regulation"] }));
    answer["output"]["findings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// A four-storey building of two flats (M34, docs/adr/3034): its common stair (1.25 m, 0.1667 m risers, a railing on
/// its left only), a ramp of 0.40 m over 4 m without railings, an air shaft of 0.40 × 0.90 m, a light well of 1.40 ×
/// 3.00 m, a window of 0.40 m sill on the first floor, a toilet's door of 0.86 m clear and no elevator, read by PAİY 30,
/// 31, 32, 34, 38 and 39. Then Sivas's regulation over them: its 1.30 m stair binding with PAİY's 1.20 m beside it, the
/// air shaft and the toilet's door a conflict, the elevator asked from three storeys. Then an elevator too small, and the
/// stair's fix.
#[test]
fn a_four_storey_building_s_stairs_ramp_shafts_and_elevator_are_said() {
    let mut app = blank();
    let level = |id: &str, name: &str, z: f64| json!({ "id": id, "name": name, "elevation": z, "height": 3.0 });
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            level("zemin", "Zemin kat", 0.0), level("k1", "1. kat", 3.0), level("k2", "2. kat", 6.0),
            level("k3", "3. kat", 9.0)
        ] } }),
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
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| json!({"outer": {"pts": [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]}});
    let drawn = |r: Value, name: &str, void: bool| json!({"parts": [r], "method": "net", "name": name, "void": void, "unit": "ortak", "arch": {"level": "zemin"}});
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            drawn(rect(0.0, 0.0, 3.0, 6.0), "Merdiven evi", false),
            drawn(rect(3.2, 0.0, 3.6, 0.9), "Hava bacası", true),
            drawn(rect(4.0, 0.0, 5.4, 3.0), "Işıklık", true)
        ]}),
    );
    run(
        &mut app,
        "cad.stairs.create",
        json!({"layerId": layer, "stairs": [{"pts": [p(0.8, 0.5), p(0.8, 4.5), p(2.8, 4.5)], "width": 1.25,
            "risers": 18, "riserHeight": 0.1667, "tread": 0.27, "railings": "left", "arch": {"level": "zemin"}}]}),
    );
    run(
        &mut app,
        "cad.ramps.create",
        json!({"layerId": layer, "ramps": [{"pts": [p(5.0, -3.0), p(9.0, -3.0)], "width": 1.2, "rise": 0.4,
            "arch": {"level": "zemin"}}]}),
    );
    // A toilet and a hall on Zemin kat, a door of 1.00 m rough (0.86 m clear) between them; a wall on the first floor.
    let wall = |a: (f64, f64), b: (f64, f64), w: f64, level: &str| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}});
    let walls = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((10.0, 2.0), (14.0, 2.0), 0.2, "zemin"), wall((14.0, 2.0), (14.0, 5.0), 0.2, "zemin"),
            wall((14.0, 5.0), (10.0, 5.0), 0.2, "zemin"), wall((10.0, 5.0), (10.0, 2.0), 0.2, "zemin"),
            wall((12.0, 2.0), (12.0, 5.0), 0.1, "zemin"), wall((10.0, 10.0), (14.0, 10.0), 0.25, "k1")
        ]}),
    );
    let uid = |i: usize| {
        walls["output"]["uids"][i]
            .as_str()
            .expect("a wall")
            .to_owned()
    };
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"seed": p(11.0, 3.5), "name": "WC", "arch": {"level": "zemin"}},
            {"seed": p(13.0, 3.5), "name": "Hol", "arch": {"level": "zemin"}}
        ]}),
    );
    run(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [
            {"host": uid(4), "at": 1.5, "form": "door", "width": 1.0, "head": 2.1},
            {"host": uid(5), "at": 2.0, "form": "window", "width": 1.2, "sill": 0.4, "head": 2.2}
        ]}),
    );
    let said = |found: &[Value]| -> Vec<(String, String)> {
        found
            .iter()
            .map(|f| {
                (
                    f["code"].as_str().unwrap_or_default().to_owned(),
                    f["message"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    };
    let found = said(&regulation_findings(&mut app));
    let expected = [
        (
            "paiy_door_width_below",
            "Kapının net genişliği 0.86 m < 0.90 m — PAİY 39/1-c (13.8.2025)",
        ),
        (
            "paiy_stair_riser_high",
            "Merdivenin rıhtı 0.167 m > 0.16 m (asansörsüz bina) — PAİY 31/2-a",
        ),
        (
            "paiy_railing_missing",
            "Ortak merdivenin sağ yanında korkuluk yok — PAİY 31/1-ç",
        ),
        ("paiy_railing_missing", "Rampada korkuluk yok — PAİY 38/1"),
        (
            "paiy_ramp_steep",
            "Rampanın eğimi %10.00 > %7 (0.40 m kot farkı) — PAİY 30/12",
        ),
        (
            "paiy_window_railing",
            "Pencerenin denizliği 0.40 m < 0.90 m: 1.10 m korkuluk gerekir — PAİY 38/1",
        ),
        (
            "paiy_lightwell_small",
            "Işıklığın dar kenarı 1.40 m < 1.50 m (4 katlı bina) — PAİY 32/2-a",
        ),
        (
            "paiy_lightwell_small",
            "Işıklığın net alanı 4.20 m² < 4.50 m² (4 katlı bina) — PAİY 32/2-a",
        ),
        (
            "paiy_air_shaft_small",
            "Hava bacasının dar kenarı 0.40 m < 0.60 m — PAİY 32/3",
        ),
        (
            "paiy_elevator_missing",
            "Binada asansör yok (4 kat) — PAİY 34/1 (1.7.2026)",
        ),
    ];
    assert_eq!(
        found,
        expected
            .iter()
            .map(|(c, m)| (c.to_string(), m.to_string()))
            .collect::<Vec<_>>()
    );
    // The real window's drawing (fixtures/interaction/real/mevzuat-merdiven.txt): PAİY alone, four storeys, no elevator.
    if let Some(out) = std::env::var_os("KENTOS_PAIY_OUT") {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(std::path::Path::new(&out).join("bina.kcad"), bytes).expect("written");
    }
    // Sivas's regulation over PAİY: its stair and landing bind with PAİY's beside them; the air shaft and the toilet's
    // door keep Sivas's looser values, not PAİY's: a conflict the administration decides.
    set_arch(&mut app, |a| {
        a.local_rules = Some(kentos_commands::LocalRules::Sivas)
    });
    let findings = regulation_findings(&mut app);
    let found = said(&findings);
    for line in [
        (
            "regulation_conflict",
            "Tuvalet kapısının net genişliği 0.86 m: Sivas 39/1-c en az 0.80 m ister, uyuyor; PAİY 39/1-c (13.8.2025) en \
             az 0.90 m ister, uymuyor — çelişkide idare karar verir",
        ),
        (
            "paiy_stair_width_narrow",
            "Ortak merdivenin kolu 1.25 m < 1.30 m — Sivas 31/1-a; PAİY 31/1-a: 1.20 m",
        ),
        (
            "paiy_stair_landing_short",
            "Ortak merdivenin ara sahanlığı 1.25 m < 1.30 m — Sivas 31/1-a; PAİY 31/1-a: 1.20 m",
        ),
        (
            "regulation_conflict",
            "Hava bacasının dar kenarı 0.40 m: Sivas 32/3 en az 0.30 m ister, uyuyor; PAİY 32/3 en az 0.60 m ister, \
             uymuyor — çelişkide idare karar verir",
        ),
        (
            "paiy_elevator_missing",
            "Binada asansör yok (4 kat) — Sivas 34/1; PAİY 34/1 (1.7.2026): 4 ve daha fazla katta asansör",
        ),
        (
            "paiy_lightwell_small",
            "Işıklığın dar kenarı 1.40 m < 1.50 m (4 katlı bina, ışıklık 1 katta) — PAİY 32/2-a",
        ),
    ] {
        assert!(
            found.contains(&(line.0.to_owned(), line.1.to_owned())),
            "{line:?} in {found:#?}"
        );
    }
    assert!(
        !found.iter().any(|(c, _)| c == "paiy_air_shaft_small"),
        "the air shaft is a conflict, not a fault: {found:#?}"
    );
    let rule_of = |code: &str, start: &str| {
        findings
            .iter()
            .find(|f| {
                f["code"] == code && f["message"].as_str().is_some_and(|m| m.starts_with(start))
            })
            .map(|f| f["rule"].as_str().unwrap_or_default().to_owned())
    };
    assert_eq!(
        rule_of("regulation_conflict", "Hava bacası").as_deref(),
        Some("Sivas İmar Yön. 32/3 · RG 10.11.2022/32009 ↔ PAİY 32/3 · RG 3.7.2017/30113")
    );
    assert_eq!(
        rule_of("paiy_stair_width_narrow", "Ortak").as_deref(),
        Some("Sivas İmar Yön. 31/1-a · RG 10.11.2022/32009")
    );
    // Three storeys: Sivas asks for an elevator, PAİY for its place only.
    set_arch(&mut app, |a| a.levels.retain(|l| l.id != "k3"));
    let elevator = |app: &mut App| {
        said(&regulation_findings(app))
            .into_iter()
            .filter(|(c, _)| c == "paiy_elevator_missing")
            .map(|(_, m)| m)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        elevator(&mut app),
        ["Binada asansör yok (3 kat) — Sivas 34/1; PAİY 34/1 (1.7.2026): 3 katta asansör yeri"]
    );
    set_arch(&mut app, |a| a.local_rules = None);
    assert_eq!(
        elevator(&mut app),
        ["Binada asansör yeri yok (3 kat; 3 katta asansör yeri bırakılır) — PAİY 34/1 (1.7.2026)"]
    );
    // An elevator of 1.10 × 1.50 m: no longer missing, too small for 34/2's cabin; the riser may be 0.18 m now.
    run(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [drawn(rect(6.0, 0.0, 7.1, 1.5), "Asansör", true)]}),
    );
    set_arch(&mut app, |a| {
        a.local_rules = Some(kentos_commands::LocalRules::Sivas)
    });
    let findings = regulation_findings(&mut app);
    let found = said(&findings);
    assert!(elevator(&mut app).is_empty(), "{found:#?}");
    assert!(
        !found.iter().any(|(c, _)| c == "paiy_stair_riser_high"),
        "{found:#?}"
    );
    for line in [
        "Asansör kuyusunun net dar kenarı 1.10 m < 1.20 m (kabinin en az dar kenarı) — PAİY 34/2",
        "Asansör kuyusunun net alanı 1.65 m² < 1.80 m² (kabinin en az alanı) — PAİY 34/2",
    ] {
        assert!(
            found.contains(&("paiy_elevator_small".to_owned(), line.to_owned())),
            "{line} in {found:#?}"
        );
    }
    // The stair's fix: 1.30 m wide, as Sivas asks; its landing follows its width.
    let fix = findings
        .iter()
        .find(|f| f["code"] == "paiy_stair_width_narrow")
        .map(|f| f["fix"].clone())
        .expect("the stair's finding");
    assert_eq!(fix["label"], "Genişliği 1.30 m yap");
    run(
        &mut app,
        fix["step"]["command"].as_str().expect("its command"),
        fix["step"]["input"].clone(),
    );
    let found = said(&regulation_findings(&mut app));
    assert!(
        !found
            .iter()
            .any(|(c, _)| c == "paiy_stair_width_narrow" || c == "paiy_stair_landing_short"),
        "{found:#?}"
    );
}
