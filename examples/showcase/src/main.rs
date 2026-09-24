//! KentOS CAD: kentos-rc bileşenlerinin vitrin uygulaması.
//!
//! Uygulama yalnızca durumu, mesajları, komut yorumlayıcısını ve örnek
//! veriyi tutar; arayüzün tamamı kentos-rc bileşenleriyle kurulur.

mod app;
mod command;
mod gallery;
mod message;
mod sample;
mod table;
mod view;

use iced::Size;

use kentos_rc::theme::typography;

fn main() -> iced::Result {
    iced::application(
        app::Showcase::new,
        app::Showcase::update,
        app::Showcase::view,
    )
    .title("KentOS CAD — Türkiye örnek verisi")
    .theme(app::Showcase::theme)
    .default_font(typography::UI)
    .subscription(app::Showcase::subscription)
    .window_size(Size::new(1440.0, 900.0))
    .antialiasing(true)
    .run()
}
