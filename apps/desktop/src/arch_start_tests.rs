//! KentOS Mimari's start (docs/adr/8007): its wall tools begin on Dış duvar 25, a new project opens on Mimari on its
//! ground floor, and the first wall is written with its style and level, poşeli at 1:100.

use kentos_launch::Module;

use super::FIRST_WALL_STYLE;
use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

/// A program as main.rs starts it (none: kentos-cad), its settings and connections in memory.
fn started(module: Option<Module>) -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(module, None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    app
}

/// The wall written last.
fn newest_wall(app: &App) -> kentos_commands::WallFields {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Wall(w) => Some(w),
            _ => None,
        })
        .max_by_key(|w| w.base.id)
        .expect("a wall")
        .wall
        .clone()
}

#[test]
fn kentos_mimari_s_wall_tools_start_on_dis_duvar_25_and_the_other_programs_on_yok() {
    let mimari = started(Some(Module::Architecture));
    assert_eq!(
        mimari.memory.wall_style,
        Some(kentos_interaction::wall::style_key(FIRST_WALL_STYLE))
    );
    assert_eq!(mimari.memory.wall_width, 0.25, "the style's thickness");
    for other in [None, Some(Module::Map), Some(Module::Gis)] {
        let app = started(other);
        assert_eq!(app.memory.wall_style, None, "{other:?}: Yok");
        assert_eq!(app.memory.wall_width, kentos_commands::DEFAULT_WALL_WIDTH);
    }
}

#[test]
fn a_new_mimari_project_opens_on_mimari_on_zemin_kat_and_its_first_wall_is_hatched() {
    let mut app = started(Some(Module::Architecture));
    // A layout kept on Giriş: the new project still opens where Duvar is.
    app.tab = "home";
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    assert_eq!(app.regular_tab(), "arch", "Mimari");
    let settings = app
        .document
        .as_ref()
        .expect("a new project")
        .settings()
        .clone();
    let levels = &settings
        .arch
        .as_ref()
        .expect("a building's settings")
        .levels;
    assert_eq!(levels, &vec![kentos_commands::ground_floor()]);
    assert_eq!(
        app.active_level().as_deref(),
        Some("zemin"),
        "Etkin kat: Zemin kat"
    );
    // Dikdörtgen duvar and its two corners: the first wall, nothing chosen before.
    let _ = app.update(Message::Run("tool.wallRectangle"));
    let _ = app.submit_line("0,0");
    let _ = app.submit_line("5,4");
    let wall = newest_wall(&app);
    assert_eq!(wall.style.as_deref(), Some(FIRST_WALL_STYLE));
    assert_eq!(wall.arch.level.as_deref(), Some("zemin"));
    assert_eq!(wall.width, 0.25);
    // The project took KentOS's style with the wall: its fill is the poşe a 1:100 plan draws.
    let doc = app.document.as_ref().unwrap();
    let style = doc
        .settings()
        .wall_styles
        .iter()
        .find(|s| s.id == FIRST_WALL_STYLE)
        .expect("taken into the project");
    assert!(style.fill.is_some(), "poşe");
    assert_eq!(doc.settings().plot_scale, 100.0);
}

#[test]
fn a_file_opened_and_the_other_programs_keep_the_tab() {
    // kentos-cad's new project keeps the tab it had (its wizard is another story: docs/adr/0802).
    let mut app = started(None);
    app.tab = "home";
    app.open_new_project_on_program_tab();
    assert_eq!(app.tab, "home");
}

/// The layer id of the wall written last.
fn newest_wall_layer(app: &App) -> String {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Wall(w) => Some(w),
            _ => None,
        })
        .max_by_key(|w| w.base.id)
        .expect("a wall")
        .base
        .layer_id
        .clone()
}

/// A new Mimari project, Dikdörtgen duvar running.
fn rectangle_tool() -> App {
    let mut app = started(Some(Module::Architecture));
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    let _ = app.update(Message::Run("tool.wallRectangle"));
    app
}

#[test]
fn kentos_mimari_writes_the_first_wall_on_its_ek4_layer_and_one_undo_takes_both() {
    // docs/adr/8008: in the 2+1 flat all 54 objects were on Çizim.
    let mut app = rectangle_tool();
    assert!(app.memory.layer_keys, "KentOS Mimari keys its layers");
    assert_eq!(
        app.session.layer_shown(),
        Some("MM-DUVA-TUGLA"),
        "the panel's Katman"
    );
    let doc = &app.document.as_ref().unwrap().model;
    assert!(
        doc.layers().get("mm-duva-tugla").is_none(),
        "the layer opens with the first wall"
    );
    let _ = app.submit_line("0,0");
    let _ = app.submit_line("5,4");
    assert_eq!(newest_wall_layer(&app), "mm-duva-tugla");
    let doc = &app.document.as_ref().unwrap().model;
    let layer = doc.layers().get("mm-duva-tugla").expect("opened");
    assert_eq!(layer.name, "MM-DUVA-TUGLA");
    assert!(kentos_commands::is_ek4_layer(&layer.name));
    // One undo: the wall and the layer it opened.
    let _ = app.update(Message::Run("edit.undo"));
    let doc = &app.document.as_ref().unwrap().model;
    assert!(
        !doc.entities()
            .any(|e| matches!(e, kentos_commands::Entity::Wall(_)))
    );
    assert!(
        doc.layers().get("mm-duva-tugla").is_none(),
        "the layer went with its wall"
    );
}

#[test]
fn katman_set_to_etkin_katman_writes_on_the_active_layer() {
    let mut app = rectangle_tool();
    let _ = app.update(Message::LayerKeys(false));
    assert!(!app.memory.layer_keys);
    assert_eq!(
        app.session.layer_shown(),
        Some("Çizim"),
        "the active layer's name"
    );
    let _ = app.submit_line("0,0");
    let _ = app.submit_line("5,4");
    let active = app
        .document
        .as_ref()
        .unwrap()
        .model
        .layers()
        .active()
        .to_owned();
    assert_eq!(newest_wall_layer(&app), active);
    // Back on: the next wall goes on its kind's layer.
    let _ = app.update(Message::LayerKeys(true));
    assert_eq!(app.session.layer_shown(), Some("MM-DUVA-TUGLA"));
}

#[test]
fn kentos_cad_keeps_the_active_layer() {
    let app = started(None);
    assert!(!app.memory.layer_keys, "the key is KentOS Mimari's");
}
