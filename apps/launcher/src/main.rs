//! KentOS Launcher (docs/adr/0800): KentOS's programs (KentOS Mimari,
//! Harita, GIS) and any other program or web address, each from its
//! manifest, started from one window; the recent drawings of the KentOS
//! programs. Built on KentOS UI like the desktop program; a program it starts
//! is a process of its own.
//!
//! `kentos-launcher` opens the window; `kentos-launcher snapshot çıktı.png
//! [seçenekler]` draws it into an image without opening it.

mod app;
mod cards;
mod draft;
mod files;
mod glyphs;
mod locations;
mod manifest;
mod notices;
mod pictures;
mod process;
mod recent;
mod registry;
mod resolve;
mod settings;
mod snapshot;
mod tones;
mod view;

use iced::{Size, window};
use kentos_ui::theme::typography;

/// The window at first, and the smallest it is made (CLAUDE.md §1: 1100 wide at least).
const WINDOW: Size = Size::new(1440.0, 900.0);
const SMALLEST: Size = Size::new(1100.0, 650.0);

fn main() -> iced::Result {
    // The embedded faces: nothing has to be installed on the machine.
    typography::load();

    let mut args = std::env::args().skip(1).peekable();
    if args.peek().map(String::as_str) == Some("snapshot") {
        args.next();
        if let Err(error) = snapshot::run(args) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }

    iced::application(app::Launcher::boot, app::Launcher::update, view::view)
        .title("KentOS Launcher")
        .theme(app::Launcher::theme)
        .subscription(app::Launcher::subscription)
        .default_font(typography::ui())
        .window(window::Settings {
            size: WINDOW,
            min_size: Some(SMALLEST),
            ..window::Settings::default()
        })
        .antialiasing(true)
        .run()
}
