//! Aplikasyon cetveli (stakeout_window.rs, plan/road.md Y10): the window reads the table as it is typed and writes
//! nothing to the drawing; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::stakeout_window::Event as W;

/// A road project with `road.stakeout.query`'s drawing: three corridors.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.stakeout.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    setup["settings"]["survey"] = serde_json::json!({ "groundHeight": 1250.0 });
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Stakeout(e));
}

#[test]
fn the_window_reads_the_table_as_it_is_typed() {
    let mut app = app();
    let revision = app.document.as_ref().expect("a drawing").model.revision();
    let _ = app.update(Message::Run("stakeout.table"));
    assert_eq!(app.dialog, Some(Dialog::Stakeout));
    send(&mut app, W::Field(2, "50".into()));
    send(&mut app, W::Field(3, "axis, daylight".into()));
    let w = app.stakeout_window.as_ref().expect("open");
    let t = w.preview.table.as_ref().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert!(t.rows.iter().all(|r| r.code == "axis" || r.code.ends_with(".daylight")), "{t:?}");
    assert!(t.rows.iter().all(|r| r.ground_offset.is_none()));
    send(&mut app, W::Distances(kentos_commands::StakeoutDistances::Ground));
    let t = app.stakeout_window.as_ref().and_then(|w| w.preview.table.clone()).expect("reads");
    let r = t.rows.iter().find(|r| r.code != "axis").expect("a side point");
    let g = r.ground_offset.expect("a ground length");
    assert!(g.abs() > r.offset.abs() * 0.99 && g.abs() < r.offset.abs() * 1.01, "{r:?}");
    send(&mut app, W::Field(3, "kerb".into()));
    let why = app.stakeout_window.as_ref().and_then(|w| w.preview.why.clone()).expect("refused");
    assert!(why.contains("kerb"), "{why}");
    send(&mut app, W::Close);
    assert!(app.stakeout_window.is_none());
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision(), revision, "nothing written");
}

#[test]
fn the_points_go_to_an_instrument_and_as_a_string_road() {
    let mut app = app();
    let revision = app.document.as_ref().expect("a drawing").model.revision();
    let _ = app.update(Message::Run("stakeout.table"));
    send(&mut app, W::Field(0, "100".into()));
    send(&mut app, W::Field(1, "200".into()));
    send(&mut app, W::Field(2, "20".into()));
    let folder = std::env::temp_dir().join(format!("kentos-stakeout-send-window-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    // A string road: the file's name without its extension takes it.
    send(&mut app, W::Sent(Some(folder.join("ana"))));
    let xml = std::fs::read_to_string(folder.join("ana.xml")).expect("written");
    assert!(xml.contains("<PlanFeatures name=\"Ana koridor\">"), "{xml}");
    // Leica GSI-8 from 500: the long codes are left off.
    send(&mut app, W::SendFormat(kentos_commands::StakeoutSendFormat::Gsi8));
    send(&mut app, W::SendField(0, "500".into()));
    send(&mut app, W::Sent(Some(folder.join("ana.gsi"))));
    let gsi = std::fs::read_to_string(folder.join("ana.gsi")).expect("written");
    assert!(gsi.starts_with("110001+00000500 "), "{gsi}");
    send(&mut app, W::SendField(0, "bir".into()));
    send(&mut app, W::Sent(Some(folder.join("yok.gsi"))));
    assert!(!folder.join("yok.gsi").exists());
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision(), revision, "nothing written");
    let _ = std::fs::remove_dir_all(&folder);
}

/// The window, wide and narrow, light and dark; `.run/gui-evidence/yol-y10-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib stakeout_window_tests::screens -- --ignored --nocapture
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
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y10-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("stakeout.table"));
        send(&mut app, W::Field(0, "200".into()));
        send(&mut app, W::Field(1, "400".into()));
        send(&mut app, W::Field(2, "20".into()));
        shot(&mut app, "aplikasyon");
        send(&mut app, W::Field(3, "axis, hinge, daylight".into()));
        send(&mut app, W::Distances(kentos_commands::StakeoutDistances::Ground));
        shot(&mut app, "aplikasyon-zemin");
        send(&mut app, W::SendFormat(kentos_commands::StakeoutSendFormat::Gsi16));
        send(&mut app, W::SendField(0, "1001".into()));
        send(&mut app, W::SendField(1, "Ana yol".into()));
        shot(&mut app, "aplikasyon-gonder");
        send(&mut app, W::Close);
    }
}
