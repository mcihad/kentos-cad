//! KentOS CAD desktop (docs/adr/0010, 0017): native Iced with the KentOS UI
//! components, the shared Rust contracts and KentOS's own wgpu drawing area
//! (docs/adr/0019).
//! `kentos-cad [çizim.kcad]` opens the drawing at once; `kentos-cad snapshot
//! çıktı.png [çizim.kcad]` draws the window into an image without opening it.

#[cfg(test)]
mod acceptance_tests;
#[cfg(test)]
mod annotative_sheet_tests;
#[cfg(test)]
mod text_outputs_tests;
mod address;
#[cfg(test)]
mod address_scenes;
mod aids;
mod alias_profile;
#[cfg(test)]
mod alias_profile_scenes;
mod annotation_scale;
mod annotative;
mod annotation_styles;
mod app;
mod app_menu;
mod appearance;
#[cfg(test)]
mod appearance_tokens_tests;
#[cfg(test)]
mod assoc_array_scenes;
#[cfg(test)]
mod attachment_scenes;
mod attachments;
#[cfg(test)]
mod attachments_tests;
mod attribute_table;
mod attribute_values;
#[cfg(test)]
mod audit_scenes;
#[cfg(test)]
mod block_attr_scenes;
mod block_attributes;
mod block_card;
#[cfg(test)]
mod block_dynamic_scenes;
mod block_editor;
#[cfg(test)]
mod block_scenes;
mod block_schedule;
mod blocks;
mod blocks_files;
mod blocks_panel;
mod bottom;
mod calc;
mod catalog;
#[cfg(test)]
mod centerline_scenes;
#[cfg(test)]
mod chart_scenes;
mod charts;
#[cfg(test)]
mod cleanup_scenes;
mod clipboard;
#[cfg(test)]
mod clipboard_scenes;
mod cloud;
#[cfg(test)]
mod cloud_scenes;
mod cogo;
mod command_bar;
#[cfg(test)]
mod coordinate_scenes;
mod crs;
#[cfg(test)]
mod curve_edit_scenes;
#[cfg(test)]
mod curve_scenes;
mod data_compare;
#[cfg(test)]
mod dimension_follow_scenes;
#[cfg(test)]
mod dimension_mark_scenes;
#[cfg(test)]
mod annotative_scenes;
#[cfg(test)]
mod paragraph_layout_scenes;
#[cfg(test)]
mod text_field_scenes;
#[cfg(test)]
mod multileader_scenes;
#[cfg(test)]
mod table_cells_scenes;
#[cfg(test)]
mod dimension_option_scenes;
#[cfg(test)]
mod dimension_scenes;
#[cfg(test)]
mod divide_scenes;
mod document;
mod draw_order;
mod drawing_exchange;
mod drawing_fonts;
mod drawing_menus;
mod drawing_standard;
#[cfg(test)]
mod edge_shift_scenes;
#[cfg(test)]
mod elevation_scenes;
#[cfg(test)]
mod elevation_tests;
mod exchange;
#[cfg(test)]
mod explode_scenes;
mod expression;
mod features;
#[cfg(test)]
mod files_testing;
mod find_replace;
mod grids;
#[cfg(test)]
mod hatch_scenes;
#[cfg(test)]
mod hatch_boundary_scenes;
mod history;
#[cfg(test)]
mod history_scenes;
mod hover_card;
#[cfg(test)]
mod icon_tour;
mod icons;
#[cfg(test)]
mod image_scenes;
#[cfg(test)]
mod impact_scenes;
#[cfg(test)]
mod imperial_scenes;
mod info_card;
mod input;
#[cfg(test)]
mod input_scenes;
mod keys;
mod keytips;
mod label_picture;
#[cfg(test)]
mod label_scenes;
mod labelling;
mod labels;
mod layer_commands;
mod layer_fields;
mod layer_filters;
mod layer_info;
mod layer_list;
mod font_report;
mod text_tools;
mod leader_tools;
mod layer_merge;
mod layer_purge;
mod layer_snap;
mod layer_states;
mod layer_translate;
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
#[cfg(test)]
mod lengthen_scenes;
#[cfg(test)]
mod line_type_scenes;
mod line_types_window;
mod locks;
mod log_plan;
#[cfg(test)]
mod log_plan_tests;
#[cfg(test)]
mod look_scenes;
mod map_marks;
mod marks;
#[cfg(test)]
mod match_scenes;
mod message_log;
#[cfg(test)]
mod message_log_tests;
#[cfg(test)]
mod method_scenes;
#[cfg(test)]
mod mirror_scenes;
mod modes;
#[cfg(test)]
mod multipart_scenes;
mod navigation;
mod navigation_cards;
#[cfg(test)]
mod netcad_names_tests;
#[cfg(test)]
mod nurbs_scenes;
mod object_info;
#[cfg(test)]
mod offset_scenes;
mod opening;
#[cfg(test)]
mod overlap_tests;
mod paragraph_editor;
#[cfg(test)]
mod parts_scenes;
#[cfg(test)]
mod perf;
mod photo_view;
mod pictures;
mod point_calc;
#[cfg(test)]
mod point_calc_scenes;
mod pointclouds;
mod points;
#[cfg(test)]
mod polyline_scenes;
mod preview;
mod processing;
mod project;
mod properties;
mod python;
#[cfg(test)]
mod query_tests;
#[cfg(test)]
mod range_server;
#[cfg(test)]
mod raster_scenes;
mod rasters;
mod recent;
mod recovery;
#[cfg(test)]
mod renderer_scenes;
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
#[cfg(test)]
mod segment_scenes;
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
#[cfg(test)]
mod standard_scenes;
#[cfg(test)]
mod stretch_scenes;
mod style_impact;
#[cfg(test)]
mod style_override_scenes;
mod sub_object;
#[cfg(test)]
mod subref_scenes;
#[cfg(test)]
mod transform_scenes;
mod typed_names;
mod ucs;
#[cfg(test)]
mod ucs_scenes;
// Map services (docs/adr/0208): their tiles, connections' secrets and the device's cache.
#[cfg(test)]
mod distance_scenes;
#[cfg(test)]
mod hydrology_scenes;
#[cfg(test)]
mod interpolation_scenes;
#[cfg(test)]
mod layer_style_scenes;
#[cfg(test)]
mod multidim_scenes;
mod networks;
#[cfg(test)]
mod raster_ops_scenes;
#[cfg(test)]
mod raster_vector_scenes;
#[cfg(test)]
mod remote_scenes;
#[cfg(test)]
mod service_scenes;
mod services;
mod sheet_book;
mod sheet_library;
mod sheet_pdf;
mod sheets;
mod shortcuts;
#[cfg(test)]
mod snap_commit_tests;
mod snap_cycle;
mod snap_menu;
#[cfg(test)]
mod snap_tests;
mod snapshot;
mod sources;
mod split_view;
mod start;
#[cfg(test)]
mod stationing_scenes;
mod statistics;
#[cfg(test)]
mod stats_scenes;
mod style;
#[cfg(test)]
mod style_scenes;
#[cfg(test)]
mod suitability_scenes;
#[cfg(test)]
mod surface_scenes;
#[cfg(test)]
mod table_scenes;
mod table_write;
mod tables;
mod template_editor;
mod template_members;
mod templates;
mod templates_panel;
mod temporal;
#[cfg(test)]
mod temporal_scenes;
mod text_field;
mod text_file;
#[cfg(test)]
mod text_scenes;
mod toasts;
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
mod transparent_tests;
#[cfg(test)]
mod ui_screens;
mod usage;
mod vertices;
mod view;
mod view_commands;
#[cfg(test)]
mod view_scenes;
mod viewport;
mod views;
#[cfg(test)]
mod window_layers_tests;
#[cfg(test)]
mod wipeout_scenes;

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
    // Today's midnight for Yeni sürüm oluştur and Sona erdir is the device's (docs/adr/0210 §7).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    kentos_interaction::time_version::LOCAL_OFFSET.store(
        cloud::local_time::Zone::system().offset_at(now),
        std::sync::atomic::Ordering::Relaxed,
    );

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
