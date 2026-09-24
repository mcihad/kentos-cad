//! Çerçeve sayfası: durum çubuğu, komut kutusu, gezinme çubuğu, uygulama
//! menüsü ve iletişim kutuları.

use iced::widget::{button, column, container, row, space};
use iced::{Center, Element, Fill, Theme};

use kentos_rc::icon::{Icon, Tone, icon};
use kentos_rc::label;
use kentos_rc::spatial::model_space;
use kentos_rc::style;
use kentos_rc::widget::command_line::Prompt;
use kentos_rc::widget::status_bar::{Readout, Toggle};
use kentos_rc::widget::{CommandLine, ContextMenu, Dialog, Menu, NavigationBar, StatusBar};

use super::{entry, pressed};
use crate::app::Showcase;
use crate::command;
use crate::gallery::Demo;
use crate::message::Message;

impl Showcase {
    pub(super) fn frame_page(&self) -> Vec<Element<'_, Message>> {
        let gallery = &self.gallery;

        let coordinates = row![
            label::mono("41.00820°"),
            label::caption("K"),
            space::horizontal().width(10),
            label::mono("28.97840°"),
            label::caption("D"),
        ]
        .spacing(3)
        .align_y(Center);

        let status = StatusBar::new()
            .push(
                Readout::new(coordinates)
                    .icon(Icon::Target)
                    .width(170.0)
                    .menu(|| {
                        Menu::new()
                            .header("Koordinat biçimi")
                            .check("Ondalık derece", true, pressed("Ondalık derece"))
                            .shortcut("41.00820° K")
                            .check("Derece, dakika, saniye", false, pressed("DMS"))
                            .shortcut("41°00'29.5\" K")
                    }),
            )
            .separator()
            .push(
                Readout::new(label::body("2 seçili"))
                    .icon(Icon::Select)
                    .menu(|| {
                        Menu::new()
                            .item("Seçime odaklan", pressed("Seçime odaklan"))
                            .icon(Icon::Target)
                            .item("Seçimi kaldır", pressed("Seçimi kaldır"))
                            .icon(Icon::ClearSelection)
                            .shortcut("Esc")
                    }),
            )
            .spacer()
            .push(
                Toggle::new("Izgara", gallery.toggles[0])
                    .icon(Icon::Grid)
                    .shortcut("F7")
                    .on_press(Message::Gallery(Demo::Toggled(0))),
            )
            .push(
                Toggle::new("Yakalama", gallery.toggles[1])
                    .icon(Icon::Magnet)
                    .shortcut("F3")
                    .on_press(Message::Gallery(Demo::Toggled(1))),
            )
            .push(
                Toggle::new("Etiketler", gallery.toggles[2])
                    .icon(Icon::Type)
                    .on_press(Message::Gallery(Demo::Toggled(2))),
            )
            .separator()
            .push(Readout::new(label::mono("1:25.000")).menu(|| {
                Menu::new()
                    .header("Ölçek")
                    .check("1:5.000", false, pressed("1:5.000"))
                    .check("1:25.000", true, pressed("1:25.000"))
                    .check("1:100.000", false, pressed("1:100.000"))
            }))
            .separator()
            .push(
                Readout::new(label::mono_caption("EPSG:3857"))
                    .icon(Icon::Globe)
                    .tip("WGS 84 / Pseudo-Mercator"),
            );

        let command_line = CommandLine::new(&gallery.history, &gallery.command)
            .commands(command::catalog())
            .prompt(
                Prompt::new("Sonraki köşeyi belirtin")
                    .command("ALAN")
                    .option("Geri al", pressed("Geri al"))
                    .description("Son köşeyi kaldırır.")
                    .option("Kapat", pressed("Kapat"))
                    .key("Enter")
                    .description("Son köşeyi ilk köşeye bağlayıp alanı tamamlar.")
                    .placeholder("bir komut deneyin"),
            )
            .on_input(|command| Message::Gallery(Demo::CommandChanged(command)))
            .on_submit(Message::Gallery(Demo::CommandSubmitted))
            .on_run(|command| Message::Gallery(Demo::CommandRun(command)))
            .expanded(gallery.command_expanded, |expanded| {
                Message::Gallery(Demo::CommandExpanded(expanded))
            });

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
                "Göstergeler değer gösterir; menüsü olan gösterge tıklanınca yukarı doğru \
                 açılır ve bunu sağındaki ok belli eder. Anahtarlar açıkken ikonlarıyla \
                 vurgulanır; zemin yalnızca üzerine gelince belirir. Değişen değerler sabit \
                 genişlikte durur, imleç hareket ettikçe çubuk kıpırdamaz.",
                status,
                Some(
                    "StatusBar::new()\n    \
                     .push(Readout::new(coordinates).icon(Icon::Target).width(170.0).menu(formats))\n    \
                     .separator()\n    \
                     .push(Toggle::new(\"Izgara\", grid).icon(Icon::Grid).shortcut(\"F7\").on_press(message))\n    \
                     .spacer()\n    \
                     .push(Readout::new(label::mono_caption(\"EPSG:3857\")).icon(Icon::Globe).tip(name))",
                ),
            ),
            entry(
                "Komut kutusu",
                "kentos_rc::widget::CommandLine",
                "Geçmiş, istem ve giriş. Yazılan komutlar › işaretiyle ve eş aralıklı \
                 yazıyla, yanıtlar düz yazıyla, hatalar kırmızıyla gösterilir; eski satırlar \
                 soluklaşır. İstem etkin komutun adımını ve seçeneklerini gösterir. Yazmaya \
                 başlayın: öneriler açılır; ↑ ↓ gezinir, Tab tamamlar, Enter çalıştırır. Giriş \
                 boşken ↑ önceki komutları getirir, ↓ bütün komutları listeler.",
                command_line,
                Some(
                    "CommandLine::new(&history, &input)\n    \
                     .commands(CATALOG)\n    \
                     .prompt(Prompt::new(\"Sonraki köşeyi belirtin\").command(\"ALAN\")\n        \
                     .option(\"Geri al\", Message::Undo)\n        \
                     .option(\"Kapat\", Message::Close).key(\"Enter\"))\n    \
                     .on_input(Message::CommandInput)\n    \
                     .on_submit(Message::CommandSubmitted)\n    \
                     .on_run(Message::CommandRun)",
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
                "Bağlam menüsü",
                "kentos_rc::widget::ContextMenu",
                "Herhangi bir öğeyi sarar; sağ tıklanan yerde açılır, pencere kenarına \
                 taşacaksa sola ya da yukarı döner. Komut, işaret, kısayol, başlık, alt \
                 menü, devre dışı ve tehlikeli komut destekler. Oklarla gezinilir, Enter \
                 seçer, Esc kapatır. Aşağıdaki alana sağ tıklayın.",
                self.context_menu_sample(),
                Some(
                    "ContextMenu::new(content, |position| {\n    Menu::new()\n        \
                     .item(\"Kopyala\", Message::Copy).icon(Icon::Copy).shortcut(\"Ctrl+C\")\n        \
                     .check(\"Izgara\", grid, Message::ToggleGrid)\n        \
                     .submenu(\"Hizala\", Menu::new().item(\"Sola\", Message::AlignLeft))\n        \
                     .separator()\n        \
                     .item(\"Sil\", Message::Delete).danger()\n})",
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

    /// Sağ tıklanınca bütün komut türlerini gösteren örnek menü açan alan.
    fn context_menu_sample(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;

        let area = container(
            column![
                icon(Icon::Select).size(20.0).tone(Tone::Muted),
                label::muted("Bu alana sağ tıklayın"),
            ]
            .spacing(8)
            .align_x(Center),
        )
        .center_x(Fill)
        .center_y(120)
        .style(style::container::field);

        ContextMenu::new(area, move |_| {
            Menu::new()
                .header("Düzen")
                .item("Kes", pressed("Kes"))
                .shortcut("Ctrl+X")
                .item("Kopyala", pressed("Kopyala"))
                .icon(Icon::Copy)
                .shortcut("Ctrl+C")
                .item("Yapıştır", None)
                .shortcut("Ctrl+V")
                .separator()
                .check(
                    "Izgara",
                    gallery.toggles[0],
                    Message::Gallery(Demo::Toggled(0)),
                )
                .shortcut("F7")
                .check(
                    "Yakalama",
                    gallery.toggles[1],
                    Message::Gallery(Demo::Toggled(1)),
                )
                .shortcut("F3")
                .submenu(
                    "Hizala",
                    Menu::new()
                        .item("Sola", pressed("Sola hizala"))
                        .item("Ortaya", pressed("Ortaya hizala"))
                        .item("Sağa", pressed("Sağa hizala")),
                )
                .icon(Icon::Layout)
                .separator()
                .item("Sil", pressed("Sil"))
                .icon(Icon::Eraser)
                .shortcut("Del")
                .danger()
        })
        .into()
    }
}
