//! Çerçeve sayfası: durum çubuğu, komut satırı, gezinme çubuğu, uygulama
//! menüsü ve iletişim kutuları.

use iced::widget::{button, column, container, row};
use iced::{Center, Element, Theme};

use kentos_rc::icon::Icon;
use kentos_rc::label;
use kentos_rc::spatial::model_space;
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::widget::status_bar::Toggle;
use kentos_rc::widget::{CommandLine, Dialog, NavigationBar, StatusBar};

use super::{entry, pressed};
use crate::app::Showcase;
use crate::gallery::Demo;
use crate::message::Message;

impl Showcase {
    pub(super) fn frame_page(&self) -> Vec<Element<'_, Message>> {
        let gallery = &self.gallery;

        let status = StatusBar::new()
            .push(label::mono(" 41.00820,  28.97840").width(156))
            .separator()
            .push(label::caption("MODEL").font(typography::UI_STRONG))
            .separator()
            .push(
                Toggle::new("Izgara", gallery.toggles[0])
                    .shortcut("F7")
                    .on_press(Message::Gallery(Demo::Toggled(0))),
            )
            .push(
                Toggle::new("Yakalama", gallery.toggles[1])
                    .shortcut("F3")
                    .on_press(Message::Gallery(Demo::Toggled(1))),
            )
            .push(
                Toggle::new("Etiketler", gallery.toggles[2])
                    .on_press(Message::Gallery(Demo::Toggled(2))),
            )
            .spacer()
            .push(label::mono_caption("1:25.000").style(style::text::default))
            .separator()
            .push(label::mono_caption("EPSG:3857"));

        let command_line = CommandLine::new(&gallery.history, &gallery.command)
            .placeholder("Bir komut deneyin")
            .on_input(|command| Message::Gallery(Demo::CommandChanged(command)))
            .on_submit(Message::Gallery(Demo::CommandSubmitted));

        let navigation = container(
            NavigationBar::new()
                .button(Icon::ZoomIn, "Yakınlaştır", pressed("Yakınlaştır"))
                .button(Icon::ZoomOut, "Uzaklaştır", pressed("Uzaklaştır"))
                .separator()
                .button(Icon::ZoomExtents, "Tümünü gör", pressed("Tümünü gör"))
                .button(
                    Icon::Home,
                    "Başlangıç görünümü",
                    pressed("Başlangıç görünümü"),
                ),
        )
        .padding(16)
        .style(|theme: &Theme| {
            style::container::solid(model_space::Style::of(theme).background)(theme)
        });

        let dialog = Dialog::new("Çizim kaydedilmedi")
            .hint("Ctrl+S")
            .width(440.0)
            .push(label::muted(
                "Çizimler katmanındaki değişiklikler kaydedilmedi. Kapatmadan önce \
                 kaydetmek ister misiniz?",
            ))
            .action(
                button(label::body("Kaydetme"))
                    .on_press(pressed("Kaydetme"))
                    .padding([5, 16])
                    .style(style::button::secondary),
            )
            .action(
                button(label::body("Kaydet"))
                    .on_press(pressed("Kaydet"))
                    .padding([5, 16])
                    .style(style::button::primary),
            );

        vec![
            entry(
                "Durum çubuğu",
                "kentos_rc::widget::StatusBar",
                "Öğeler soldan dizilir; spacer sonrakileri sağ uca iter. Anahtarların \
                 ipucu durumu ve kısayolu gösterir. Anahtarlara tıklayın.",
                status,
                Some(
                    "StatusBar::new()\n    .push(label::mono(coordinates))\n    .separator()\n    \
                     .push(Toggle::new(\"Izgara\", grid).shortcut(\"F7\").on_press(message))\n    \
                     .spacer()\n    .push(label::mono_caption(\"EPSG:3857\"))",
                ),
            ),
            entry(
                "Komut satırı",
                "kentos_rc::widget::CommandLine",
                "Son komutların geçmişi ve komut girişi. Kullanıcının yazdıkları \
                 \"Komut:\" önekiyle, yanıtlar sönük gösterilir. Bu örneğe yazıp Enter'a \
                 basın.",
                command_line,
                Some(
                    "CommandLine::new(&history, &input)\n    .placeholder(\"Komut yazın\")\n    \
                     .on_input(Message::CommandInput)\n    \
                     .on_submit(Message::CommandSubmitted)",
                ),
            ),
            entry(
                "Gezinme çubuğu",
                "kentos_rc::widget::NavigationBar",
                "Model alanının köşesinde duran dar, dikey görünüm denetimleri. \
                 Yüksekliğini bildirir; model alanı o bölgede artı imleci çizmez.",
                row![
                    navigation,
                    label::muted(
                        "İpuçları düğmelerin soluna açılır. Model alanında ViewCube'un \
                         altında yer alır."
                    )
                    .width(320),
                ]
                .spacing(16)
                .align_y(Center),
                Some(
                    "NavigationBar::new()\n    \
                     .button(Icon::ZoomIn, \"Yakınlaştır\", Message::ZoomIn)\n    \
                     .separator()\n    \
                     .button(Icon::Home, \"Başlangıç görünümü\", Message::ResetView)",
                ),
            ),
            entry(
                "İletişim kutusu",
                "kentos_rc::widget::Dialog",
                "Başlık, gövde ve sağa hizalı eylemlerden oluşan kutu. Kendi başına bir \
                 kaplama değildir; overlay::modal ile ortalanır ve arkası karartılır.",
                column![
                    dialog,
                    button(label::body("Kısayollar iletişim kutusunu aç"))
                        .on_press(Message::HelpToggled)
                        .padding([4, 12])
                        .style(style::button::secondary),
                ]
                .spacing(12),
                Some(
                    "overlay::modal(\n    Dialog::new(\"Çizim kaydedilmedi\").push(body).action(save),\n    \
                     Message::CloseDialog,\n)",
                ),
            ),
            entry(
                "Uygulama menüsü",
                "kentos_rc::widget::AppMenu",
                "Şeridin marka düğmesinden açılan büyük menü: solda komutlar, sağda \
                 ayrıntı bölmesi, altta eylemler. Bir kaplama olduğu için burada \
                 gösterilmez; düğmeyle açın.",
                button(label::body("Uygulama menüsünü aç"))
                    .on_press(Message::AppMenuToggled)
                    .padding([4, 12])
                    .style(style::button::secondary),
                Some(
                    "AppMenu::new(Message::CloseMenu)\n    \
                     .entry(Entry::new(Icon::DocumentNew, \"Yeni\", \"Görünümü sıfırlar\"))\n    \
                     .detail(Pane::new(\"Son kullanılanlar\", \"…\").push(item))\n    \
                     .action(Action::primary(Icon::Power, \"Çık\", Message::Quit))",
                ),
            ),
        ]
    }
}
