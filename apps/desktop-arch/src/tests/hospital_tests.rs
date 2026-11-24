//! Sağlık yapıları (docs/adr/8110; BYKHY 49/1-b, 49/2) worked by hand through the commands, checked in Denetim with the
//! standards package:
//!
//! - a two-storey hospital (Hastane in the building programme): on 1. kat an L-shaped Koridor, its arms `width` wide,
//!   six patient rooms along its long arm and three along its short one, about 330 m² in all and no wall between them.
//! - The corridor's least width is its arms' (1.80 m), where its largest circle (2.11 m, in the corner) would have
//!   passed 2.00 m: 49/2 fails; 2.20 m arms pass. The storey is one fire compartment: 49/1-b fails.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib hospital_tests::screens -- --ignored --nocapture
//! ```

use crate::arch_ext::*;
use kentos_commands::ArchAuditOutcome as O;
use serde_json::{Value, json};
use crate::arch::ArchState;

use crate::app::App;
use crate::use_class_tests::{building, checked, p, rect, spaces};

/// The hospital's 1. kat: the L-shaped corridor (arms `width` wide, 30 m and 20 m long) and the nine patient rooms.
fn hospital(width: f64) -> App {
    let mut app = building(2, "Hastane");
    let w = width;
    let corridor = json!([{"outer": {"pts": [p(0.0, 0.0), p(30.0, 0.0), p(30.0, w), p(w, w), p(w, 20.0), p(0.0, 20.0)]}}]);
    let mut list: Vec<Value> = vec![json!({"parts": corridor, "name": "Koridor", "arch": {"level": "k01"}})];
    let along = (30.0 - w) / 6.0;
    for i in 0..6 {
        let x = w + along * i as f64;
        list.push(json!({"parts": rect(x, w, x + along, w + 6.0), "name": format!("Hasta odası {}", 101 + i),
                         "unit": "b1", "arch": {"level": "k01"}}));
    }
    let up = (20.0 - (w + 6.0)) / 3.0;
    for i in 0..3 {
        let y = w + 6.0 + up * i as f64;
        list.push(json!({"parts": rect(w, y, w + 6.0, y + up), "name": format!("Hasta odası {}", 107 + i),
                         "unit": "b1", "arch": {"level": "k01"}}));
    }
    spaces(&mut app, list);
    app
}

/// The two articles' outcomes: 49/1-b, 49/2.
fn rows(app: &App) -> [O; 2] {
    let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
    ["BYKHY 49/1-b", "BYKHY 49/2"].map(|rule| {
        set.rules.iter().find(|r| r.rule == rule).unwrap_or_else(|| panic!("{rule}: {:#?}", set.rules)).state
    })
}

fn findings(app: &App) -> Vec<(String, String)> {
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    audited
        .findings
        .iter()
        .filter(|f| f.code == "bykhy_hospital_corridor" || f.code == "bykhy_patient_floor_compartments")
        .map(|f| (f.code.clone(), f.message.clone()))
        .collect()
}

/// The building facts' corridor: its least width, and its largest circle as the architecture facts measure it.
fn corridor_widths(app: &App) -> (f64, f64) {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let corridor = facts.spaces.iter().find(|s| s.name == "Koridor").expect("the corridor");
    let least = corridor.least.expect("a corridor's least width");
    let space = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Space(s) if s.space.name.as_deref() == Some("Koridor") => Some(s.space.clone()),
            _ => None,
        })
        .expect("the corridor drawn");
    (least, kentos_domain::room_width::width_of(&space.parts).0)
}

#[test]
fn an_l_shaped_corridor_is_as_wide_as_its_arms_not_its_corner() {
    let mut app = hospital(1.8);
    let (least, circle) = corridor_widths(&app);
    assert!((least - 1.8).abs() < 1e-3, "{least}");
    // Its largest circle sits in the corner: (2 - √2) · 2 · 1.80 = 2.11 m, wider than the corridor is.
    assert!(circle > 2.1, "{circle}");
    checked(&mut app);
    assert_eq!(rows(&app), [O::Failed, O::Failed]);
    let said = findings(&app);
    assert!(said.iter().any(|(c, m)| c == "bykhy_hospital_corridor" && m.contains("1,80 m < 2,00 m")), "{said:?}");
    assert!(
        said.iter().any(|(c, m)| c == "bykhy_patient_floor_compartments" && m.contains("tek yangın kompartımanı")),
        "{said:?}"
    );
    let mut app = hospital(2.2);
    checked(&mut app);
    assert_eq!(rows(&app), [O::Failed, O::Passed]);
}

/// Denetim's pictures (`.run/gui-evidence/saglik-8110/`): the hospital's narrow corridor and its one compartment.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    use crate::app::Message;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/saglik-8110");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let shot = |app: &mut App, file: &str| {
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            app.toasts.clear();
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let choose = |app: &mut App, code: &str| {
            let group = crate::arch::Event::ArchAudit(crate::arch_audit::Event::Group(Some(
                kentos_commands::ArchAuditCheck::Regulation,
            )));
            let _ = app.update(Message::from(group));
            let shown = app.arch_audit_shown();
            let findings = &app.modules.arch().arch_audit.audited.as_ref().expect("checked").findings;
            let row = shown.iter().position(|&i| findings[i].code == code).expect("the finding shown");
            let _ = app.update(Message::from(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Press(row))));
        };
        let mut app = hospital(1.8);
        checked(&mut app);
        choose(&mut app, "bykhy_hospital_corridor");
        shot(&mut app, "1-dar-koridor");
        choose(&mut app, "bykhy_patient_floor_compartments");
        shot(&mut app, "2-tek-kompartiman");
    }
}
