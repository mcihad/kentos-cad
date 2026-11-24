//! Kazan dairesi (docs/adr/8113; BYKHY 54/2, 54/4, 54/5) worked by hand through the commands, checked in Denetim with the
//! standards package:
//!
//! - a two-storey dwelling building: on its ground storey a Kazan dairesi (x 0–6), a Merdiven evi (x 6–9) and a
//!   Koridor (x 9–11) inside 20 cm walls. The boiler room's door in the wall to the stair, nothing declared: 54/4 fails
//!   (it opens into a stair), 54/2 and 54/5 have missing data (the wall's and the door's classes).
//! - The door in the wall to the corridor instead, the boiler room's walls EI 120 and its door EI2 90-S200C5: 54/2 and
//!   54/4 pass; 54/5 is for the reviewer (its heat output is not drawn).
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib boiler_room_tests::screens -- --ignored --nocapture
//! ```

use crate::arch_ext::*;
use kentos_commands::ArchAuditOutcome as O;
use serde_json::json;
use crate::arch::ArchState;

use crate::app::App;
use crate::use_class_tests::{building, checked, declare, p, rect, run, spaces};

/// The ground storey's boiler room (x 0–6) and, beyond the wall at x = 6 that holds its door, the stair (`to_stair`) or
/// the corridor, the other one past it (x 9–11). The walls' persistent ids and the door's.
fn boiler(to_stair: bool) -> (App, Vec<String>, String) {
    let mut app = building(2, "Konut");
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": "k00"}});
    let walls = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (11.0, 0.0)), wall((11.0, 0.0), (11.0, 5.0)), wall((11.0, 5.0), (0.0, 5.0)),
            wall((0.0, 5.0), (0.0, 0.0)), wall((6.0, 0.0), (6.0, 5.0)), wall((9.0, 0.0), (9.0, 5.0))
        ]}),
    );
    let walls: Vec<String> =
        walls["output"]["uids"].as_array().expect("walls").iter().map(|u| u.as_str().expect("a wall").to_owned()).collect();
    let doors = run(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [{"host": walls[4], "at": 2.5, "form": "door", "width": 1.0, "head": 2.1}]}),
    );
    let door = doors["output"]["uids"][0].as_str().expect("the door").to_owned();
    let (next, last) = if to_stair { ("Merdiven evi", "Koridor") } else { ("Koridor", "Merdiven evi") };
    spaces(
        &mut app,
        vec![
            json!({"parts": rect(0.1, 0.1, 5.9, 4.9), "name": "Kazan dairesi", "arch": {"level": "k00"}}),
            json!({"parts": rect(6.1, 0.1, 8.9, 4.9), "name": next, "arch": {"level": "k00"}}),
            json!({"parts": rect(9.1, 0.1, 10.9, 4.9), "name": last, "arch": {"level": "k00"}}),
        ],
    );
    (app, walls, door)
}

fn rows(app: &App) -> [O; 3] {
    let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
    ["BYKHY 54/2", "BYKHY 54/4", "BYKHY 54/5"]
        .map(|rule| set.rules.iter().find(|r| r.rule == rule).unwrap_or_else(|| panic!("{rule}: {:#?}", set.rules)).state)
}

fn said(app: &App, code: &str) -> Vec<String> {
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    audited.findings.iter().filter(|f| f.code == code).map(|f| f.message.clone()).collect()
}

#[test]
fn a_boiler_room_opening_into_the_stair_fails_and_into_the_corridor_with_its_classes_passes() {
    let (mut app, _, _) = boiler(true);
    checked(&mut app);
    // Its door undeclared: 54/5 lacks its class too.
    assert_eq!(rows(&app), [O::MissingData, O::Failed, O::MissingData]);
    assert!(said(&app, "bykhy_boiler_room_door").iter().any(|m| m.contains("merdivene açılıyor")), "{:?}", said(&app, "bykhy_boiler_room_door"));
    let (mut app, walls, door) = boiler(false);
    // The boiler room's walls (its outer walls are no separation) and its door.
    declare(&mut app, &[&walls[4]], "EI 120");
    declare(&mut app, &[&door], "EI2 90-S200C5");
    checked(&mut app);
    assert_eq!(rows(&app), [O::Passed, O::Passed, O::ExpertReview]);
    assert!(said(&app, "bykhy_boiler_room_exits").iter().any(|m| m.contains("350 kW")));
}

/// Denetim's picture (`.run/gui-evidence/kazan-dairesi-8113/`): the boiler room opening into the stair.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    use crate::app::Message;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kazan-dairesi-8113");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let (mut app, _, _) = boiler(true);
        checked(&mut app);
        let group =
            crate::arch::Event::ArchAudit(crate::arch_audit::Event::Group(Some(kentos_commands::ArchAuditCheck::Regulation)));
        let _ = app.update(Message::from(group));
        let shown = app.arch_audit_shown();
        let findings = &app.modules.arch().arch_audit.audited.as_ref().expect("checked").findings;
        let row = shown.iter().position(|&i| findings[i].code == "bykhy_boiler_room_door").expect("the finding shown");
        let _ = app.update(Message::from(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Press(row))));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        app.toasts.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-merdivene-acilan-kapi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
