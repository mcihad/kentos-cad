//! KentOS CAD desktop (docs/adr/0010, 0017): native Iced with the KentOS UI
//! components, the shared Rust contracts and KentOS's own wgpu drawing area
//! (docs/adr/0019).
//! `kentos-cad [çizim.kcad]` opens the drawing at once; `kentos-cad snapshot
//! çıktı.png [çizim.kcad]` draws the window into an image without opening it.

mod app;
mod catalog;
mod cloud;
mod document;
#[cfg(test)]
mod files_testing;
mod icons;
mod input;
mod keys;
mod marks;
mod opening;
#[cfg(test)]
mod perf;
mod preview;
mod recovery;
mod saving;
#[cfg(test)]
mod screens;
mod selecting;
mod settings;
mod settings_view;
mod snapshot;
mod traces;
mod view;
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
            (app, task)
        },
        app::App::update,
        app::App::view,
    )
    .title(app::App::title)
    .theme(app::App::theme)
    .default_font(typography::ui())
    .subscription(app::App::subscription)
    .window_size(iced::Size::new(1440.0, 900.0))
    // A drawing with unsaved changes asks before the window closes.
    .exit_on_close_request(false)
    .antialiasing(true)
    .run()
}
