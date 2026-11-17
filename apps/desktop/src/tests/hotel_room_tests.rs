//! Otel yatak odasının içindeki kaçış uzaklığı (docs/adr/8111; BYKHY 50/3) worked by hand through the commands, checked
//! in Denetim with the standards package:
//!
//! - a two-storey hotel (Otel): on 1. kat a Koridor (x 0–2) and Oda 101 (x 2–20, 18 m deep) inside 20 cm walls, the
//!   room's door in the wall to the corridor. From its farthest point (0.40 m in front of its far corners) to the door
//!   is more than 15 m: 50/3-a fails; a second door in its far wall: it passes.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib hotel_room_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::ArchAuditOutcome as O;
use serde_json::json;

use crate::app::App;
use crate::use_class_tests::{building, checked, p, rect, run, spaces};

/// The hotel's deep room and its corridor; its far wall's persistent id.
fn hotel() -> (App, String) {
    let mut app = building(2, "Otel");
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": "k01"}});
    let walls = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (20.0, 0.0)), wall((20.0, 0.0), (20.0, 4.0)), wall((20.0, 4.0), (0.0, 4.0)),
            wall((0.0, 4.0), (0.0, 0.0)), wall((2.0, 0.0), (2.0, 4.0))
        ]}),
    );
    let walls: Vec<String> =
        walls["output"]["uids"].as_array().expect("walls").iter().map(|u| u.as_str().expect("a wall").to_owned()).collect();
    run(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [{"host": walls[4], "at": 2.0, "form": "door", "width": 0.9, "head": 2.1}]}),
    );
    spaces(
        &mut app,
        vec![
            json!({"parts": rect(0.1, 0.1, 1.9, 3.9), "name": "Koridor", "arch": {"level": "k01"}}),
            json!({"parts": rect(2.1, 0.1, 19.9, 3.9), "name": "Oda 101", "unit": "b1", "arch": {"level": "k01"}}),
        ],
    );
    (app, walls[1].clone())
}

fn row(app: &App) -> O {
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    set.rules.iter().find(|r| r.rule == "BYKHY 50/3").unwrap_or_else(|| panic!("50/3: {:#?}", set.rules)).state
}

fn walk(app: &App) -> f64 {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    facts.spaces.iter().find(|s| s.name == "Oda 101").and_then(|s| s.to_door).expect("the room's walk to its door")
}

#[test]
fn a_deep_hotel_room_with_one_door_fails_and_a_second_door_passes() {
    let (mut app, far) = hotel();
    // From 0.40 m in front of the far corners to 0.10 m in front of the door: about 17.4 m.
    let d = walk(&app);
    assert!(d > 17.0 && d < 18.0, "{d}");
    checked(&mut app);
    assert_eq!(row(&app), O::Failed);
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    assert!(
        audited
            .findings
            .iter()
            .any(|f| f.code == "bykhy_hotel_room_distance" && f.message.contains("> 15,00 m") && f.message.contains("1 kapı")),
        "{:?}",
        audited.findings.iter().map(|f| (&f.code, &f.message)).collect::<Vec<_>>()
    );
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    run(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [{"host": far, "at": 2.0, "form": "door", "width": 0.9, "head": 2.1}]}),
    );
    assert!(walk(&app) < 15.0, "{}", walk(&app));
    checked(&mut app);
    assert_eq!(row(&app), O::Passed);
}

/// Denetim's picture (`.run/gui-evidence/otel-odasi-8111/`): the deep room's finding.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    use crate::app::Message;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/otel-odasi-8111");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let (mut app, _) = hotel();
        checked(&mut app);
        let group =
            crate::arch::Event::ArchAudit(crate::arch_audit::Event::Group(Some(kentos_commands::ArchAuditCheck::Regulation)));
        let _ = app.update(Message::Arch(group));
        let shown = app.arch_audit_shown();
        let findings = &app.arch.arch_audit.audited.as_ref().expect("checked").findings;
        let row = shown.iter().position(|&i| findings[i].code == "bykhy_hotel_room_distance").expect("the finding shown");
        let _ = app.update(Message::Arch(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Press(row))));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        app.toasts.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-derin-oda-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
