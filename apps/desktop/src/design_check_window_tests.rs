//! Tasarım denetimi (design_check_window.rs, plan/road.md Y21): the window reads `road.alignments.check` and
//! `road.profiles.check` as the shared cases do.

use crate::app::{App, Dialog, Message};
use crate::design_check_window::{Event as W, Read};

const ALIGNMENTS: &str = include_str!("../../../fixtures/commands/v1/road.alignments.check.json");
const PROFILES: &str = include_str!("../../../fixtures/commands/v1/road.profiles.check.json");

fn check_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(ALIGNMENTS).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn case(text: &str, name: &str) -> serde_json::Value {
    let cases: serde_json::Value = serde_json::from_str(text).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case")["steps"][0].clone()
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::DesignCheck(e));
}

fn read(app: &App) -> Read {
    match &app.design_check_window.as_ref().expect("open").preview {
        Some(Ok(r)) => r.clone(),
        other => panic!("read: {other:?}"),
    }
}

/// Fills the alignment's rules: radius, arc, spiral, tangent, reverse, broken-back, ratio.
fn alignment_rules(app: &mut App, values: [&str; 7]) {
    for (k, v) in values.iter().enumerate() {
        send(app, W::AlignmentField(k, (*v).to_owned()));
    }
}

#[test]
fn the_window_checks_an_alignment_and_a_profile() {
    let mut app = check_app();
    let before = app.document.as_ref().expect("a drawing").model.revision().to_string();
    let _ = app.update(Message::Run("alignment.check"));
    assert!(matches!(app.dialog, Some(Dialog::DesignCheck)));
    send(&mut app, W::Preview);
    assert!(matches!(&app.design_check_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("kural")));
    // Every rule of the shared case at once.
    alignment_rules(&mut app, ["180", "120", "50", "100", "60", "60", "1.2"]);
    send(&mut app, W::Kinks(true));
    send(&mut app, W::Preview);
    let Read::Alignment(a) = read(&app) else { panic!("an alignment") };
    let want = &case(ALIGNMENTS, "Hepsi birden")["result"]["output"];
    assert_eq!(a.findings.len(), want["findings"].as_array().expect("findings").len());
    assert_eq!(a.pairs.len(), 1);
    // The profile, its grades in percent.
    send(&mut app, W::Mode(true));
    send(&mut app, W::Profile("b-1".into()));
    for (k, v) in ["3.5", "2", "15", "25", "100", "120"].iter().enumerate() {
        send(&mut app, W::ProfileField(k, (*v).to_owned()));
    }
    send(&mut app, W::Preview);
    let Read::Profile(p) = read(&app) else { panic!("a profile") };
    let want = &case(PROFILES, "Hepsi birden")["result"]["output"];
    assert_eq!(p.findings.len(), want["findings"].as_array().expect("findings").len());
    assert!(app.design_check_window.as_ref().expect("open").warnings.iter().any(|w| w.contains("tasarım kurallarına uymuyor")));
    send(&mut app, W::Close);
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision().to_string(), before);
}

/// Tasarım denetimi on the reverse pair and on the profile, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y21-denetim-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib design_check_window_tests::screens -- --ignored --nocapture
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
        let mut app = check_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y21-denetim-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("alignment.check"));
        alignment_rules(&mut app, ["180", "120", "50", "100", "60", "60", "1.2"]);
        send(&mut app, W::Kinks(true));
        send(&mut app, W::Preview);
        shot(&mut app, "aks");
        send(&mut app, W::Mode(true));
        send(&mut app, W::Profile("b-1".into()));
        for (k, v) in ["3.5", "2", "15", "25", "100", "120"].iter().enumerate() {
            send(&mut app, W::ProfileField(k, (*v).to_owned()));
        }
        send(&mut app, W::Preview);
        shot(&mut app, "boykesit");
    }
}
