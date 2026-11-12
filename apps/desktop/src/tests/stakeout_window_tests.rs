//! Aplikasyon cetveli (stakeout_window.rs, plan/road.md Y10): the window reads the table as it is typed and writes
//! nothing to the drawing; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::stakeout_window::Event as W;

/// A road project with `road.stakeout.query`'s drawing: three corridors; its heights TUDKA99's orthometric (Antalya
/// height, TG-20), as a project that sends to the field says (docs/adr/1031).
fn app() -> App {
    app_with(Some(serde_json::json!({ "kind": "orthometric", "verticalSrid": 5775, "geoid": { "name": "TG-20" } })))
}

fn app_with(heights: Option<serde_json::Value>) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.stakeout.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    setup["settings"]["survey"] = serde_json::json!({ "groundHeight": 1250.0 });
    if let Some(h) = heights {
        setup["settings"]["heights"] = h;
    }
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

/// ROAD-SEC-10 (docs/adr/1031): a project that does not say its heights' system sends nothing to the field and the
/// window says so before; the string road says the systems.
#[test]
fn nothing_goes_to_the_field_without_the_heights_system() {
    let said = |app: &App| app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    let folder = std::env::temp_dir().join(format!("kentos-stakeout-heights-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut unsaid = app_with(None);
    let _ = unsaid.update(Message::Run("stakeout.table"));
    send(&mut unsaid, W::Field(2, "20".into()));
    send(&mut unsaid, W::Sent(Some(folder.join("ana"))));
    assert!(!folder.join("ana.xml").exists(), "nothing sent");
    assert!(said(&unsaid).contains("Proje ayarları › Kotlar"), "{}", said(&unsaid));
    let mut known = app();
    let _ = known.update(Message::Run("stakeout.table"));
    send(&mut known, W::Field(2, "20".into()));
    send(&mut known, W::Sent(Some(folder.join("ana"))));
    let xml = std::fs::read_to_string(folder.join("ana.xml")).expect("written");
    assert!(xml.contains("verticalCoordinateSystemName=\"Antalya height\""), "{xml}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// The window's footer with the heights said and not, light; `.run/gui-evidence/yol-y96/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib stakeout_window_tests::screens_heights -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_heights() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y96");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (made, step) in [(app(), "kotlar"), (app_with(None), "kotsuz")] {
        let mut app = made;
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("stakeout.table"));
        send(&mut app, W::Field(0, "200".into()));
        send(&mut app, W::Field(1, "400".into()));
        send(&mut app, W::Field(2, "20".into()));
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y96-aplikasyon-{step}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
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

/// ROAD-IO-12 (docs/adr/1038): the table says what it is computed from, and the sheet written beside it says the
/// same with the time it was written.
#[test]
fn the_table_says_its_sources_and_when_it_was_written() {
    let mut app = app();
    let _ = app.update(Message::Run("stakeout.table"));
    send(&mut app, W::Field(2, "20".into()));
    let table = app.stakeout_window.as_ref().and_then(|w| w.preview.table.clone()).expect("reads");
    assert_eq!((table.sources.corridor.id.as_str(), table.sources.alignment.id.as_str()), ("c-1", "g-1"));
    let sheet = kentos_native_application::road_stakeout::sources_sheet(&table, "2026-11-07 10:00:00");
    assert_eq!(sheet.name, "Kaynaklar");
    assert_eq!(sheet.rows[0], vec!["Üretildi".to_owned(), "2026-11-07 10:00:00".to_owned(), String::new()]);
    assert!(sheet.rows.iter().any(|r| r[0] == "Tip kesit" && r[1] == "tk-2" && r[2] == "2"), "{:?}", sheet.rows);
    // Docs/adr/1040: the ground's digest of now with its acceptance.
    assert!(
        sheet.rows.iter().any(|r| r[0] == "Arazi" && r[2].len() == 64 + " (kabul edilmedi)".len() && r[2].ends_with(" (kabul edilmedi)")),
        "{:?}",
        sheet.rows
    );
}
