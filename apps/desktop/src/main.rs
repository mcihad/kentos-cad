//! KentOS CAD desktop (docs/adr/0010, 0017): native Iced with the KentOS UI
//! components, the shared Rust contracts and KentOS's own wgpu drawing area
//! (docs/adr/0019).
//! `kentos-cad [çizim.kcad]` opens the drawing at once; `kentos-cad snapshot
//! çıktı.png [çizim.kcad]` draws the window into an image without opening it.

mod annotation_scale;
mod annotation_styles;
mod app;
mod app_menu;
mod appearance;
#[cfg(test)]
mod appearance_tokens_tests;
mod attribute_table;
mod attribute_values;
mod block_attributes;
mod blocks;
mod blocks_panel;
mod bottom;
mod calc;
mod catalog;
#[cfg(test)]
mod centerline_scenes;
mod clipboard;
mod cloud;
mod cogo;
mod command_bar;
#[cfg(test)]
mod coordinate_scenes;
mod crs;
mod data_compare;
#[cfg(test)]
mod dimension_scenes;
mod document;
mod drawing_exchange;
mod drawing_fonts;
mod drawing_menus;
#[cfg(test)]
mod edge_shift_scenes;
#[cfg(test)]
mod elevation_scenes;
#[cfg(test)]
mod elevation_tests;
mod exchange;
mod expression;
mod features;
#[cfg(test)]
mod files_testing;
mod find_replace;
mod grids;
#[cfg(test)]
mod hatch_scenes;
mod hover_card;
#[cfg(test)]
mod icon_tour;
mod icons;
#[cfg(test)]
mod image_scenes;
mod input;
mod keys;
mod keytips;
mod labels;
mod layer_fields;
mod layer_list;
mod layer_merge;
mod layer_purge;
mod layer_snap;
mod layer_states;
mod layer_tree;
mod layering;
mod layout;
mod layout_plan;
#[cfg(test)]
mod layout_plan_tests;
#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod leader_scenes;
mod locks;
mod log_plan;
#[cfg(test)]
mod log_plan_tests;
mod map_marks;
mod map_vectors;
mod marks;
mod message_log;
#[cfg(test)]
mod message_log_tests;
mod modes;
mod navigation;
mod navigation_cards;
#[cfg(test)]
mod netcad_names_tests;
mod opening;
#[cfg(test)]
mod overlap_tests;
mod paragraph_editor;
#[cfg(test)]
mod parts_scenes;
#[cfg(test)]
mod perf;
mod pictures;
mod point_calc;
#[cfg(test)]
mod point_calc_scenes;
mod points;
mod preview;
mod processing;
mod project;
mod properties;
mod python;
#[cfg(test)]
mod query_tests;
#[cfg(test)]
mod raster_scenes;
mod rasters;
mod recent;
mod recovery;
mod ribbon_bar;
mod ribbon_keys;
mod ribbon_panels;
mod ribbon_plan;
mod ribbon_search;
#[cfg(test)]
mod ribbon_tests;
mod saving;
mod screen_scale;
#[cfg(test)]
mod screens;
mod search;
mod second_crs;
mod selecting;
mod selection_chip;
mod selection_commands;
#[cfg(test)]
mod selection_scenes;
mod settings;
mod settings_look;
#[cfg(test)]
mod settings_look_tests;
mod settings_sections;
mod settings_view;
mod sheet_inputs;
mod sheet_library;
mod sheet_pdf;
mod sheets;
mod shortcuts;
mod snap_menu;
#[cfg(test)]
mod snap_tests;
mod snapshot;
mod sources;
mod start;
#[cfg(test)]
mod stationing_scenes;
mod style;
#[cfg(test)]
mod style_scenes;
#[cfg(test)]
mod table_scenes;
mod tables;
mod template_editor;
mod template_members;
mod templates;
mod templates_panel;
mod text_field;
mod text_file;
#[cfg(test)]
mod text_scenes;
#[cfg(test)]
mod tools_scenes;
#[cfg(test)]
mod tools_screens;
mod topology;
#[cfg(test)]
mod topology_scenes;
#[cfg(test)]
mod topology_tests;
mod traces;
mod tracking;
#[cfg(test)]
mod ui_screens;
mod usage;
mod vertices;
mod view;
mod view_commands;
mod viewport;

use kentos_ui::theme::typography;

fn main() -> iced::Result {
    // Embedded faces: nothing has to be installed on the machine.
    typography::load();

    // `kentos-cad snapshot çıktı.png [çizim.kcad]`: the window without a window.
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().map(String::as_str) == Some("snapshot") {
        args.next();
        if let Err(error) = snapshot::run(args) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    // `kentos-cad kullan <iz>`: a usage scenario played and pictured step by step.
    if args.peek().map(String::as_str) == Some("kullan") {
        args.next();
        if let Err(error) = usage::run(args) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }

    let path = std::env::args_os().nth(1).map(std::path::PathBuf::from);

    iced::application(
        // The user's settings file (docs/adr/0023); in memory where no configuration folder is known.
        move || {
            let settings = settings::Settings::config_dir()
                .map_or_else(settings::Settings::memory, |dir| {
                    settings::Settings::open(&dir, std::time::SystemTime::now())
                });
            // Recovery copies of unsaved work (docs/adr/0030): none when the data folder cannot be used.
            let recovery =
                match recovery::default_root().map(|root| recovery::Recovery::open(&root)) {
                    Some(Ok(recovery)) => recovery,
                    Some(Err(error)) => recovery::Recovery::unavailable(error),
                    None => recovery::Recovery::off(),
                };
            let (mut app, task) = app::App::start_with(path.clone(), settings, recovery);
            // Device drafts of cloud projects (docs/adr/0040, 0041), next to the recovery copies.
            app.cloud.drafts = cloud::default_drafts();
            // Local copies of cloud projects: they open without a connection (docs/adr/0043).
            app.cloud.replicas = cloud::default_replicas();
            // The sheets' books, by the project's key, beside them (docs/sheet/design.md §10).
            app.sheet_store = sheets::default_store();
            // The device's NTv2 grids for the projects' datum choices (docs/adr/0168 §4).
            app.grids.library = grids::Library::device();
            // The recent files, kept beside the program's other history.
            if let Some(folder) = recent::RecentFiles::default_folder() {
                app.recent = recent::RecentFiles::open(&folder);
                // Kaynaklar's folders, beside them (docs/adr/0199 §7).
                app.sources.folders = sources::SourceFolders::open(&folder);
                // The Python tab's unsaved script, kept beside them (docs/adr/0136).
                app.python.script = python::script::Script::load(&folder);
                // Each processing tool's last values (islemler.json), beside them.
                app.processing.memory = processing::memory::Memory::open(&folder);
                // The user's models (the designer's Kaydet) beside the built-in ones.
                let models = app.processing.memory.models().to_vec();
                app.processing.registry.set_user_models(models);
            }
            // Kitaplığım, the user's own symbols (kitaplik.kstil, docs/adr/0092).
            if let Some(folder) = style::user_library::default_folder()
                && let Some(problem) = app.styles.open_user_library(&folder)
            {
                app.warn(problem);
            }
            // The layout kept last time (yerlesim.json beside the settings, docs/adr/0115).
            if let Some(dir) = settings::Settings::config_dir() {
                app.layout = layout::Keeper::open(&dir);
                app.apply_layout();
            }
            // Without a drawing named, the start screen (when the preference wants it).
            if path.is_none() {
                app.start_at_launch();
            }
            // The server cell's first answer (the web asks once it is idle after start).
            let server = app.check_server_quietly();
            (app, iced::Task::batch([task, server]))
        },
        app::App::update,
        app::App::view,
    )
    .title(app::App::title)
    .theme(app::App::theme)
    .default_font(typography::ui())
    .subscription(app::App::subscription)
    .window(iced::window::Settings {
        size: iced::Size::new(1440.0, 900.0),
        // The smallest window the layouts are checked at: the ribbon fits (docs/adr/0051),
        // the windows keep their buttons in view.
        min_size: Some(iced::Size::new(1100.0, 650.0)),
        // A drawing with unsaved changes asks before the window closes.
        exit_on_close_request: false,
        ..iced::window::Settings::default()
    })
    .antialiasing(true)
    .run()
}
