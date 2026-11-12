//! Kataloğa taşı in Katalog geçiş raporu (catalog_report_window.rs; plan/map-cad.md MAP-04, docs/adr/5340) on the
//! report's shared drawing carrying the institution's new catalog: the button carries the objects as the shared case of
//! `cad.profiles.carry` does (its counts), the report then says the classes' objects are on their new layers, and one
//! undo puts everything back. And the pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};
use crate::catalog_report_window::Event as C;
use crate::catalog_report_window_tests::opened;

fn shared() -> Value {
    let all: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.profiles.carry.json")).expect("cases");
    all["cases"][0]["steps"][2]["result"]["output"].clone()
}

fn layer_of(app: &App, id: u32) -> String {
    app.document.as_ref().expect("a drawing").model.entities().find(|e| e.base().id == id).expect("the object").base().layer_id.clone()
}

#[test]
fn kataloga_tasi_carries_as_the_command_and_undoes_in_one_step() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.catalogReport"));
    let before = layer_of(&app, 1);
    let _ = app.update(Message::CatalogReport(C::Carry));
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    let moved = shared()["moved"].as_u64().expect("moved");
    assert!(said.iter().any(|l| l.contains(&format!("{moved} nesne yeni kataloğun katmanlarına taşındı"))), "{said:?}");
    assert!(said.iter().any(|l| l.contains("Yerinde kalan nesneler")), "{said:?}");
    assert_ne!(layer_of(&app, 1), before, "the control point went to its new class's layer");
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(doc.model.undo().as_deref(), Some("Kataloğa taşı"));
    assert_eq!(layer_of(&app, 1), before);
}

/// `.run/gui-evidence/katalog-tasi-5340/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib catalog_carry_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/katalog-tasi-5340");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, w, h, name) in [("light", 1440.0, 900.0, "acik-genis"), ("dark", 1280.0, 800.0, "koyu-dar")] {
        for (shot, carry) in [("once", false), ("sonra", true)] {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::Run("map.catalogReport"));
            if carry {
                let _ = app.update(Message::CatalogReport(C::Carry));
            }
            app.follow.flash = None;
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{shot}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
