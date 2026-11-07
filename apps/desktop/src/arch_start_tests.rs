//! KentOS Mimari's start (docs/adr/3007): its wall tools begin on Dış duvar 25, a new project opens on Mimari on its
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
