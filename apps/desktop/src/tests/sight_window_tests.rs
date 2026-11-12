//! Görüş mesafesi (sight_window.rs, plan/road.md Y20): the window reads `road.profiles.sight` as the shared cases do.

use crate::app::{App, Dialog, Message};
use crate::road;
use crate::sight_window::Event as W;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/road.profiles.sight.json");

/// `road.profiles.sight`'s drawing: the profiles of the shared cases.
fn sight_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn case(name: &str) -> serde_json::Value {
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case")["steps"][0].clone()
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::Sight(e)));
}

/// Fills the window from a case's input (no range: the window reads the whole profile).
fn fill(app: &mut App, input: &serde_json::Value) {
    send(app, W::Profile(input["profile"].as_str().expect("a profile").to_owned()));
    for (k, key) in ["eye", "object", "reach", "step", "required"].iter().enumerate() {
        let text = input[key].as_f64().map_or(String::new(), |v| v.to_string());
        send(app, W::Field(k, text));
    }
}

#[test]
fn the_window_reads_a_profile_s_sight_distance() {
    let mut app = sight_app();
    let before = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .revision()
        .to_string();
    let _ = app.update(Message::Run("profile.sight"));
    assert!(matches!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Sight))
    ));
    send(&mut app, W::Preview);
    assert!(
        matches!(&app.road.sight_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("yazılmadı"))
    );
    // The whole crest profile, as the shared case reads it by its name.
    let step = case("Ad ile, bütün boykesit");
    fill(&mut app, &step["input"]);
    send(&mut app, W::Preview);
    let Some(Ok(r)) = &app.road.sight_window.as_ref().expect("open").preview else {
        panic!("a sight")
    };
    let want = &step["result"]["output"];
    assert_eq!(r.rows.len(), want["rows"].as_array().expect("rows").len());
    let lf = r.least_forward.as_ref().expect("ahead");
    assert!((lf.distance - want["leastForward"]["distance"].as_f64().expect("a distance")).abs() < 2e-3);
    // A required distance not met warns and gathers the short stretches.
    send(&mut app, W::Field(4, "150".into()));
    send(&mut app, W::Preview);
    let w = app.road.sight_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else {
        panic!("a sight")
    };
    assert!(!r.short.is_empty());
    assert!(w.warnings.iter().any(|x| x.contains("150 m'den kısa")));
    send(&mut app, W::Close);
    assert!(app.road.sight_window.is_none());
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .revision()
            .to_string(),
        before
    );
}

/// Görüş mesafesi on the circle crest and the parabola, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y20-gorus-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib sight_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = sight_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y20-gorus-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("profile.sight"));
        fill(&mut app, &serde_json::json!({ "profile": "b-1", "eye": 1.08, "object": 0.6, "reach": 300.0, "step": 10.0, "required": 150.0 }));
        send(&mut app, W::Preview);
        shot(&mut app, "daire");
        fill(&mut app, &serde_json::json!({ "profile": "b-4", "eye": 1.08, "object": 0.6, "reach": 250.0, "step": 5.0, "required": 140.0 }));
        send(&mut app, W::Preview);
        shot(&mut app, "parabol");
    }
}
