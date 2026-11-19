//! Otel ve yurt yatak odasının kapısından kaçış uzaklığı (docs/adr/8156; BYKHY 50/4, 50/5) worked by hand through the
//! commands, checked in Denetim with the standards package:
//!
//! - a two-storey hotel (Otel): on 1. kat an escape stair's room (Merdiven evi, x −4–0) at the west end of a Koridor
//!   (x 0–20, y 0–2), Oda 101 (x 0–10) and Oda 102 (x 10–20) north of it (y 2–8), 20 cm walls, each room's door in
//!   the corridor's wall. Oda 101's door (x 8) is 7.84 + 0.40 = 8.24 m from the stair's room: within Oteller's one way
//!   15 m, though its farthest point is about 17.8 m away (the fire rules measured that before, 32/4); Oda 102's door
//!   (x 19) is 18.82 + 0.40 = 19.22 m away: 50/4 fails. A second stair at the east end gives the corridor two ways:
//!   it passes.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib lodging_escape_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::ArchAuditOutcome as O;
use serde_json::json;

use crate::app::{App, Message};
use crate::use_class_tests::{building, checked, p, rect, run, spaces};

/// A 20 cm wall on 1. kat.
fn wall(a: (f64, f64), b: (f64, f64)) -> serde_json::Value {
    json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": "k01"}})
}

/// The hotel's 1. kat; its east wall's persistent id (a second stair is opened through it).
fn hotel() -> (App, String) {
    let mut app = building(2, "Otel");
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let walls = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((-4.0, 0.0), (20.0, 0.0)), wall((20.0, 0.0), (20.0, 8.0)), wall((20.0, 8.0), (-4.0, 8.0)),
            wall((-4.0, 8.0), (-4.0, 0.0)), wall((0.0, 0.0), (0.0, 8.0)), wall((0.0, 2.0), (20.0, 2.0)),
            wall((10.0, 2.0), (10.0, 8.0))
        ]}),
    );
    let walls: Vec<String> = walls["output"]["uids"]
        .as_array()
        .expect("walls")
        .iter()
        .map(|u| u.as_str().expect("a wall").to_owned())
        .collect();
    let door = |host: &str, at: f64| json!({"host": host, "at": at, "form": "door", "width": 0.9, "head": 2.1});
    run(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [door(&walls[4], 1.0), door(&walls[5], 8.0), door(&walls[5], 19.0)]}),
    );
    spaces(
        &mut app,
        vec![
            json!({"parts": rect(-3.9, 0.1, -0.1, 7.9), "name": "Merdiven evi", "arch": {"level": "k01"}}),
            json!({"parts": rect(0.1, 0.1, 19.9, 1.9), "name": "Koridor", "arch": {"level": "k01"}}),
            json!({"parts": rect(0.1, 2.1, 9.9, 7.9), "name": "Oda 101", "unit": "b1", "arch": {"level": "k01"}}),
            json!({"parts": rect(10.1, 2.1, 19.9, 7.9), "name": "Oda 102", "unit": "b1", "arch": {"level": "k01"}}),
        ],
    );
    (app, walls[1].clone())
}

/// The east stair: its room beyond the east wall, a door through it.
fn east_stair(app: &mut App, east: &str) {
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    run(
        app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((20.0, 0.0), (24.0, 0.0)), wall((24.0, 0.0), (24.0, 8.0)), wall((24.0, 8.0), (20.0, 8.0))
        ]}),
    );
    run(
        app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [{"host": east, "at": 1.0, "form": "door", "width": 0.9, "head": 2.1}]}),
    );
    spaces(app, vec![json!({"parts": rect(20.1, 0.1, 23.9, 7.9), "name": "Merdiven evi 2", "arch": {"level": "k01"}})]);
}

fn row(app: &App) -> O {
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    set.rules.iter().find(|r| r.rule == "BYKHY 50/4").unwrap_or_else(|| panic!("50/4: {:#?}", set.rules)).state
}

/// A room's walk from its door as the facts carry it.
fn door_exit(app: &App, name: &str) -> kentos_extensions::audit_facts::building::DoorExitFacts {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    facts
        .spaces
        .iter()
        .find(|s| s.name == name)
        .and_then(|s| s.door_exit.clone())
        .unwrap_or_else(|| panic!("{name}'s walk from its door"))
}

fn findings(app: &App) -> Vec<(String, String)> {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    audited.findings.iter().map(|f| (f.code.clone(), f.message.clone())).collect()
}

#[test]
fn a_hotel_bedroom_is_measured_from_its_door() {
    let (mut app, east) = hotel();
    // The facts: from each door's outer side to the stair's room, one exit; the door opens into the corridor.
    let a = door_exit(&app, "Oda 101");
    let b = door_exit(&app, "Oda 102");
    assert!((a.length - 8.241).abs() < 0.01, "{a:?}");
    assert!((b.length - 19.217).abs() < 0.01, "{b:?}");
    assert_eq!((a.exits, b.exits), (1, 1));
    assert!((a.one_way - a.length).abs() < 1e-9, "one exit: all of it is one way: {a:?}");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let corridor = kentos_processing::audit_facts::building::measure(doc)
        .spaces
        .into_iter()
        .find(|s| s.name == "Koridor")
        .map(|s| s.uid)
        .expect("the corridor");
    assert_eq!(a.into.as_deref(), Some(corridor.as_str()));
    checked(&mut app);
    // The fire rules leave the bedrooms to the package: no 32/4 finding from their farthest points (Oda 101's ~17.8 m).
    let said = findings(&app);
    assert!(
        !said.iter().any(|(code, message)| code == "bykhy_escape_distance_over" && message.contains("Oda 10")),
        "{said:#?}"
    );
    // 50/4: Oda 102's door 19.22 m > 15 m one way; Oda 101's 8.24 m keeps it.
    assert_eq!(row(&app), O::Failed);
    let over: Vec<&String> =
        said.iter().filter(|(code, _)| code == "bykhy_bedroom_escape_distance").map(|(_, m)| m).collect();
    assert_eq!(over.len(), 1, "{said:#?}");
    let message = over[0];
    assert!(
        message.contains("Oda 102") && message.contains("> 15,00 m (otel, tek yön)") && message.contains("BYKHY 50/4"),
        "{message}"
    );
    // The plan's walks: each bedroom's from its door, against Oteller's one way.
    let doc = &app.document.as_ref().expect("a drawing").model;
    let walks = kentos_native_application::fire_audit::walks(doc);
    let from = |x: f64| {
        walks
            .iter()
            .find(|w| w.path.first().is_some_and(|s| (s.x - x).abs() < 1e-6 && (s.y - 1.8).abs() < 1e-6))
            .unwrap_or_else(|| panic!("the walk from the door at x {x}: {walks:#?}"))
    };
    let (w1, w2) = (from(8.0), from(19.0));
    assert_eq!((w1.limit, w1.words, w1.over), (Some(15.0), "tek yön", false));
    assert_eq!((w2.limit, w2.words, w2.over), (Some(15.0), "tek yön", true));
    // A second stair at the east end: two ways, the nearest 1.53 m; 50/4 passes.
    east_stair(&mut app, &east);
    let b = door_exit(&app, "Oda 102");
    assert_eq!(b.exits, 2, "{b:?}");
    assert!(b.length < 2.0 && b.one_way < 1e-9, "{b:?}");
    checked(&mut app);
    assert_eq!(row(&app), O::Passed);
}

/// Denetim's 50/4 finding beside 1. kat's plan with the escape routes drawn from the doors, light and dark;
/// `.run/gui-evidence/yatak-odasi-kacis-8156/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yatak-odasi-kacis-8156");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sizes = [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")];
    for (theme, size, name) in sizes {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, _) = hotel();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        let _ = app.update(Message::Run("tool.escapeRoutes"));
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_bedroom_escape_distance");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-yatak-odasi-kacis-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
