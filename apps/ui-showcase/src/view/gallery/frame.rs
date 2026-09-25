//! Çerçeve sayfası: kayan araç pencereleri, durum çubuğu, komut kutusu,
//! gezinme çubuğu, bağlam menüsü, mini araç çubuğu, dairesel menü,
//! uygulama menüsü ve iletişim kutuları.

use iced::widget::{
    Column, button, checkbox, column, container, mouse_area, pin, row, space, stack,
};
use iced::{Border, Center, Element, Fill, Theme, keyboard};

use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::spatial::{Tool, model_space};
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::command_line::Prompt;
use kentos_ui::widget::status_bar::{Readout, Toggle};
use kentos_ui::widget::{
    CommandLine, ContextMenu, Dialog, Floating, Menu, MiniToolbar, NavigationBar, RadialMenu,
    StatusBar, ToolWindow,
};

use super::{entry, pressed};
use crate::app::Showcase;
use crate::command;
use crate::gallery::{Demo, DemoPane, SHAPES, SNAP_KINDS};
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
                "Kayan araç pencereleri",
                "kentos_ui::widget::Floating",
                "Harita üstünde sürüklenen, arkadaki işi kilitlemeyen pencereler. Başlıktan \
                 sürükleyin: kenarlara ve birbirlerine yaklaşınca yakalanırlar, sarı kılavuz \
                 nerede durduklarını gösterir (Ctrl yakalamayı kapatır). Tıklanan pencere öne \
                 gelir ve üstünde vurgu çizgisi belirir. Başlığa çift tıklamak pencereyi \
                 başlığına daraltır; başlıktaki bilgi daraltılmışken de okunur. Konum en yakın \
                 kenara göre saklanır: alan daralınca pencere o kenarla birlikte kayar.",
                self.floating_sample(),
                Some(
                    "Floating::new(map, &self.windows, Message::Window, |pane| match pane {\n    \
                     Pane::Snap => ToolWindow::new(\"Nesne yakalama\", snaps)\n        \
                     .icon(Icon::Magnet)\n        \
                     .meta(\"3 açık\")\n        \
                     .resizable(),\n    \
                     Pane::Layer => ToolWindow::new(\"Katman\", summary),\n\
                     })\n\n\
                     // update\n\
                     Message::Window(event) => self.windows.update(event),",
                ),
            ),
            entry(
                "Durum çubuğu",
                "kentos_ui::widget::StatusBar",
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
                "kentos_ui::widget::CommandLine",
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
                "kentos_ui::widget::NavigationBar",
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
                "kentos_ui::widget::Dialog",
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
                "kentos_ui::widget::ContextMenu",
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
                "Mini araç çubuğu",
                "kentos_ui::widget::MiniToolbar",
                "Seçimin üstünde beliren küçük çubuk: seçimle en sık yapılan işler. Üstte yer \
                 yoksa altına geçer, alanın kenarlarından taşmaz. İmleç uzaklaştıkça \
                 soluklaşır ve çizimi kapatmaz, yaklaşınca belirginleşir; düğmenin adı ve \
                 kısayolu çubuğun seçimden uzak yanında yazar. Şekillere tıklayın; kilitli \
                 şekil silinemez.",
                self.mini_toolbar_sample(),
                Some(
                    "MiniToolbar::new(model_space, self.selection_bounds())\n    \
                     .button(Icon::Target, \"Seçime yakınlaştır\", Message::FocusSelection)\n    \
                     .button(Icon::Copy, \"Kopyala\", Message::Copy)\n    \
                     .shortcut(\"Ctrl+C\")\n    \
                     .separator()\n    \
                     .button(Icon::Lock, \"Kilitle\", Message::Lock)\n    \
                     .active(locked)\n    \
                     .button(Icon::Eraser, \"Sil\", (!locked).then_some(Message::Delete))\n    \
                     .danger()",
                ),
            ),
            entry(
                "Dairesel menü",
                "kentos_ui::widget::RadialMenu",
                "İmlecin yerinde açılan, komutları çevresinde hep aynı yönlerde dizen menü \
                 (Blender'daki pasta, Maya'daki işaretleme menüsü gibi). Alana basılı tutup \
                 bir yöne çekin ve bırakın; kısa tıklarsanız menü açık kalır, yönü seçip \
                 tıklayın. Ortaya tıklamak, sağ tık ya da Esc kapatır. Model alanında Boşluk \
                 tuşu aynı menüyü açar: basılı tutup çekip bırakmak aracı hemen seçer.",
                self.radial_sample(),
                Some(
                    "RadialMenu::new(model_space, self.radial_open, Message::RadialClosed)\n    \
                     .hold(keyboard::Key::Named(key::Named::Space))\n    \
                     .item(Icon::Select, \"Seç\", Message::ToolSelected(Tool::Select))\n    \
                     .item(Icon::Line, \"Çizgi\", Message::ToolSelected(Tool::Line))\n    \
                     .item(Icon::Polyline, \"Çoklu çizgi\", Message::ToolSelected(Tool::Polyline))",
                ),
            ),
            entry(
                "Uygulama menüsü",
                "kentos_ui::widget::AppMenu",
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

    /// İki kayan pencere ve arkalarında, pencereler açıkken de çalışan bir
    /// düğme.
    fn floating_sample(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;

        let presses = match gallery.stage_presses {
            0 => "Pencereler açıkken de arkadaki alan çalışır.".to_owned(),
            count => format!("Arkadaki düğmeye {count} kez basıldı."),
        };

        let stage = container(
            column![
                button(label::body("Arkadaki düğme"))
                    .on_press(Message::Gallery(Demo::StagePressed))
                    .padding([4, 12])
                    .style(style::button::secondary),
                label::caption(presses),
                button(label::caption("Pencereleri ilk yerlerine al").style(style::text::default))
                    .on_press(Message::Gallery(Demo::PanesReset))
                    .padding([2, 8])
                    .style(style::button::flat),
            ]
            .spacing(8)
            .align_x(Center),
        )
        .center_x(Fill)
        .center_y(Fill)
        .style(|theme: &Theme| {
            style::container::solid(model_space::Style::of(theme).background)(theme)
        });

        let floating = Floating::new(
            stage,
            &gallery.panes,
            |event| Message::Gallery(Demo::Window(event)),
            move |pane| match pane {
                DemoPane::Snap => {
                    let snaps = SNAP_KINDS.iter().enumerate().map(|(index, name)| {
                        checkbox(gallery.snaps[index])
                            .label(*name)
                            .size(13.0)
                            .font(typography::ui())
                            .text_size(typography::body())
                            .on_toggle(move |_| Message::Gallery(Demo::SnapToggled(index)))
                            .into()
                    });

                    let open = gallery.snaps.iter().filter(|snap| **snap).count();

                    ToolWindow::new(
                        "Nesne yakalama",
                        Column::with_children(snaps).spacing(6).padding([10, 12]),
                    )
                    .icon(Icon::Magnet)
                    .meta(format!("{open} açık"))
                    .width(236.0)
                    .min_size(iced::Size::new(180.0, 72.0))
                    .resizable()
                }
                DemoPane::Layer => ToolWindow::new(
                    "Katman",
                    column![
                        summary_row("Ad", "Parseller"),
                        summary_row("Öğe", "1.284"),
                        summary_row("Seçili", "12"),
                        summary_row("Koordinat", "EPSG:5254"),
                    ]
                    .spacing(4)
                    .padding([10, 12]),
                )
                .icon(Icon::Layers)
                .meta("12 seçili")
                .width(220.0),
            },
        )
        .height(typography::scaled(260.0));

        container(floating)
            .padding(1)
            .style(style::container::bordered)
            .into()
    }

    /// Mini araç çubuğu örneği: seçilebilen üç şekil ve seçimin üstündeki
    /// çubuk.
    fn mini_toolbar_sample(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;

        let background = mouse_area(
            container(label::caption("Boşluğa tıklamak seçimi bırakır.").style(style::text::muted))
                .padding([8, 12])
                .width(Fill)
                .height(Fill)
                .style(|theme: &Theme| {
                    style::container::solid(model_space::Style::of(theme).background)(theme)
                }),
        )
        .on_press(Message::Gallery(Demo::ShapeSelected(None)));

        let mut stage = stack![background];

        for (index, (name, bounds, color)) in SHAPES.into_iter().enumerate() {
            if gallery.shapes_deleted[index] {
                continue;
            }

            let selected = gallery.shape == Some(index);
            let locked = gallery.shapes_locked[index];
            let mut title = row![label::caption(name).style(style::text::default)]
                .spacing(4)
                .align_y(Center);

            if locked {
                title = title.push(icon(Icon::Lock).size(12.0).tone(Tone::Muted));
            }

            let shape = button(container(title).padding([4, 6]))
                .on_press(Message::Gallery(Demo::ShapeSelected(Some(index))))
                .width(bounds.width)
                .height(bounds.height)
                .padding(0)
                .style(move |theme: &Theme, _| {
                    let t = Tokens::of(theme);

                    button::Style {
                        background: Some(color.scale_alpha(0.22).into()),
                        text_color: t.text,
                        border: Border {
                            color: if selected { t.accent } else { color },
                            width: if selected { 2.0 } else { 1.0 },
                            radius: 2.0.into(),
                        },
                        ..button::Style::default()
                    }
                });

            stage = stage.push(pin(shape).x(bounds.x).y(bounds.y));
        }

        let anchor = gallery
            .shape
            .filter(|shape| !gallery.shapes_deleted[*shape])
            .map(|shape| SHAPES[shape].1);
        let locked = gallery
            .shape
            .is_some_and(|shape| gallery.shapes_locked[shape]);

        let toolbar = match gallery.shape {
            Some(shape) => MiniToolbar::new(stage.height(typography::scaled(250.0)), anchor)
                .button(
                    Icon::Target,
                    "Seçime yakınlaştır",
                    pressed("Seçime yakınlaştır"),
                )
                .button(Icon::Copy, "Kopyala", pressed("Kopyala"))
                .shortcut("Ctrl+C")
                .button(Icon::Properties, "Özellikler", pressed("Özellikler"))
                .separator()
                .button(
                    if locked { Icon::Lock } else { Icon::Unlock },
                    if locked { "Kilidi aç" } else { "Kilitle" },
                    Message::Gallery(Demo::ShapeLocked(shape)),
                )
                .active(locked)
                .separator()
                .button(
                    Icon::Eraser,
                    "Sil",
                    (!locked).then_some(Message::Gallery(Demo::ShapeDeleted(shape))),
                )
                .shortcut("Delete")
                .danger()
                .button(
                    Icon::ClearSelection,
                    "Seçimi bırak",
                    Message::Gallery(Demo::ShapeSelected(None)),
                )
                .shortcut("Esc"),
            None => MiniToolbar::new(stage.height(typography::scaled(250.0)), None),
        };

        let deleted = gallery
            .shapes_deleted
            .iter()
            .filter(|deleted| **deleted)
            .count();
        let mut sample = column![
            container(toolbar)
                .padding(1)
                .style(style::container::bordered)
        ]
        .spacing(8);

        if deleted > 0 {
            sample = sample.push(
                button(label::caption("Silinen şekilleri geri getir").style(style::text::default))
                    .on_press(Message::Gallery(Demo::ShapesReset))
                    .padding([2, 8])
                    .style(style::button::flat),
            );
        }

        sample.into()
    }

    /// Dairesel menü örneği: alana basmak menüyü imlecin yerinde açar.
    fn radial_sample(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let tool = gallery.radial_tool;

        let stage = mouse_area(
            container(
                column![
                    row![
                        icon(tool.icon()).size(16.0),
                        label::strong(format!("Etkin araç: {}", tool.label())),
                    ]
                    .spacing(8)
                    .align_y(Center),
                    label::caption(
                        "Basılı tutup bir yöne çekin ve bırakın, ya da tıklayıp yönü seçin."
                    ),
                ]
                .spacing(6),
            )
            .padding([10, 12])
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| {
                style::container::solid(model_space::Style::of(theme).background)(theme)
            }),
        )
        .on_press(Message::Gallery(Demo::RadialOpened));

        let menu = [
            Tool::Select,
            Tool::Line,
            Tool::Polyline,
            Tool::Polygon,
            Tool::Measure,
            Tool::Point,
            Tool::Circle,
            Tool::Rectangle,
        ]
        .into_iter()
        .fold(
            RadialMenu::new(
                container(stage).height(typography::scaled(300.0)),
                gallery.radial_open,
                Message::Gallery(Demo::RadialClosed),
            )
            .hold(keyboard::Key::Named(keyboard::key::Named::Space)),
            |menu, tool| {
                menu.item(
                    tool.icon(),
                    tool.label(),
                    Message::Gallery(Demo::RadialChosen(tool)),
                )
            },
        );

        container(menu)
            .padding(1)
            .style(style::container::bordered)
            .into()
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

/// Katman özetinin satırı: sönük ad, eş aralıklı değer.
fn summary_row<'a>(name: &'a str, value: &'a str) -> Element<'a, Message> {
    row![
        label::muted(name).width(typography::scaled(72.0)),
        label::mono(value),
    ]
    .spacing(8)
    .into()
}
