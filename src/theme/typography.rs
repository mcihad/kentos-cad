//! Yazı tipleri ve tip ölçeği.
//!
//! Arayüz IBM Plex ailesini kullanır: metin için Plex Sans, koordinat,
//! ölçü ve komut satırı için Plex Mono. Yazı tipi sistemde yoksa iced
//! varsayılan yazı tipine döner.

use iced::Font;
use iced::font::Weight;

/// Arayüz metni.
pub const UI: Font = Font::with_name("IBM Plex Sans");

/// Başlıklar ve vurgulu etiketler.
pub const UI_STRONG: Font = Font {
    weight: Weight::Semibold,
    ..UI
};

/// Koordinatlar, ölçüler ve komut satırı için eş aralıklı yazı tipi.
pub const MONO: Font = Font::with_name("IBM Plex Mono");

/// Komut adları: yazılabilen anahtar sözcükler (ör. CIZGI).
pub const MONO_STRONG: Font = Font {
    weight: Weight::Semibold,
    ..MONO
};

/// Açıklamalar, grup adları, tablo başlıkları, meta bilgisi.
pub const CAPTION: f32 = 11.0;

/// Kontroller ve gövde metni.
pub const BODY: f32 = 12.0;

/// Menü komutları ve marka adı.
pub const HEADING: f32 = 13.0;

/// İletişim kutusu ve seçili öğe başlıkları.
pub const TITLE: f32 = 14.0;

/// Öne çıkan tek bir değer (ör. toplam uzunluk).
pub const FIGURE: f32 = 20.0;
