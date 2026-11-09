//! Boru hattı denetimi, docs/adr/4360 (plan/map-cad.md MAP-16): the run's profile drawn from the window with Diğer ağ
//! draws each crossing pipe's outer wall round its inner one and fills the ring between them when the other network's
//! Et is known; and the window's answer is read again when the network changes while it is open (an attribute written
//! by a script). And its pictures.

use kentos_commands::Entity;
use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::pipes_check_window::Event as P;

const P1: &str = "0192f640-1640-7000-8000-0000000000f1";

/// The walls case's drawing of `cad.pipes.check` (the pipes and the water pipes with `Et`).
fn walled() -> App {
    kentos_processing::command::register_as("desktop");
    let cases: Value = serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.pipes.check.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("et kalınlığı: örtü")))
        .expect("the walls case")
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The ellipses drawn after `before`: (centre, semi-axis across).
fn ellipses(app: &App, before: usize) -> Vec<([f64; 2], f64)> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities()
        .skip(before)
        .filter_map(|e| match e {
            Entity::Ellipse(e) => {
                let m = e.major.x.hypot(e.major.y);
                Some(([e.c.x, e.c.y], if e.major.x.abs() > e.major.y.abs() { m } else { m * e.ratio }))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_profile_draws_the_crossing_pipes_outer_walls() {
    let mut app = walled();
    let _ = app.update(Message::Run("pipes.check"));
    let _ = app.update(Message::PipesCheck(P::Other("su".into())));
    assert_eq!(app.pipes_check.other_wall, "Et", "the water pipes' wall guessed");
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::PipesCheck(P::DrawProfile));
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("3 kesişen borunun dış duvarı"), "{said}");
    // W3, W1 (5 mm) and W2 (8 mm): each an inner and an outer ellipse about one centre, the outer the wall wider.
    let drawn = ellipses(&app, before);
    assert_eq!(drawn.len(), 6, "{drawn:?}");
    let walls = [0.005, 0.005, 0.008];
    for (pair, wall) in drawn.chunks(2).zip(walls) {
        assert_eq!(pair[0].0, pair[1].0, "one centre");
        assert!((pair[1].1 - pair[0].1 - wall).abs() < 1e-12, "{pair:?} {wall}");
    }
    let doc = &app.document.as_ref().expect("a drawing").model;
    let rings = doc.entities().skip(before).filter(|e| matches!(e, Entity::Hatch(h) if h.boundary.as_ref().is_some_and(|b| b.len() == 2))).count();
    assert_eq!(rings, 3, "the rings filled");
    // Without the water pipes' Et: no outer wall.
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Hat profili"));
    let _ = app.update(Message::Run("pipes.check"));
    let _ = app.update(Message::PipesCheck(P::Other("su".into())));
    let _ = app.update(Message::PipesCheck(P::OtherWall(String::new())));
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::PipesCheck(P::DrawProfile));
    assert_eq!(ellipses(&app, before).len(), 3);
    assert!(!crate::files_testing::last_said(&app).contains("dış duvar"));
}

#[test]
fn the_check_is_read_again_when_the_network_changes_while_it_is_open() {
    let mut app = walled();
    let _ = app.update(Message::Run("pipes.check"));
    let slope = |app: &App| {
        let r = app.pipes_check.answer.clone().expect("an answer").expect("checked");
        r.pipes.iter().find(|p| p.uid == P1).and_then(|p| p.slope).expect("P1's slope")
    };
    let before = slope(&app);
    // A script lowers P1's lower invert by 30 cm (97.5 to 97.2); the next message the app takes reads the answer again.
    crate::program_scenes::command(&mut app, "cad.entities.set", json!({"uids": [P1], "attrs": {"Akar son": "97.20"}, "operation": "attributes"}));
    assert_eq!(slope(&app), before, "not read yet");
    let _ = app.update(Message::WindowResized(iced::Size::new(1440.0, 900.0)));
    let after = slope(&app);
    assert!(after > before, "{before} → {after}");
}

/// `.run/gui-evidence/dis-duvar-4360/`: the profile's crossing pipes with their outer walls, close on W2.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib pipes_profile_walls_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dis-duvar-4360");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = walled();
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pipes.check"));
        let _ = app.update(Message::PipesCheck(P::Other("su".into())));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        // The window first, so what it queued (the extents' fit) is done before the camera is set below.
        snapshot.settle(&mut app, App::view, &mut update);
        let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
        let _ = app.update(Message::PipesCheck(P::DrawProfile));
        let drawn = ellipses(&app, before);
        let (x0, x1) = drawn.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), (c, _)| (a.min(c[0]), b.max(c[0])));
        let (y0, y1) = drawn.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), (c, _)| (a.min(c[1]), b.max(c[1])));
        let w2 = drawn.get(4).map(|(c, _)| *c).expect("W2");
        for (bounds, file) in [
            (kentos_render_wgpu::Bounds { min_x: x0 - 15.0, min_y: y0 - 45.0, max_x: x1 + 15.0, max_y: y1 + 45.0 }, "1-profil"),
            (kentos_render_wgpu::Bounds { min_x: w2[0] - 1.2, min_y: w2[1] - 1.6, max_x: w2[0] + 1.2, max_y: w2[1] + 2.2 }, "2-w2-yakin"),
        ] {
            app.viewport.camera.fit(&bounds, 12.0);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
