//! Kırmızı kot önerisi (propose_window.rs, plan/road.md Y25): the window reads `road.profiles.propose` as the shared
//! cases do and writes the proposal as a new profile.

use crate::app::{App, Dialog, Message};
use crate::propose_window::Event as W;

const CASES: &str = include_str!("../../../fixtures/commands/v1/road.profiles.propose.json");

fn case(name: &str) -> serde_json::Value {
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case")["steps"][0].clone()
}

fn propose_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Propose(e));
}

#[test]
fn the_window_proposes_and_writes_a_profile() {
    let mut app = propose_app();
    let _ = app.update(Message::Run("profile.propose"));
    assert!(matches!(app.dialog, Some(Dialog::Propose)));
    send(&mut app, W::Profile("b-4".into()));
    send(&mut app, W::Ends(true));
    send(&mut app, W::Preview);
    let w = app.propose_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    let want = &case("Kurplu, uçlar kilitli")["result"]["output"];
    for (p, q) in r.answer.pvis.iter().zip(want["pvis"].as_array().expect("pvis")) {
        assert!((p.elevation - q["elevation"].as_f64().expect("an elevation")).abs() < 1e-6);
    }
    let before = app.document.as_ref().expect("a drawing").model.road_profiles().len();
    send(&mut app, W::Write);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.road_profiles().len(), before + 1);
    assert!(doc.road_profiles().iter().any(|p| p.name == "Uzun kot önerisi"));
    assert!(app.propose_window.is_none());
}

/// Kırmızı kot önerisi, wide and narrow, light and dark; `.run/gui-evidence/yol-y25-oneri-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib propose_window_tests::screens -- --ignored --nocapture
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
        for (file, ends, most) in [("serbest", false, ""), ("sinirli", true, "2")] {
            let mut app = propose_app();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            let _ = app.update(Message::RibbonTab("alignment"));
            let _ = app.update(Message::Run("profile.propose"));
            send(&mut app, W::Profile("b-4".into()));
            send(&mut app, W::Ends(ends));
            send(&mut app, W::Most(most.into()));
            send(&mut app, W::Preview);
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("yol-y25-oneri-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
            println!("{}", path.display());
        }
    }
}

/// Y40.3 (docs/adr/0946): a profile's crossings hold the proposal as `road.profiles.propose`'s shared case does: the
/// overpass and the culvert at their limits, the pipe clear.
#[test]
fn the_crossings_hold_the_proposal() {
    let mut app = propose_app();
    let _ = app.update(Message::Run("profile.propose"));
    send(&mut app, W::Profile("b-6".into()));
    send(&mut app, W::Preview);
    let w = app.propose_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    let want = &case("Üst geçit altında ve menfez üstünde: ikisi de tutuyor")["result"]["output"];
    for (c, q) in r.answer.crossings.iter().zip(want["crossings"].as_array().expect("crossings")) {
        assert!((c.margin.expect("on the line") - q["margin"].as_f64().unwrap()).abs() < 1e-6, "{}", c.name);
        assert_eq!(c.holds, q.get("holds").and_then(serde_json::Value::as_bool).unwrap_or(false), "{}", c.name);
    }
    assert_eq!(r.answer.crossings.iter().filter(|c| c.holds).count(), 2);
}

/// The proposal held by its crossings, light and dark; `.run/gui-evidence/yol-y40/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib propose_window_tests::screens_crossings -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_crossings() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y40");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = propose_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("profile"));
        let _ = app.update(Message::Run("profile.propose"));
        send(&mut app, W::Profile("b-6".into()));
        send(&mut app, W::Preview);
        send(&mut app, W::Detail(crate::propose_window::Detail::Crossings));
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y40-oneri-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        // The same profile in Boykesit: its line as it is, the culvert short.
        let _ = app.update(Message::Propose(W::Close));
        let _ = app.update(Message::Run("profile.editor"));
        let rows = app.profile_window.as_ref().and_then(|w| w.rows.iter().position(|r| r_id(r) == "b-6"));
        if let Some(i) = rows {
            let _ = app.update(Message::Profiles(crate::profile_window::Event::Choose(i)));
        }
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y40-boykesit-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

fn r_id(r: &crate::profile_window::Row) -> String {
    r.id_text()
}
