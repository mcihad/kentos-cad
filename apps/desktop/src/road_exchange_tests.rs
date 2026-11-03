//! LandXML ver and LandXML'den aks al (road_exchange.rs, plan/road.md Y10.2): a road project's alignments go out and
//! come back; and a picture of the alignments taken in.

use crate::app::{App, Message};

/// A road project with no alignments: `road.alignments.import`'s drawing.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.alignments.import.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn aks() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/road/v1/landxml/aks.xml")
}

#[test]
fn alignments_come_in_from_landxml_and_go_out_again() {
    let mut app = app();
    let _ = app.update(Message::LandxmlAlignments(Some(aks())));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let names: Vec<&str> = doc.road_alignments().iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Ana aks", "Dar kurp", "Kırık aks", "Denklemli aks"]);
    assert_eq!(doc.road_profiles().len(), 1);
    let out = std::env::temp_dir().join(format!("kentos-desktop-landxml-{}.xml", std::process::id()));
    let _ = app.update(Message::LandxmlExport(Some(out.clone())));
    let text = std::fs::read_to_string(&out).expect("written");
    assert_eq!(text.matches("<Alignment ").count(), 4);
    assert!(text.contains("<StaEquation staInternal=\"120\""), "the km equation goes out again");
    let _ = std::fs::remove_file(out);
    assert_eq!(app.document.as_mut().expect("a drawing").model.undo().as_deref(), Some("LandXML aks al"));
}

/// The Güzergâh window on the alignments taken in; `.run/gui-evidence/yol-y10-landxml-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_exchange_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::LandxmlAlignments(Some(aks())));
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("alignment.editor"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("yol-y10-landxml-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
