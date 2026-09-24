//! Kısayollar ve komutlar iletişim kutusu (F1).

use iced::Element;
use iced::widget::button;

use kentos_rc::label;
use kentos_rc::style;
use kentos_rc::widget::{Dialog, ShortcutList, overlay};

use crate::app::Showcase;
use crate::message::Message;

impl Showcase {
    pub(super) fn help(&self) -> Element<'_, Message> {
        let shortcuts = ShortcutList::new()
            .item(
                "Seç + tık",
                "Öğeyi seçer; özellikleri sağ panelde düzenlenir",
            )
            .item(
                "Soldan sağa sürükle",
                "Pencere seçimi: tamamı içeride kalan öğeler",
            )
            .item(
                "Sağdan sola sürükle",
                "Kesişen seçim: pencereye değen öğeler de",
            )
            .item("Shift / Ctrl", "Seçime ekler / seçimden çıkarır")
            .item(
                "Tabloda Shift",
                "Birincil satırdan tıklanan satıra kadar seçer",
            )
            .item("Ctrl+A", "Tablodaki kayıtların hepsini seçer")
            .item("Sağ tık", "Katman, harita ya da tablo satırı menüsünü açar")
            .item(
                "Orta tuş + sürükle",
                "Her araçta gezinme; Kaydır aracında sol tuş da",
            )
            .item("Tekerlek", "İmlecin altındaki noktaya yakınlaştırma")
            .item("Ölç + tık", "Ölçüm noktası ekler; sağ tık temizler")
            .item(
                "Çizim + tık",
                "Nokta ekler; sağ tık çoklu çizgiyi veya alanı bitirir",
            )
            .item(
                "Esc",
                "Pencereyi, haritadan seçimi ya da etkin komutu kapatır; yoksa seçimi temizler",
            )
            .item("Delete", "Seçili çizimleri siler")
            .item(
                "F1  F2  F3  F7",
                "Kısayollar, komut geçmişi, nesne yakalama, ızgara",
            )
            .item(
                "Yazmaya başla",
                "Komut kutusuna yazar; komut ya da \"enlem, boylam\" girilir",
            )
            .item("↑ ↓  Tab", "Önerilerde gezinir, öneriyi tamamlar")
            .item(
                "Boşken ↑ ↓",
                "Önceki komutları getirir; bütün komutları listeler",
            )
            .item(
                "Boşken Enter",
                "Çizimi bitirir; etkin komut yokken son komutu yineler",
            );

        let dialog = Dialog::new("Kısayollar ve komutlar")
            .hint("F1")
            .push(shortcuts)
            .action(
                button(label::body("Kapat"))
                    .on_press(Message::HelpToggled)
                    .padding([5, 16])
                    .style(style::button::primary),
            );

        overlay::modal(dialog, Message::HelpToggled)
    }
}
