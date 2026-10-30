//! KentOS CAD's desktop (docs/adr/0010, 0017): native Iced with the KentOS UI
//! components, the shared Rust contracts and KentOS's own wgpu drawing area
//! (docs/adr/0019), as a library its programs run (docs/adr/0801):
//!
//! - `kentos-cad [--modul <id>] [dosya]`, every module at once: a drawing's
//!   ribbon follows its type and CAD work; `--modul mimari|harita|cbs` makes it
//!   that module's program. `kentos-cad snapshot çıktı.png [çizim.kcad]` draws
//!   the window into an image without opening it, `kentos-cad kullan <iz>`
//!   plays a usage scenario and pictures it ([`all_in_one`]);
//! - `kentos-mimari`, `kentos-harita`, `kentos-gis [dosya]`: one module's own
//!   program, the same library fixed to it ([`module_program`]).

#[cfg(test)]
mod acceptance_gis_tests;
#[cfg(test)]
mod acceptance_tests;
#[cfg(test)]
mod measure_probe_tests;
#[cfg(test)]
mod semantic_tool_tests;
#[cfg(test)]
mod annotative_sheet_tests;
#[cfg(test)]
mod dimension_set_sheet_tests;
#[cfg(test)]
mod symbols_sheet_tests;
#[cfg(test)]
mod wall_detail_sheet_tests;
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
mod wall_joins;
mod tags_check;
mod schedule_window;
mod arch_audit;
mod viewmark_refs;
mod sheet_set;
mod xrefs_window;
mod xrefs_watch;
mod arch_project;
mod architecture_settings;
mod architecture_ribbon;
mod architecture_template;
mod quantities;
mod wall_styles;
mod app;
mod app_menu;
mod appearance;
#[cfg(test)]
mod appearance_tokens_tests;
#[cfg(test)]
mod assoc_array_scenes;
mod constraint_relax;
mod constraint_view;
#[cfg(test)]
mod constraint_follow_scenes;
#[cfg(test)]
mod constraint_scenes;
#[cfg(test)]
mod constraint_ui_scenes;
#[cfg(test)]
mod attachment_scenes;
mod attachments;
mod bound;
#[cfg(test)]
mod attachments_tests;
mod attribute_table;
mod attribute_values;
#[cfg(test)]
mod audit_scenes;
mod band_ghosts;
#[cfg(test)]
mod band_scenes;
#[cfg(test)]
mod block_attr_scenes;
mod block_attributes;
mod block_card;
#[cfg(test)]
mod block_dynamic_scenes;
#[cfg(test)]
mod furniture_scenes;
#[cfg(test)]
mod casework_scenes;
#[cfg(test)]
mod temporary_scenes;
#[cfg(test)]
mod arch_audit_scenes;
#[cfg(test)]
mod arch_audit_exception_scenes;
#[cfg(test)]
mod schedule_extras_scenes;
mod block_editor;
#[cfg(test)]
mod block_scenes;
mod block_schedule;
mod blocks;
mod blocks_files;
mod blocks_library;
mod blocks_panel;
mod bottom;
mod calc;
mod catalog;
#[cfg(test)]
mod centerline_scenes;
#[cfg(test)]
mod source_relation_scenes;
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
mod facade_dimension_scenes;
#[cfg(test)]
mod dimension_layout_scenes;
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
mod clip_frames;
mod doc_tabs;
mod document;
mod draw_order;
mod drawing_exchange;
mod drawing_fonts;
mod drawing_menus;
mod data_profile;
mod subnetworks_window;
mod source_status;
mod source_trouble;
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
#[cfg(test)]
mod hatch_pattern_scenes;
#[cfg(test)]
mod hatch_look_scenes;
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
mod label_probe;
#[cfg(test)]
mod inspector_probe;
#[cfg(test)]
mod inspector_screens;
#[cfg(test)]
mod label_scenes;
mod labelling;
mod labels;
mod layer_commands;
mod layer_fields;
mod layer_filters;
mod dataset_window;
mod dialog_sizes;
#[cfg(test)]
mod dialog_size_tests;
#[cfg(test)]
mod constraint_screens;
#[cfg(test)]
mod form_fit_screens;
mod layer_info;
mod layer_list;
mod font_report;
mod text_tools;
mod leader_tools;
mod layer_merge;
mod joins_window;
mod event_links_window;
mod layer_order;
mod layer_purge;
mod source_reconcile;
mod layer_maplibre;
mod layer_qml;
mod layer_sld;
mod layer_style_file;
mod layer_snap;
mod layer_states;
mod layer_translate;
mod layer_tree;
mod layer_types;
#[cfg(test)]
mod layer_type_scenes;
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
mod hatch_patterns_window;
mod line_types_window;
mod parameters_window;
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
mod module_screens;
#[cfg(test)]
mod module_tests;
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
mod program;
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
mod layer_crs;
mod map_crs;
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
mod address_compare_scenes;
#[cfg(test)]
mod data_profile_tests;
#[cfg(test)]
mod profile_scenes;
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
mod sheet_model;
mod shortcuts;
#[cfg(test)]
mod snap_commit_tests;
mod snap_cycle;
mod snap_menu;
#[cfg(test)]
mod snap_tests;
mod snapshot;
mod db_connections;
mod sources;
mod split_revision;
#[cfg(test)]
mod split_scenes;
#[cfg(test)]
mod split_many_tests;
#[cfg(test)]
mod split_tests;
mod split_view;
#[cfg(test)]
mod split_wall_tests;
mod start;
#[cfg(test)]
mod stationing_scenes;
mod statistics;
#[cfg(test)]
mod stats_scenes;
mod style;
#[cfg(test)]
mod style_exchange_scenes;
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
mod map_crs_tests;
#[cfg(test)]
mod ui_screens;
#[cfg(test)]
mod ux_screens;
mod usage;
mod vertices;
mod view;
mod view_commands;
#[cfg(test)]
mod view_scenes;
mod viewport;
mod views;
mod forms;
#[cfg(test)]
mod form_scenes;
#[cfg(test)]
mod form_access_scenes;
#[cfg(test)]
mod window_layers_tests;
#[cfg(test)]
mod wipeout_scenes;
#[cfg(test)]
mod wall_scenes;
#[cfg(test)]
mod interior_scenes;
mod active_level;
#[cfg(test)]
mod column_scenes;
#[cfg(test)]
mod walkthrough_scenes;
#[cfg(test)]
mod opening_scenes;
#[cfg(test)]
mod recess_scenes;
#[cfg(test)]
mod level_state_scenes;
#[cfg(test)]
mod symbols_scenes;
#[cfg(test)]
mod ramp_scenes;
#[cfg(test)]
mod stair_scenes;
#[cfg(test)]
mod grid_scenes;
#[cfg(test)]
mod block_library_scenes;
#[cfg(test)]
mod slab_scenes;
#[cfg(test)]
mod space_scenes;
#[cfg(test)]
mod arch_scenes;
#[cfg(test)]
mod tag_scenes;
#[cfg(test)]
mod viewmark_scenes;
#[cfg(test)]
mod arch_project_scenes;
#[cfg(test)]
mod xref_scenes;
#[cfg(test)]
mod underlay_scenes;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use kentos_launch::Module;
use kentos_ui::theme::typography;

/// The exit code of a command line the program could not read.
const BAD_ARGUMENTS: u8 = 2;

/// `kentos-cad`: its own words first (`snapshot`, `kullan`), then
/// `[--modul <id>] [dosya]` (kentos-launch's command line). A command line it
/// cannot read is said on stderr, with exit code 2.
pub fn all_in_one(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let args: Vec<OsString> = args.into_iter().collect();
    let words = || {
        args.iter()
            .skip(1)
            .map(|a| a.to_string_lossy().into_owned())
    };
    match args.first().and_then(|a| a.to_str()) {
        // `kentos-cad snapshot çıktı.png [çizim.kcad]`: the window without a window.
        Some("snapshot") => return said(snapshot::run(words())),
        // `kentos-cad kullan <iz>`: a usage scenario played and pictured step by step.
        Some("kullan") => return said(usage::run(words())),
        _ => {}
    }
    match kentos_launch::args::parse(args) {
        Ok(launch) => run(launch.module, launch.path),
        Err(why) => refused(&why),
    }
}

/// A module's own program (`kentos-mimari [dosya]`): `--modul` may name only
/// this module, any other is refused with the program to start instead.
pub fn module_program(module: Module, args: impl IntoIterator<Item = OsString>) -> ExitCode {
    match program::fixed_launch(module, args) {
        Ok(path) => run(Some(module), path),
        Err(why) => refused(&why),
    }
}

/// A word's run, its error on stderr.
fn said(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn refused(why: &str) -> ExitCode {
    eprintln!("{why}");
    ExitCode::from(BAD_ARGUMENTS)
}

/// The window: `program` the module it works as (none: every module, kentos-cad's
/// own), `path` the drawing it opens at once.
fn run(program: Option<Module>, path: Option<PathBuf>) -> ExitCode {
    // Embedded faces: nothing has to be installed on the machine.
    typography::load();
    // Bound layers on PostGIS (docs/adr/0505).
    kentos_postgis::register();
    kentos_processing::command::register_as("desktop");
    // STAC searches (docs/adr/0528 §2): over the cloud client's HTTP.
    kentos_native_application::stac_host::register(Box::new(kentos_cloud::stac::HttpFetch));
    kentos_native_application::remote_host::register(Box::new(kentos_cloud::range::HttpRemote));

    // Today's midnight for Yeni sürüm oluştur and Sona erdir is the device's (docs/adr/0210 §7).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    kentos_interaction::time_version::LOCAL_OFFSET.store(
        cloud::local_time::Zone::system().offset_at(now),
        std::sync::atomic::Ordering::Relaxed,
    );

    let result = iced::application(
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
            let (mut app, task) = app::App::start_in(program, path.clone(), settings, recovery);
            // Device drafts of cloud projects (docs/adr/0040, 0041), next to the recovery copies.
            app.cloud.drafts = cloud::default_drafts();
            // Local copies of cloud projects: they open without a connection (docs/adr/0043).
            app.cloud.replicas = cloud::default_replicas();
            // The sheets' books, by the project's key, beside them (docs/sheet/design.md §10).
            app.sheet_store = sheets::default_store();
            // The device's NTv2 grids for the projects' datum choices (docs/adr/0168 §4).
            app.grids.library = grids::Library::device();
            // The forms designer's section library and kept drafts (docs/adr/0441 §1, §5).
            app.form_files = forms::library::FormFiles::device();
            // The recent files, kept beside the program's other history.
            if let Some(folder) = recent::RecentFiles::default_folder() {
                app.recent = recent::RecentFiles::open(&folder);
                // Kaynaklar's folders, beside them (docs/adr/0199 §7).
                app.sources.folders = sources::SourceFolders::open(&folder);
                // Bloklar's libraries, beside them (docs/adr/0726 §1).
                app.blocks_panel.library = blocks_library::Library::open(&folder);
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
                // Each window's size (pencereler.json, docs/adr/0746).
                app.dialog_sizes = dialog_sizes::Keeper::open(&dir);
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
    .run();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
