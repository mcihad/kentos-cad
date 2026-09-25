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
                "Seçimin üstündeki çubuk",
                "Yakınlaştır, özellikler, tablo, kilitle, sil; imleç uzaklaşınca soluklaşır",
            )
            .item(
                "Boşluk",
                "İmlecin yerinde dairesel araç menüsü: yöne çekip tıklayın ya da basılı tutup bırakın",
            )
            .item(
                "Panel sekmesi",
                "Sürükleyerek başka yığına, kenara ya da ortaya (yüzen pencere) taşır",
            )
            .item(
                "Yuva kenarı",
                "Alanları ve yığınları sürükleyerek boyutlandırır",
            )
            .item(
                "Katman ağacında sürükle",
                "Katmanı ya da grubu önüne, ardına ya da grubun içine taşır",
            )
            .item(
                "Ağaçta F2",
                "Seçili grubu ya da katmanı yerinde adlandırır; Enter kaydeder, Esc vazgeçer",
            )
            .item(
                "Panel başlığına çift tık",
                "Yığını başlığına daraltır ya da açar; ⋯ menüsü yüzdürür, kapatır",
            )
            .item(
                "Düzende cetvelden sürükle",
                "Kılavuz çıkarır; kılavuzu cetvele geri bırakmak siler, Shift çizgilere oturtur",
            )
            .item(
                "F1  F2  F3  F7",
                "Kısayollar, komut geçmişi (ağaçta: adlandır), nesne yakalama, ızgara",
            )
            .item("Ctrl+R", "Düzende cetvelleri gösterir ya da gizler")
            .item(
                "Ctrl +  Ctrl −  Ctrl 0",
                "Yazıyı büyütür, küçültür, varsayılan boyuta döndürür",
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
