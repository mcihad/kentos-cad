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

/// `file.export.landxml`'s drawing, or a case's of it (docs/adr/1031).
fn exported_as(case: Option<&str>) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/file.export.landxml.json")).expect("cases");
    let setup = match case {
        Some(name) => cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).expect("the case")["setup"].clone(),
        None => cases["setup"].clone(),
    };
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// ROAD-SEC-10 (docs/adr/1031): Makine kontrolüne ver writes the file saying the project's systems, and says what the
/// heights are; from a project that does not say its heights' system it writes nothing and says why, while LandXML ver
/// still writes the same drawing for exchange.
#[test]
fn machine_control_needs_the_projects_heights() {
    let said = |app: &App| app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    let mut app = exported_as(None);
    let out = std::env::temp_dir().join(format!("kentos-desktop-machine-{}.xml", std::process::id()));
    let _ = app.update(Message::LandxmlMachine(Some(out.clone())));
    let text = std::fs::read_to_string(&out).expect("written");
    assert!(text.contains("<CoordinateSystem name=\"TUREF / TM30\" epsgCode=\"5254\""), "the horizontal system");
    assert!(text.contains("verticalCoordinateSystemName=\"Antalya height\""), "the heights' system");
    assert!(said(&app).contains("Makine kontrolü dosyası yazıldı") && said(&app).contains("Antalya height (EPSG:5775)"));
    let _ = std::fs::remove_file(&out);
    let mut app = exported_as(Some("Makine kontrolü: projenin kotları söylenmemiş"));
    let _ = app.update(Message::LandxmlMachine(Some(out.clone())));
    assert!(!out.exists(), "nothing written");
    assert!(said(&app).contains("Proje ayarları › Kotlar"), "{}", said(&app));
    let _ = app.update(Message::LandxmlExport(Some(out.clone())));
    assert!(out.exists(), "LandXML ver writes it for exchange");
    let _ = std::fs::remove_file(&out);
}

/// Koridor › Aplikasyon with Makine kontrolüne ver, after it refused a project without its heights' system, light and
/// dark; `.run/gui-evidence/yol-y96/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib road_exchange_tests::screens_machine -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_machine() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y96");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 650.0), "koyu")] {
        for (case, step) in [(Some("Makine kontrolü: projenin kotları söylenmemiş"), "ret"), (None, "yazildi")] {
            let mut app = exported_as(case);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("corridor"));
            let file = std::env::temp_dir().join(format!("kentos-y96-{step}-{name}-{}.xml", std::process::id()));
            let _ = app.update(Message::LandxmlMachine(Some(file.clone())));
            let _ = std::fs::remove_file(&file);
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let shot = out.join(format!("y96-makine-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&shot).expect("writes the picture");
            println!("{}", shot.display());
        }
    }
}
