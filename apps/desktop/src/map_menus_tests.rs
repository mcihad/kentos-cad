//! Where MAP-06…11's windows sit in the menus (plan/map-cad.md; the owner asks for the menus' pictures with the new
//! icons): Harita's Plan tab (Ada ve parsel, İmar planı), the area's context tab (Köşe: Eğimle kot ver), GIS's Veri
//! and Analiz tabs. Pictures only.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};

fn opened(program: Option<Module>) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.zoning.check.json")).expect("cases");
    let mut setup = cases["cases"][1]["setup"].clone();
    if program == Some(Module::Gis) {
        setup["settings"]["workspace"] = Value::from("gis");
        if let Some(m) = setup["settings"].as_object_mut() {
            m.remove("cadWork");
        }
    }
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// `.run/gui-evidence/menuler-map-06-11/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_menus_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/menuler-map-06-11");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1920.0, 1000.0);
    for (module, tab, select, name) in [
        (Module::Map, Some("plan"), false, "1-harita-plan"),
        (Module::Map, None, true, "2-harita-alan-baglam"),
        (Module::Gis, Some("data"), false, "3-cbs-veri"),
        (Module::Gis, Some("analysis"), false, "4-cbs-analiz"),
    ] {
        let mut app = opened(Some(module));
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        if select {
            app.selection.set([Slot(5)]);
            app.follow_selection_tab();
        }
        if let Some(tab) = tab {
            let _ = app.update(Message::RibbonTab(tab));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
