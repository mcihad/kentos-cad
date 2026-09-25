//! # kentos-ui: KentOS UI bileşenleri
//!
//! [iced] üzerine kurulu, CBS ve CAD uygulamaları için bileşen kütüphanesi.
//!
//! Katmanlar alttan üste:
//!
//! - [`attribute`]: öznitelik veri modeli; türlü değerler, alanlar, tarih
//!   ve saat, sorgular. Arayüzden bağımsızdır.
//! - [`theme`]: renk belirteçleri ([`Tokens`](theme::Tokens)), tip ölçeği
//!   ve iced teması. Bileşenler renklerini temadan okur.
//! - [`style`]: iced bileşenleri için stil fonksiyonları.
//! - [`icon`]: 16×16 ızgarada çizilmiş vektör ikon seti.
//! - [`label`]: tip ölçeğine bağlı hazır metin biçimleri.
//! - [`widget`]: uygulama çerçevesi; şerit (ribbon), uygulama menüsü, yan
//!   paneller, tablo, özellik ızgarası, komut satırı, durum çubuğu.
//! - [`spatial`]: CBS ve CAD bileşenleri; projeksiyon, katman modeli,
//!   yakalama, çizim araçları, model alanı ve ViewCube (`spatial`
//!   özelliği, varsayılan olarak açık).
//! - `snapshot`: arayüzü pencere açmadan çizip PNG'ye yazar; ekran kapalı
//!   ya da kilitliyken de çalışır (`snapshot` özelliği).
//!
//! Bileşenler uygulamanın `Message` türünden bağımsızdır ve yapıcı
//! (builder) desenini izler:
//!
//! ```ignore
//! use kentos_ui::widget::ribbon::{self, Ribbon};
//! use kentos_ui::icon::Icon;
//!
//! Ribbon::new()
//!     .tabs(Tab::ALL, self.tab, Message::TabSelected)
//!     .group(
//!         ribbon::Group::new("Görünüm")
//!             .push(ribbon::Button::large(Icon::ZoomExtents, "Tümünü\ngör").on_press(Message::FitAll)),
//!     )
//! ```
//!
//! [iced]: https://github.com/iced-rs/iced

pub mod attribute;
pub mod icon;
pub mod label;
pub mod style;
pub mod theme;
pub mod widget;

#[cfg(feature = "spatial")]
pub mod spatial;

#[cfg(feature = "snapshot")]
pub mod snapshot;
