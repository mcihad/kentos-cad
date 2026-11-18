//! 3B görünüm kalitesi (docs/adr/8120; Uygulama ayarları › Çizim motoru › 3B görünüm): the setting's three levels reach
//! the 3D views (the building's viewer, IFC modeli, the road's scene), Kaliteli by default. `screens` draws the section
//! building at each level and the setting in its section, for the owner (`.run/gui-evidence/3b-kalite-8120/`):
//!
//! ```text
//! KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_quality_tests::screens -- --ignored --nocapture
//! ```

use kentos_model_view_ui::scene::Quality;
use serde_json::Value;

use crate::app::{App, Message};
use crate::scene3d_building_tests::{building, open};

fn choose(app: &mut App, value: &str) {
    let _ = app.settings.choose(&[("graphics.scene3dQuality", Value::from(value))]);
    app.apply_settings();
}

#[test]
fn the_quality_follows_the_setting_kaliteli_by_default() {
    let (mut app, _) = App::boot(None);
    assert_eq!(app.scene3d_quality(), Quality::High, "Kaliteli unless asked");
    for (value, quality) in [("fast", Quality::Fast), ("balanced", Quality::Balanced), ("high", Quality::High)] {
        choose(&mut app, value);
        assert_eq!(app.scene3d_quality(), quality, "{value}");
    }
    // The schema offers the three by their Turkish names.
    let d = crate::settings::schema().get("graphics.scene3dQuality").expect("the setting");
    let names: Vec<&str> = d.choices.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(names, ["Hızlı", "Dengeli", "Kaliteli"]);
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/3b-kalite-8120");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    for (theme, size, tag) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        for (value, name) in [("fast", "hizli"), ("balanced", "dengeli"), ("high", "kaliteli")] {
            let mut app = open(building());
            let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
            choose(&mut app, value);
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("scene3d.view"));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::scene3d_shown, &mut update);
            snapshot.settle(&mut app, App::scene3d_shown, &mut update);
            let file = out.join(format!("bina-{name}-{tag}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
    // The setting in Uygulama ayarları › Çizim motoru.
    let size = Size::new(1440.0, 900.0);
    let mut app = open(building());
    let _ = app.settings.choose(&[("appearance.theme", Value::from("light"))]);
    app.apply_settings();
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let _ = app.update(Message::WindowResized(size));
    let _ = app.run("tools.options");
    let _ = app.update(Message::Settings(crate::settings_view::Edit::Section(crate::settings_sections::Section::Engine)));
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("ayar-3b-kalite-acik.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
    println!("{}", file.display());
}
