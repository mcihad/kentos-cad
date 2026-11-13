//! Proje ayarları › Birim ve ölçek › Araç katmanları (usage review UR-003): a CAD project's dimensions, texts and
//! hatches go to the layers chosen here; Etkin katman leaves a tool on the active layer; the section's reset gives
//! back a new CAD project's named layers. Only a CAD project shows the group. `tool_layer_screens` draws it:
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib tool_layer_settings_tests::tool_layer_screens -- --ignored --nocapture
//! ```

use iced::{Point, Size};
use kentos_commands::{AnnotationTool, ToolLayer};
use kentos_ui::snapshot::{Input, Snapshot};

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, find_texts, make_cad, offscreen};
use crate::project::settings::Section;
use crate::project::{SettingsEvent, settings_message};

fn send(app: &mut App, e: SettingsEvent) {
    let _ = app.update(settings_message(e));
}

fn update(app: &mut App, m: Message) {
    let _ = app.update(m);
}

fn units(app: &mut App) {
    let _ = app.update(Message::Run("file.settings"));
    send(app, SettingsEvent::Section(Section::Units));
}

fn saved(app: &App) -> Vec<ToolLayer> {
    app.document.as_ref().expect("a drawing").settings().tool_layers.clone()
}

#[test]
fn a_tools_layer_is_chosen_saved_and_reset() {
    let mut app = app_with_drawing();
    make_cad(&mut app);
    units(&mut app);
    send(&mut app, SettingsEvent::ToolLayer(AnnotationTool::Text, Some("bina".into())));
    send(&mut app, SettingsEvent::ToolLayer(AnnotationTool::Dimension, Some("cizim".into())));
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_none(), "Kaydet closes the window");
    let both = vec![
        ToolLayer { tool: AnnotationTool::Dimension, layer: "cizim".into() },
        ToolLayer { tool: AnnotationTool::Text, layer: "bina".into() },
    ];
    assert_eq!(saved(&app), both, "in the tools' order");

    // Etkin katman takes the tool's entry away.
    units(&mut app);
    send(&mut app, SettingsEvent::ToolLayer(AnnotationTool::Dimension, None));
    send(&mut app, SettingsEvent::Save);
    assert_eq!(saved(&app), both[1..].to_vec());

    // The section's reset: a new CAD project's named layers.
    units(&mut app);
    send(&mut app, SettingsEvent::ResetSection);
    send(&mut app, SettingsEvent::Save);
    assert_eq!(saved(&app), kentos_project::new_project::cad_tool_layers());
}

#[test]
fn only_a_cad_project_shows_the_tools_layers() {
    let size = Size::new(1280.0, 4000.0);
    for cad in [false, true] {
        let mut app = app_with_drawing();
        if cad {
            make_cad(&mut app);
        }
        units(&mut app);
        let mut snapshot = offscreen(size);
        snapshot.settle(&mut app, App::view, &mut update);
        let found = !find_texts(&mut snapshot, &app, "Araç katmanları").is_empty();
        assert_eq!(found, cad, "cad: {cad}");
    }
}

/// The group with a text's layer chosen, in both themes (`.run/gui-evidence/kullanim-UR-003/`).
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn tool_layer_screens() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-003");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        make_cad(&mut app);
        units(&mut app);
        send(&mut app, SettingsEvent::ToolLayer(AnnotationTool::Text, Some("bina".into())));
        let size = Size::new(1280.0, 800.0);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(Point::new(760.0, 420.0), -60.0));
        let file = out.join(format!("arac-katmanlari-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
