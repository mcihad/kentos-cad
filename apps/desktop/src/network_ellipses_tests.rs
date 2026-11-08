//! Ağ dengelemesi's Hata elipsleri row (survey_network_window.rs, plan/map-cad.md MAP-05, docs/adr/2960) on the
//! shared case's archive whose baselines' deviations are turned 30°: Kabul et with the box ticked draws the three
//! points' error ellipses on the layer chosen in Katmanlar, magnified as typed, in the same undo step as the points;
//! the archive records them as a use; unticked, none. And the picture.

use kentos_commands::Entity;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};
use crate::survey_network_window::Event as N;

fn opened() -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/map.survey.edit.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.contains("sapmalar 30° dönük: elipslerin")))
        .expect("the case")
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn ellipses(app: &App) -> Vec<kentos_commands::EllipseEntity> {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model
        .entities()
        .filter_map(|e| match e {
            Entity::Ellipse(el) => Some(el.clone()),
            _ => None,
        })
        .collect()
}

/// The window opened, the ellipses' box as given, magnified `times`, accepted.
fn accepted(draw: bool, times: f64) -> App {
    let mut app = opened();
    app.selected_layer = Some("gnss".into());
    let _ = app.update(Message::Run("map.survey.adjustment"));
    let _ = app.update(Message::SurveyNetwork(N::Ellipses(draw)));
    let _ = app.update(Message::SurveyNetwork(N::Magnify(times)));
    let _ = app.update(Message::SurveyNetwork(N::Accept));
    app
}

#[test]
fn kabul_et_draws_the_ellipses_with_the_points_in_one_step() {
    let app = accepted(true, 2000.0);
    let drawn = ellipses(&app);
    assert_eq!(drawn.len(), 3);
    assert!(drawn.iter().all(|e| e.base.layer_id == "gnss" && e.base.attrs.get("Büyütme").map(String::as_str) == Some("2000")));
    // Turned some 30° east of the map's north, the convergence taken off; the minor axis 0.375 of the major.
    let e = &drawn[0];
    let bearing = e.major.x.atan2(e.major.y).to_degrees();
    assert!((bearing - 29.67).abs() < 0.05 && (e.ratio - 0.375).abs() < 0.001, "{bearing} {}", e.ratio);
    let survey = app.document.as_ref().and_then(|d| d.model.map_survey()).cloned().expect("an archive");
    let used = survey.uses.iter().find(|u| u.operation == "Hata elipsleri (da-1)").expect("recorded");
    assert_eq!(used.uids.len(), 3);
    // One undo step takes the points and the ellipses away.
    let mut app = app;
    let _ = app.update(Message::Run("edit.undo"));
    assert!(ellipses(&app).is_empty());
    // Unticked: none.
    assert!(ellipses(&accepted(false, 2000.0)).is_empty());
}

/// The network accepted with ellipses 50 000 times (3 mm as 150 m), fitted, light wide and dark narrow,
/// `.run/gui-evidence/hata-elipsleri-2960/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib network_ellipses_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/hata-elipsleri-2960");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.selected_layer = Some("gnss".into());
        let _ = app.update(Message::Run("map.survey.adjustment"));
        let _ = app.update(Message::SurveyNetwork(N::Ellipses(true)));
        let _ = app.update(Message::SurveyNetwork(N::Magnify(50_000.0)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::SurveyNetwork(N::Accept));
        app.dialog = None;
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("elipsler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
