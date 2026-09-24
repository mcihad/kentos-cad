//! KentOS CAD: kentos-rc bileşenlerinin vitrin uygulaması.
//!
//! Uygulama yalnızca durumu, mesajları, komut yorumlayıcısını ve örnek
//! veriyi tutar; arayüzün tamamı kentos-rc bileşenleriyle kurulur.

mod app;
mod command;
mod gallery;
mod layer_tree;
mod message;
mod sample;
mod snapshot;
mod table;
mod view;

use kentos_rc::theme::typography;

fn main() -> iced::Result {
    // `showcase snapshot çıktı.png ...`: pencere açmadan görüntü alır.
    let mut args = std::env::args().skip(1);

    if args.next().as_deref() == Some("snapshot") {
        if let Err(error) = snapshot::run(args) {
            eprintln!("{error}");
            std::process::exit(1);
        }

        return Ok(());
    }

    iced::application(
        app::Showcase::new,
        app::Showcase::update,
        app::Showcase::view,
    )
    .title("KentOS CAD — Türkiye örnek verisi")
    .theme(app::Showcase::theme)
    .default_font(typography::UI)
    .subscription(app::Showcase::subscription)
    .window_size(app::WINDOW_SIZE)
    .antialiasing(true)
    .run()
}
