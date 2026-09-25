//! KentOS CAD desktop (docs/adr/0010, 0017): native Iced with the KentOS UI
//! components, the shared Rust contracts and KentOS's own wgpu drawing area
//! (docs/adr/0019).
//! `kentos-cad [çizim.kcad]` opens the drawing at once; `kentos-cad snapshot
//! çıktı.png [çizim.kcad]` draws the window into an image without opening it.

mod app;
mod catalog;
mod document;
mod icons;
mod snapshot;
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
        move || app::App::boot(path.clone()),
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
