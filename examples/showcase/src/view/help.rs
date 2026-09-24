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
            .item("Sol tık + sürükle", "Her araçta gezinme")
            .item("Orta tuş + sürükle", "Her araçta gezinme")
            .item("Tekerlek", "İmlecin altındaki noktaya yakınlaştırma")
            .item("Seç + sol tık", "Öğeyi seçer, özellikler sağ panelde")
            .item("Ölç + sol tık", "Ölçüm noktası ekler")
            .item("Ölç + sağ tık", "Ölçümü temizler")
            .item(
                "Çizim + sol tık",
                "Nokta ekler, yakalama açıkken köşeye tutunur",
            )
            .item("Çizim + sağ tık", "Çoklu çizgiyi veya alanı bitirir")
            .item("Esc", "Çizimi bitirir, seçimi ve ölçümü temizler")
            .item("Delete", "Seçili çizimi siler")
            .item("F1  F3  F7", "Kısayollar, nesne yakalama, ızgara")
            .item(
                "Komut satırı",
                "CIZGI, DAIRE, OLC, TUMUNU, YARDIM ve diğerleri",
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
