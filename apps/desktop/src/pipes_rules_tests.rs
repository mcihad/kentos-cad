//! Boru hattı denetimi, docs/adr/4780 (plan/map-cad.md MAP-16) on the shared cases' outers drawing (the sewer's pipes
//! carry `Et` and `Dış çap`): the window finds the outer diameter's attribute beside Et and its answer is the shared
//! case's (P4 and P8 said); Kural › Atıksu takes the limits not written from the 2017 sewer regulation's EK-1, the
//! table shows each pipe's class and limits; Hat profili draws the run's own outer walls. And its pictures.

use kentos_commands::Entity;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};
use crate::pipes_check_window::Event as P;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.pipes.check.json")).expect("cases")
}

fn case(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone()
}

/// The outers' drawing, the window open on the sewer with the cases' catalogue.
fn opened() -> App {
    kentos_processing::command::register_as("desktop");
    let setup = case("dış çap: et dış çaptan, iç çapın altındaki dış çap söylenir")["setup"].clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("pipes.check"));
    let _ = app.update(Message::PipesCheck(P::Catalog("200 300 400 500".into())));
    app
}

fn answer(app: &App) -> Value {
    serde_json::to_value(app.pipes_check.answer.clone().expect("an answer").expect("checked")).expect("json")
}

#[test]
fn the_outer_diameter_is_found_and_the_answer_is_the_cases() {
    let mut app = opened();
    assert_eq!((app.pipes_check.wall.as_str(), app.pipes_check.outer.as_str()), ("Et", "Dış çap"), "both guessed");
    for e in [P::MinCover("0,8".into()), P::MaxCover("3".into()), P::MinSlope("9".into()), P::MaxSlope("50".into())] {
        let _ = app.update(Message::PipesCheck(e));
    }
    let want = &case("dış çap ve et birlikte: uyuşmayan söylenir, etsiz ölçülür")["steps"][0]["result"]["output"];
    assert_eq!(&answer(&app), want);
    // Without Et, the wall is the outer's alone: P4's is half its outer less its inner.
    let _ = app.update(Message::PipesCheck(P::Wall(String::new())));
    let want = &case("dış çap: et dış çaptan, iç çapın altındaki dış çap söylenir")["steps"][0]["result"]["output"];
    assert_eq!(&answer(&app), want);
}

#[test]
fn the_rules_take_the_limits_not_written() {
    let mut app = opened();
    let _ = app.update(Message::PipesCheck(P::Wall(String::new())));
    let _ = app.update(Message::PipesCheck(P::Outer(String::new())));
    let _ = app.update(Message::PipesCheck(P::Rules("atiksu".into())));
    let want = &case("kural: atıksu yönetmeliğinin EK-1'i; eğim çap sınıfından, en az örtü 2,70 m")["steps"][0]["result"]["output"];
    assert_eq!(&answer(&app), want);
    // The table's limits and the summary's words are the package's.
    assert_eq!(crate::pipes_check_rules::limits("atiksu", "Tali kanal").as_deref(), Some("2,00–40,00"));
    assert_eq!(crate::pipes_check_rules::limits("yagmursuyu", "φ400").as_deref(), Some("1,67–40,00 (istisnai 1,11)"));
    let f = app.format();
    let line = crate::pipes_check_rules::line("atiksu", &f).expect("a line");
    assert!(line.starts_with("Kural: Atıksu — Atıksu Toplama ve Uzaklaştırma Sistemleri Hakkında Yönetmelik"), "{line}");
    assert!(line.contains("(4 sınıf)") && line.contains(&format!("en az örtü {}", f.length(2.7))), "{line}");
    assert!(line.contains(&format!("en çok {} (φ < 1200)", f.length(70.0))) && line.contains("uygunluk kararı idarenindir"), "{line}");
    // A limit written goes first: the least slope 9 ‰ finds P3 (8 ‰), the package's 1:300 did not.
    let _ = app.update(Message::PipesCheck(P::MinSlope("9".into())));
    let r = app.pipes_check.answer.clone().expect("an answer").expect("checked");
    assert!(r.pipes[2].findings.contains(&"lowSlope".to_owned()), "{:?}", r.pipes[2].findings);
}

/// The thin red lines drawn after `before`: the run's outer walls.
fn outer_walls(app: &App, before: usize) -> usize {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities()
        .skip(before)
        .filter(|e| matches!(e, Entity::Polyline(_)) && e.base().line_weight == Some(0.13) && e.base().color.as_deref() == Some("#d62828"))
        .count()
}

#[test]
fn the_profile_draws_the_runs_own_outer_walls() {
    let mut app = opened();
    let _ = app.update(Message::PipesCheck(P::Outer(String::new())));
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::PipesCheck(P::DrawProfile));
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("6 borunun dış duvarı"), "{said}");
    assert_eq!(outer_walls(&app, before), 2, "the bottom outside and the top outside");
    // Without Et, no outer wall.
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Hat profili"));
    let mut app2 = opened();
    let _ = app2.update(Message::PipesCheck(P::Wall(String::new())));
    let _ = app2.update(Message::PipesCheck(P::Outer(String::new())));
    let before = app2.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app2.update(Message::PipesCheck(P::DrawProfile));
    assert_eq!(outer_walls(&app2, before), 0);
    assert!(!crate::files_testing::last_said(&app2).contains("borunun dış duvarı"));
}

/// `.run/gui-evidence/kanal-kurali-4780/`: the window with Kural › Atıksu and Dış çap (each pipe's class and limits in
/// the table), and the run's profile with its own outer walls, close on B4 where P4 leaves.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib pipes_rules_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kanal-kurali-4780");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::PipesCheck(P::Rules("atiksu".into())));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // The profile with Et alone (P4 and P8 agree with no outer then), its own outer walls drawn.
        let _ = app.update(Message::PipesCheck(P::Outer(String::new())));
        let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
        let _ = app.update(Message::PipesCheck(P::DrawProfile));
        let doc = &app.document.as_ref().expect("a drawing").model;
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut invert = None;
        for e in doc.entities().skip(before) {
            if let Entity::Polyline(p) = e {
                for v in &p.pts {
                    (x0, y0, x1, y1) = (x0.min(v.x), y0.min(v.y), x1.max(v.x), y1.max(v.y));
                }
                if e.base().line_weight == Some(0.5) {
                    invert = p.pts.get(6).map(|v| (v.x, v.y));
                }
            }
        }
        // Close on B4, where P4 (30 mm wall) leaves at 150 m: the outer bottom, the invert, the crown, the outer top.
        let (bx, by) = invert.expect("the invert line");
        for (bounds, file) in [
            (kentos_render_wgpu::Bounds { min_x: x0 - 30.0, min_y: y0 - 10.0, max_x: x1 + 10.0, max_y: y1 + 10.0 }, "2-profil"),
            (kentos_render_wgpu::Bounds { min_x: bx - 7.0, min_y: by - 2.0, max_x: bx + 7.0, max_y: by + 6.5 }, "3-b4-yakin"),
        ] {
            app.viewport.camera.fit(&bounds, 12.0);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
