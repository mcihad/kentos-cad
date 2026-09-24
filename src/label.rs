//! Tip ölçeğine bağlı hazır metin biçimleri.
//!
//! Her fonksiyon boyutu, yazı tipi ve rengi ayarlanmış bir iced [`Text`]'i
//! döndürür; gerekirse `.width(..)` gibi ayarlarla sürdürülebilir. Yazı tipi
//! ve boyut o anki yazı ayarından okunur ([`typography`](crate::theme::typography));
//! ayar değişince bir sonraki çizimde bütün metinler yeni ayarla kurulur.

use iced::widget::text::IntoFragment;
use iced::widget::{Text, text as plain};

use crate::style;
use crate::theme::typography as ty;

/// Arayüz yazı tipinde, gövde boyutunda metin. Bileşenler iced'in `text`
/// yerine bunu kullanır: yazı tipi açıkça verildiği için ayar değişince
/// metin de değişir.
pub fn text<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    plain(content).font(ty::ui()).size(ty::body())
}

/// Gövde metni.
pub fn body<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content)
}

/// İkincil gövde metni.
pub fn muted<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    body(content).style(style::text::muted)
}

/// Açıklama, grup adı ve meta bilgisi (gövdeden 1 piksel küçük, ikincil renk).
pub fn caption<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).size(ty::caption()).style(style::text::muted)
}

/// Vurgulu gövde metni.
pub fn strong<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    body(content).font(ty::ui_strong())
}

/// Menü komutu ve marka adı (gövdeden 1 piksel büyük, yarı kalın).
pub fn heading<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).size(ty::heading()).font(ty::ui_strong())
}

/// İletişim kutusu ve seçili öğe başlığı (gövdeden 2 piksel büyük, yarı kalın).
pub fn title<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).size(ty::title()).font(ty::ui_strong())
}

/// Eş aralıklı gövde metni: koordinat, ölçü, komut.
pub fn mono<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    body(content).font(ty::mono())
}

/// Eş aralıklı ikincil metin.
pub fn mono_caption<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    caption(content).font(ty::mono())
}

/// Öne çıkan tek bir sayı (eş aralıklı, gövdenin 5/3'ü).
pub fn figure<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).size(ty::figure()).font(ty::mono())
}
