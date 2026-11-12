//! Kodla çiz (features_window.rs, docs/adr/1360): the shared cases' drawing of coded survey points opens the window
//! with Harita varlıkları; its preview names the classes; Uygula puts the points on their layers and draws the kerbs
//! and the building in one undo step; a class without its layer offers Profili uygula, which adds it; KentOS Harita
//! and KentOS GIS offer the command, Mimari and Yol do not. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::features_window::Event as F;

fn snapshot(missing_layers: bool) -> kentos_commands::DocumentSnapshotV2 {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.features.code.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    if missing_layers {
        // The profile's group taken away: the classes have no layers yet.
        let layers = setup["layers"].as_array_mut().expect("layers");
        layers.retain(|l| l["id"] != "hv");
        for e in setup["entities"].as_array_mut().expect("objects") {
            if e["layerId"] == "hv-agc" {
                e["layerId"] = Value::from("olcum");
            }
        }
    }
    serde_json::from_value(setup).expect("the drawing reads")
}

fn opened(snapshot: kentos_commands::DocumentSnapshotV2, program: Option<Module>) -> App {
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn kinds(app: &App) -> Vec<&'static str> {
    app.document.as_ref().expect("a drawing").model.entities().map(kentos_commands::Entity::kind).collect()
}

#[test]
fn coded_points_go_on_their_classes_and_strings_become_lines_and_areas() {
    let mut app = opened(snapshot(false), Some(Module::Map));
    assert!(app.offers("map.featuresCode"));
    let _ = app.update(Message::Run("map.featuresCode"));
    assert_eq!(app.dialog, Some(Dialog::FeaturesCode));
    assert!(app.features_code.profiles.iter().any(|(n, _)| n == "harita-varliklari"));
    let (plan, warnings) = app.features_code.plan.clone().expect("a plan").expect("previewed");
    let titles: Vec<&str> = plan.classes.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["Kontrol noktası", "Rögar", "Ağaç", "Bordür", "Bina izi"]);
    assert!(warnings.iter().any(|w| w.contains("“XYZ”")), "{warnings:?}");
    let before = kinds(&app).len();
    let _ = app.update(Message::FeaturesCode(F::Apply));
    assert_eq!(app.dialog, None);
    let after = kinds(&app);
    assert_eq!(after.len(), before + 3, "two kerbs and a building");
    assert_eq!(after.iter().filter(|k| **k == "polyline").count(), 2);
    assert_eq!(after.iter().filter(|k| **k == "polygon").count(), 1);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.undo_labels().first().map(String::as_str), Some("Kodla çiz"));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(kinds(&app).len(), before, "one step");
}

#[test]
fn a_class_without_its_layer_offers_the_profile_first() {
    let mut app = opened(snapshot(true), Some(Module::Map));
    let _ = app.update(Message::Run("map.featuresCode"));
    match app.features_code.plan.clone().expect("a plan") {
        Err((code, _)) => assert_eq!(code, "class_layer_missing"),
        Ok(_) => panic!("refused without the class's layer"),
    }
    let _ = app.update(Message::FeaturesCode(F::ApplyProfile));
    assert!(matches!(app.features_code.plan, Some(Ok(_))), "the layers added, the preview holds");
}

#[test]
fn harita_and_gis_offer_it_mimari_and_yol_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Architecture, false), (Module::Road, false)] {
        assert_eq!(opened(snapshot(false), Some(module)).offers("map.featuresCode"), yes, "{module:?}");
    }
}

/// The window's preview and the drawing after Uygula, light and wide, dark and narrow;
/// `.run/gui-evidence/kodla-ciz-1360/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib features_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kodla-ciz-1360");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened(snapshot(false), Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        let _ = app.update(Message::Run("map.featuresCode"));
        shot(&mut app, "1-onizleme");
        let _ = app.update(Message::FeaturesCode(F::Apply));
        shot(&mut app, "2-cizildi");
    }
}
