//! KentOS CAD: kentos-rc bileşenlerinin vitrin uygulaması.
//!
//! Uygulama yalnızca durumu, mesajları, komut yorumlayıcısını ve örnek
//! veriyi tutar; arayüzün tamamı kentos-rc bileşenleriyle kurulur.

mod app;
mod command;
mod gallery;
mod jobs;
mod layer_tree;
mod message;
mod sample;
mod settings;
mod snapshot;
mod table;
mod view;

use kentos_rc::theme::typography;

fn main() -> iced::Result {
    // Gömülü yazı tipleri: makinede kurulu olmaları gerekmez.
    typography::load();

    // `showcase snapshot çıktı.png ...`: pencere açmadan görüntü alır.
    let mut args = std::env::args().skip(1);

    if args.next().as_deref() == Some("snapshot") {
        if let Err(error) = snapshot::run(args) {
            eprintln!("{error}");
            std::process::exit(1);
        }

        return Ok(());
    }

    // Saklanan tema ve yazı ayarı; yazı ayarı pencere açılmadan verilir ki
    // varsayılan yazı tipi de seçilen aile olsun.
    let path = settings::Settings::path();
    let saved = path
        .as_deref()
        .map(settings::Settings::load)
        .unwrap_or_default();

    typography::set(saved.typography);

    iced::application(
        move || app::Showcase::boot(saved, path.clone()),
        app::Showcase::update,
        app::Showcase::view,
    )
    .title("KentOS CAD — Türkiye örnek verisi")
    .theme(app::Showcase::theme)
    .default_font(typography::ui())
    .subscription(app::Showcase::subscription)
    .window_size(app::WINDOW_SIZE)
    .antialiasing(true)
    .run()
}
