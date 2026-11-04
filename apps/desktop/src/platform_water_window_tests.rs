//! Platform suyu (platform_water_window.rs, plan/road.md Y22): the window reads `road.corridors.water` as the shared
//! cases do.

use crate::app::{App, Dialog, Message};
use crate::platform_water_window::Event as W;

const CASES: &str = include_str!("../../../fixtures/commands/v1/road.corridors.water.json");

fn water_app() -> App {
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
    let _ = app.update(Message::PlatformWater(e));
}

#[test]
fn the_window_reads_water_on_the_platform() {
    let mut app = water_app();
    let before = app.document.as_ref().expect("a drawing").model.revision().to_string();
    let _ = app.update(Message::Run("corridor.water"));
    assert!(matches!(app.dialog, Some(Dialog::PlatformWater)));
    send(&mut app, W::Preview);
    assert!(matches!(&app.platform_water_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("eğimi yazın")));
    send(&mut app, W::Corridor("c-2".into()));
    send(&mut app, W::Least("3".into()));
    send(&mut app, W::Preview);
    let w = app.platform_water_window.as_ref().expect("open");
    let Some(Ok((r, tris))) = &w.preview else { panic!("a read") };
    let want = &case("Dever geçişli koridor, %3")["result"]["output"];
    assert_eq!(r.flat_triangles as u64, want["flatTriangles"].as_u64().expect("a count"));
    assert_eq!(tris.iter().filter(|t| t.2).count(), r.flat_triangles as usize);
    assert!(w.warnings.iter().any(|x| x.contains("su orada durur")));
    send(&mut app, W::Close);
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision().to_string(), before);
}

/// Platform suyu on the superelevated corridor, wide and narrow, light and dark; `.run/gui-evidence/yol-y22-su-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib platform_water_window_tests::screens -- --ignored --nocapture
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
        let mut app = water_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y22-su-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("corridor.water"));
        send(&mut app, W::Corridor("c-2".into()));
        send(&mut app, W::Least("3".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "dever");
        send(&mut app, W::Corridor("c-1".into()));
        send(&mut app, W::Least("3".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "ana");
    }
}
