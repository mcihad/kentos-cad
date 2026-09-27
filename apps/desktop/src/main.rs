//! KentOS CAD desktop (docs/adr/0010, 0017): native Iced with the KentOS UI
//! components, the shared Rust contracts and KentOS's own wgpu drawing area
//! (docs/adr/0019).
//! `kentos-cad [çizim.kcad]` opens the drawing at once; `kentos-cad snapshot
//! çıktı.png [çizim.kcad]` draws the window into an image without opening it.

mod app;
mod app_menu;
mod appearance;
mod bottom;
mod calc;
mod catalog;
mod clipboard;
mod cloud;
mod command_bar;
mod crs;
mod document;
mod drawing_fonts;
mod drawing_menus;
mod exchange;
mod expression;
#[cfg(test)]
mod files_testing;
mod hover_card;
mod icons;
mod input;
mod keys;
mod keytips;
mod labels;
mod layer_tree;
mod layering;
mod layout;
mod layout_plan;
#[cfg(test)]
mod layout_plan_tests;
#[cfg(test)]
mod layout_tests;
mod log_plan;
#[cfg(test)]
mod log_plan_tests;
mod map_marks;
mod marks;
mod message_log;
#[cfg(test)]
mod message_log_tests;
mod modes;
mod opening;
#[cfg(test)]
mod perf;
mod point_calc;
mod preview;
mod processing;
mod project;
mod properties;
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
#[cfg(test)]
mod screens;
mod selecting;
mod settings;
mod settings_view;
mod snapshot;
mod start;
mod style;
mod text_field;
mod traces;
mod tracking;
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
            // The recent files, kept beside the program's other history.
            if let Some(folder) = recent::RecentFiles::default_folder() {
                app.recent = recent::RecentFiles::open(&folder);
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
