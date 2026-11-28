//! İçte uygulanan ısı ve ses yalıtımı (docs/adr/8151; BYKHY 20/3, Ek-2/Ç) worked by hand on a one-storey dwelling,
//! 6 × 4 m on its walls' axes: the exterior walls drawn anticlockwise (the inside on each one's left), 30 cm, their
//! layers from the left face Sıva 2 cm, EPS 5 cm, Tuğla 20 cm, Taş yünü 3 cm; a 20 cm partition on x = 2 of Sıva, Taş
//! yünü 16 cm, Sıva; Salon west of it, Oda east, drawn to the walls' faces. Malzemeler: Tuğla Taşıyıcı, EPS and Taş
//! yünü Yalıtım, Sıva Kaplama.
//!
//! - Each exterior wall's inner face is its left (a room 15 cm in): its inner layers are Sıva and EPS, up to the Tuğla;
//!   the Taş yünü outside it is the façade's. The partition has rooms on both sides: its Taş yünü is inside.
//! - EPS F (kolay alevlenici) fails 20/3's normal alevlenici, one finding for the four walls; Taş yünü A1 holds. EPS
//!   B-s1,d0 (zor alevlenici): 20/3 holds.

use crate::arch::ArchState;
use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S};
use serde_json::json;

use crate::app::App;
use crate::use_class_tests::{building, checked, p, rect, run, spaces};

/// The dwelling, its materials, walls and rooms; the exterior walls' persistent ids, then the partition's.
fn scene() -> (App, Vec<String>) {
    let mut app = building(1, "Konut");
    let materials = [
        ("Tuğla", "structure"),
        ("EPS", "insulation"),
        ("Taş yünü", "insulation"),
        ("Sıva", "finish"),
    ];
    for (name, category) in materials {
        run(
            &mut app,
            "cad.materials.edit",
            json!({"operation": {"kind": "define", "name": name, "change": {"category": category}}}),
        );
    }
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    // The two wall styles, their layers bound to the materials by name.
    let c = |name: &str, t: f64| json!({"name": name, "material": name, "thickness": t});
    for (id, name, layers) in [
        (
            "dis-duvar",
            "Dış duvar",
            json!([
                c("Sıva", 0.02),
                c("EPS", 0.05),
                c("Tuğla", 0.20),
                c("Taş yünü", 0.03)
            ]),
        ),
        (
            "bolme",
            "Bölme",
            json!([c("Sıva", 0.02), c("Taş yünü", 0.16), c("Sıva", 0.02)]),
        ),
    ] {
        run(
            &mut app,
            "cad.styles.edit",
            json!({"kind": "wall", "operation": "create", "wall": {"id": id, "name": name, "components": layers}}),
        );
    }
    let wall = |a: (f64, f64), b: (f64, f64), width: f64, style: &str| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": width, "style": style, "arch": {"level": "k00"}});
    let made = run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (6.0, 0.0), 0.3, "dis-duvar"), wall((6.0, 0.0), (6.0, 4.0), 0.3, "dis-duvar"),
            wall((6.0, 4.0), (0.0, 4.0), 0.3, "dis-duvar"), wall((0.0, 4.0), (0.0, 0.0), 0.3, "dis-duvar"),
            wall((2.0, 0.0), (2.0, 4.0), 0.2, "bolme")
        ]}),
    );
    let uids: Vec<String> = made["output"]["uids"]
        .as_array()
        .expect("the walls")
        .iter()
        .map(|u| u.as_str().expect("a uid").to_owned())
        .collect();
    spaces(
        &mut app,
        vec![
            json!({"parts": rect(0.15, 0.15, 1.9, 3.85), "name": "Salon", "unit": "b1", "arch": {"level": "k00"}}),
            json!({"parts": rect(2.1, 0.15, 5.85, 3.85), "name": "Oda", "unit": "b1", "arch": {"level": "k00"}}),
        ],
    );
    (app, uids)
}

/// 20/3's row and its inner insulation findings (state, message, objects).
#[allow(clippy::type_complexity)]
fn said(app: &App) -> (Option<O>, Vec<(Option<S>, String, usize)>) {
    let set = app
        .modules
        .arch()
        .arch_audit
        .rule_set
        .as_ref()
        .expect("the rule set");
    let row = set
        .rules
        .iter()
        .find(|r| r.rule == "BYKHY 20/3")
        .map(|r| r.state);
    let audited = app
        .modules
        .arch()
        .arch_audit
        .audited
        .as_ref()
        .expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code == "bykhy_wall_insulation_reaction")
        .map(|f| (f.state, f.message.clone(), f.uids.len()))
        .collect();
    (row, found)
}

#[test]
fn a_wall_s_insulation_inside_its_load_bearing_layer_is_read_by_20_3() {
    let (mut app, walls) = scene();
    let doc = &app.document.as_ref().expect("a drawing").model;
    // The domain: the exterior walls' EPS (their left, inner face), the partition's Taş yünü; no façade Taş yünü.
    let read: Vec<(String, bool, Vec<String>, usize, bool)> = doc
        .inner_insulations()
        .into_iter()
        .map(|w| {
            let layers = w.layers.iter().map(|l| l.material.clone()).collect();
            (w.wall, w.exterior, layers, w.rooms.len(), w.side_known)
        })
        .collect();
    let expected: Vec<(String, bool, Vec<String>, usize, bool)> = walls
        .iter()
        .enumerate()
        .map(|(i, w)| {
            if i < 4 {
                (w.clone(), true, vec!["EPS".to_owned()], 1, true)
            } else {
                (w.clone(), false, vec!["Taş yünü".to_owned()], 2, true)
            }
        })
        .collect();
    assert_eq!(read, expected);
    // The facts: five layers, no class declared yet: 20/3 lacks them.
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    assert_eq!(facts.insulations.len(), 5, "{:?}", facts.insulations);
    assert!(
        facts
            .insulations
            .iter()
            .all(|i| i.reaction.is_none() && !i.unknown_side)
    );
    checked(&mut app);
    let (row, found) = said(&app);
    assert_eq!(row, Some(O::MissingData));
    assert_eq!(found.len(), 2, "{found:#?}");
    // EPS F, Taş yünü A1: EPS fails, one finding for the four walls.
    crate::finish_reaction_tests::declared(&mut app, "EPS", "F");
    crate::finish_reaction_tests::declared(&mut app, "Taş yünü", "A1");
    app.dialog = None;
    app.modules.arch_mut().materials = None;
    checked(&mut app);
    let (row, found) = said(&app);
    assert_eq!(row, Some(O::Failed));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!((found[0].0, found[0].2), (None, 4));
    let words = "“EPS” iç yalıtımı (4 duvar): malzemesi “EPS” F, kolay alevlenici";
    assert!(found[0].1.starts_with(words), "{}", found[0].1);
    // EPS B-s1,d0, zor alevlenici: 20/3 holds.
    crate::finish_reaction_tests::declared(&mut app, "EPS", "B-s1,d0");
    app.dialog = None;
    app.modules.arch_mut().materials = None;
    checked(&mut app);
    let (row, found) = said(&app);
    assert_eq!((row, found.len()), (Some(O::Passed), 0), "{found:#?}");
}

/// Denetim's 20/3 finding of the exterior walls' EPS F beside the plan, light and dark;
/// `.run/gui-evidence/ic-yalitim-8151/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib wall_insulation_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    use crate::app::Message;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/ic-yalitim-8151");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(1280.0, 800.0), "koyu"),
    ] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, _) = scene();
        crate::finish_reaction_tests::declared(&mut app, "EPS", "F");
        crate::finish_reaction_tests::declared(&mut app, "Taş yünü", "A1");
        app.dialog = None;
        app.modules.arch_mut().materials = None;
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        checked(&mut app);
        let audited = app
            .modules
            .arch()
            .arch_audit
            .audited
            .as_ref()
            .expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited
            .findings
            .iter()
            .position(|f| f.code == "bykhy_wall_insulation_reaction");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-ic-yalitim-denetim-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}
