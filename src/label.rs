//! Tip ölçeğine bağlı hazır metin biçimleri.
//!
//! Her fonksiyon boyutu, yazı tipi ve rengi ayarlanmış bir iced
//! [`Text`]'i döndürür; gerekirse `.width(..)` gibi ayarlarla sürdürülebilir.

use iced::widget::text::IntoFragment;
use iced::widget::{Text, text};

use crate::style;
use crate::theme::typography as ty;

/// Gövde metni (12 px).
pub fn body<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).size(ty::BODY)
}

/// İkincil gövde metni.
pub fn muted<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    body(content).style(style::text::muted)
}

/// Açıklama, grup adı ve meta bilgisi (11 px, ikincil renk).
pub fn caption<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).size(ty::CAPTION).style(style::text::muted)
}

/// Vurgulu gövde metni.
pub fn strong<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    body(content).font(ty::UI_STRONG)
}

/// Menü komutu ve marka adı (13 px, yarı kalın).
pub fn heading<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).size(ty::HEADING).font(ty::UI_STRONG)
}

/// İletişim kutusu ve seçili öğe başlığı (14 px, yarı kalın).
pub fn title<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).size(ty::TITLE).font(ty::UI_STRONG)
}

/// Eş aralıklı gövde metni: koordinat, ölçü, komut.
pub fn mono<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    body(content).font(ty::MONO)
}

/// Eş aralıklı ikincil metin.
pub fn mono_caption<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    caption(content).font(ty::MONO)
}

/// Öne çıkan tek bir sayı (20 px, eş aralıklı).
pub fn figure<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).size(ty::FIGURE).font(ty::MONO)
}
