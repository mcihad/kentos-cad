//! Şerit: sekmeler ve Giriş sekmesinin grupları.

use iced::widget::{button, column, container, pick_list, row, space, text, tooltip};
use iced::{Border, Center, Color, Element, Fill, Font, Theme};

use kentos_rc::icon::Icon;
use kentos_rc::label;
use kentos_rc::spatial::Tool;
use kentos_rc::spatial::model_space::{self, Backdrop};
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::theme::typography::{Family, Mono, Typography};
use kentos_rc::theme::{Accent, Mode};
use kentos_rc::widget::ribbon::{
    self, AppButton, Button, Field, Gallery, Group, Preview, Ribbon, Stack, Tile,
};
use kentos_rc::widget::{Menu, Tip, swatch, tip};

use crate::app::Showcase;
use crate::command::{self, Command};
use crate::gallery::Page;
use crate::message::{DockPanel, EXPORT_FORMATS, Message, Pane, QueryPurpose, RibbonTab, SizeStep};
use crate::view::panes::COLORS;
use crate::view::{backdrop_note, family_note, hex_of, theme_note};

/// Katman rengi galerisindeki renklerin adları, [`COLORS`] sırasıyla.
const COLOR_NAMES: [&str; 10] = [
    "Kırmızı",
    "Turuncu",
    "Hardal",
    "Yeşil",
    "Turkuaz",
    "Mavi",
    "Petrol",
    "Mor",
    "Pembe",
    "Gri",
];

/// Çizgi kalınlığı galerisinin kalınlıkları (piksel).
const STROKE_WIDTHS: [f32; 5] = [1.0, 1.5, 2.0, 3.0, 4.0];

impl Showcase {
    pub(super) fn ribbon(&self) -> Element<'_, Message> {
        let ribbon = Ribbon::new()
            .application(
                AppButton::new("KentOS CAD")
                    .open(self.app_menu_open)
                    .on_press(Message::AppMenuToggled),
            )
            .tabs(RibbonTab::ALL, self.ribbon_tab, Message::RibbonTabSelected)
            .trailing(label::caption("Türkiye örnek verisi"))
            .collapsible(self.ribbon_collapsed, Message::RibbonCollapsed);

        // Hızlı erişim düğmeleri; ⌄ menüsü hangilerinin görüneceğini seçer.
        let quick = self.quick_commands();
        let shown = self.quick;
        let collapsed = self.ribbon_collapsed;

        let ribbon = quick
            .iter()
            .zip(shown)
            .filter(|(_, shown)| *shown)
            .fold(ribbon, |ribbon, ((glyph, name, message), _)| {
                ribbon.quick(*glyph, *name, message.clone())
            })
            .quick_menu(move || {
                quick
                    .iter()
                    .zip(shown)
                    .enumerate()
                    .fold(
                        Menu::new().header("Hızlı erişim araç çubuğu"),
                        |menu, (index, ((glyph, name, _), shown))| {
                            menu.check(*name, shown, Message::QuickToggled(index))
                                .icon(*glyph)
                        },
                    )
                    .separator()
                    .item(
                        if collapsed {
                            "Şeridi göster"
                        } else {
                            "Şeridi daralt"
                        },
                        Message::RibbonCollapsed,
                    )
                    .shortcut("Ctrl+F1")
            });

        if self.ribbon_tab == RibbonTab::Gallery {
            return ribbon
                .group(self.gallery_group("Temel", &[Page::Colors, Page::Typography, Page::Icons]))
                .group(self.gallery_group(
                    "Bileşenler",
                    &[
                        Page::Buttons,
                        Page::Data,
                        Page::Frame,
                        Page::Layout,
                        Page::Inputs,
                        Page::Feedback,
                    ],
                ))
                .group(self.gallery_group("CBS ve CAD", &[Page::Attributes, Page::Spatial]))
                .group(self.interface_group())
                .into();
        }

        if self.ribbon_tab == RibbonTab::View {
            return ribbon
                .group(view_group("Harita"))
                .group(self.sheet_group())
                .group(self.windows_group())
                .group(self.panels_group())
                .group(self.theme_group())
                .group(self.backdrop_group())
                .group(self.typeface_group())
                .group(self.mono_group())
                .group(self.size_group())
                .into();
        }

        if self.ribbon_tab == RibbonTab::Insert {
            return ribbon.group(self.insert_group()).into();
        }

        if self.ribbon_tab == RibbonTab::Annotate {
            return ribbon
                .group(self.color_gallery())
                .group(self.stroke_gallery())
                .into();
        }

        if self.ribbon_tab == RibbonTab::Manage {
            return ribbon
                .group(self.data_group())
                .group(self.export_group())
                .into();
        }

        if self.ribbon_tab != RibbonTab::Home {
            return ribbon
                .placeholder(format!(
                    "{} sekmesinde henüz araç yok. Araçlar Giriş sekmesinde.",
                    self.ribbon_tab
                ))
                .into();
        }

        ribbon
            .group(self.draw_group())
            .group(self.tools_group())
            .group(view_group("Görünüm"))
            .group(self.layers_group())
            .group(self.selection_group())
            .group(self.interface_group())
            .into()
    }

    /// Veri ekleme: içe aktarma sihirbazı ve aktif katmanın özellikleri.
    fn insert_group(&self) -> Group<'_, Message> {
        Group::new("Veri")
            .push(
                Button::large(Icon::Import, "Veri\niçe aktar")
                    .on_press(Message::ImportOpened)
                    .tip(
                        Tip::new("Veri içe aktar")
                            .body("CSV ya da GeoJSON dosyasını adım adım katman olarak ekler.")
                            .detail(format!("Komut: {}", command::name(Command::Import))),
                    ),
            )
            .push(
                Button::large(Icon::Properties, "Katman\nözellikleri")
                    .on_press(Message::PropertiesOpened(self.active_layer))
                    .tip(Tip::new("Katman özellikleri").body(
                        "Aktif katmanın adı, kaynağı, sembolizasyonu, etiketleri ve alanları.",
                    )),
            )
    }

    /// Veri bakımı: uzamsal dizin ve arka plandaki işler.
    fn data_group(&self) -> Group<'_, Message> {
        Group::new("Veri")
            .push(
                Button::large(Icon::Grid, "Dizin\noluştur")
                    .on_press(Message::IndexRequested)
                    .tip(
                        Tip::new("Uzamsal dizin oluştur").body(
                            "Katmanların dizinini yeniden kurar; seçim ve yakalama hızlanır.",
                        ),
                    ),
            )
            .push(
                Button::large(Icon::Progress, "Görevler")
                    .active(self.docks.is_shown(DockPanel::Tasks))
                    .on_press(Message::PanelToggled(DockPanel::Tasks))
                    .tip(Tip::new("Görevler").body(DockPanel::Tasks.description())),
            )
    }

    /// Bütün katmanları dosyaya yazar; iş arka planda sürer. Bölünmüş
    /// düğme: üst kısım GeoJSON'a aktarır, alt kısım biçimleri açar.
    fn export_group(&self) -> Group<'_, Message> {
        Group::new("Dışa aktar").push(
            Button::large(Icon::Export, "Dışa aktar")
                .on_press(Message::ExportPressed("GeoJSON"))
                .menu(|| {
                    EXPORT_FORMATS.iter().fold(
                        Menu::new().header("Biçim"),
                        |menu, &(format, description)| {
                            menu.item(
                                format!("{format}: {description}"),
                                Message::ExportPressed(format),
                            )
                            .icon(Icon::Export)
                        },
                    )
                })
                .tip(
                    Tip::new("GeoJSON olarak dışa aktar")
                        .body("Başka biçimler için alttaki oka basın: PDF, PNG, DXF."),
                ),
        )
    }

    /// Hızlı erişim komutları: ikon, ad ve (etkinse) mesaj.
    fn quick_commands(&self) -> [(Icon, &'static str, Option<Message>); 4] {
        [
            (
                Icon::Export,
                "GeoJSON olarak dışa aktar",
                Some(Message::ExportPressed("GeoJSON")),
            ),
            (
                Icon::Undo,
                "Silineni geri al",
                self.can_undo().then_some(Message::UndoDelete),
            ),
            (Icon::ZoomExtents, "Tümünü gör", Some(Message::FitAll)),
            (Icon::Help, "Kısayollar (F1)", Some(Message::HelpToggled)),
        ]
    }

    /// Aktif katmanın rengi: hazır renklerin galerisi.
    fn color_gallery(&self) -> Group<'_, Message> {
        let index = self.active_layer;
        let current = self.layers.get(index).map(|layer| layer.color);
        let selected = COLORS.iter().position(|color| Some(*color) == current);

        Group::new("Katman rengi").push(
            Gallery::new(
                COLORS
                    .iter()
                    .zip(COLOR_NAMES)
                    .map(|(color, name)| Tile::new(name, Preview::Color(*color))),
                selected,
                move |choice| Message::LayerColor(index, COLORS[choice]),
            )
            .columns(5),
        )
    }

    /// Aktif katmanın çizgi kalınlığı: örnek çizgilerin galerisi.
    fn stroke_gallery(&self) -> Group<'_, Message> {
        let index = self.active_layer;
        let layer = self.layers.get(index);
        let color = layer.map_or(iced::Color::WHITE, |layer| layer.color);
        let selected = layer.and_then(|layer| {
            STROKE_WIDTHS
                .iter()
                .position(|width| (width - layer.stroke_width).abs() < 0.05)
        });

        Group::new("Çizgi kalınlığı").push(Gallery::new(
            STROKE_WIDTHS
                .iter()
                .map(|width| Tile::new(format!("{width} px"), Preview::Line(color, *width))),
            selected,
            move |choice| Message::LayerStroke(index, STROKE_WIDTHS[choice]),
        ))
    }

    /// Harita üstündeki kayan pencereler; açık olan vurgulanır. AutoCAD'in
    /// Görünüm sekmesindeki "Paletler" grubu gibi.
    /// Düzen sekmelerinin cetvelleri ve kılavuzları.
    fn sheet_group(&self) -> Group<'_, Message> {
        let guides = self.sheets.sheet().map_or(0, |sheet| sheet.guides.len());

        Group::new("Pafta").push(
            Stack::new()
                .push(
                    Button::small(Icon::Ruler, "Cetveller")
                        .active(self.rulers)
                        .on_press(Message::RulersToggled)
                        .tip(
                            Tip::new("Cetveller")
                                .body(
                                    "Düzende kâğıdın üstünde ve solunda milimetre cetvelleri. \
                                     Cetvelden sürükleyerek kılavuz çıkarın; kılavuzu cetvele \
                                     geri bırakmak siler.",
                                )
                                .detail("Ctrl+R"),
                        ),
                )
                .push(
                    Button::small(Icon::Eraser, "Kılavuzları sil")
                        .on_press_maybe((guides > 0).then_some(Message::GuidesCleared))
                        .tip(Tip::new("Kılavuzları sil").body(if guides > 0 {
                            format!("Açık düzendeki {guides} kılavuz silinir.")
                        } else {
                            "Açık düzende kılavuz yok.".to_owned()
                        })),
                ),
        )
    }

    fn windows_group(&self) -> Group<'_, Message> {
        let pane = |pane: Pane, description: &'static str, command: Command| {
            Button::small(pane.icon(), pane.title())
                .active(self.windows.is_open(pane))
                .on_press(Message::PaneToggled(pane))
                .tip(
                    Tip::new(pane.title())
                        .body(description)
                        .detail(format!("Komut: {}", command::name(command))),
                )
        };

        Group::new("Pencereler").push(
            Stack::new()
                .push(pane(
                    Pane::Measure,
                    "Ölç aracını ve ölçüm penceresini açar.",
                    Command::Tool(Tool::Measure),
                ))
                .push(pane(
                    Pane::GoTo,
                    "Enlem ve boylam yazıp görünümü ortalar ya da çizime nokta ekler.",
                    Command::Pane(Pane::GoTo),
                ))
                .push(pane(
                    Pane::Style,
                    "Aktif katmanın rengini, opaklığını ve çizgi kalınlığını değiştirir.",
                    Command::Pane(Pane::Style),
                )),
        )
    }

    /// Yuvadaki paneller; görünen vurgulanır. Kapatılan panel yeniden
    /// açılınca kapatıldığı kenara döner.
    fn panels_group(&self) -> Group<'_, Message> {
        let panel = |panel: DockPanel| {
            Button::small(panel.icon(), panel.title())
                .active(self.docks.is_shown(panel))
                .on_press(Message::PanelToggled(panel))
                .tip(Tip::new(panel.title()).body(panel.description()))
        };

        Group::new("Paneller")
            .push(
                Stack::new()
                    .push(panel(DockPanel::Layers))
                    .push(panel(DockPanel::Details)),
            )
            .push(
                Stack::new()
                    .push(panel(DockPanel::Table))
                    .push(panel(DockPanel::Tasks)),
            )
    }

    fn draw_group(&self) -> Group<'_, Message> {
        Group::new("Çizim")
            .push(self.tool_button(Tool::Line, true))
            .push(self.tool_button(Tool::Polyline, true))
            .push(self.tool_button(Tool::Circle, true))
            .push(
                Stack::new()
                    .push(self.tool_button(Tool::Rectangle, false))
                    .push(self.tool_button(Tool::Polygon, false))
                    .push(self.tool_button(Tool::Point, false)),
            )
    }

    fn tools_group(&self) -> Group<'_, Message> {
        Group::new("Araçlar").push(
            Tool::NAVIGATION
                .into_iter()
                .fold(Stack::new(), |stack, tool| {
                    stack.push(self.tool_button(tool, false))
                }),
        )
    }

    fn layers_group(&self) -> Group<'_, Message> {
        let choices = self.layer_choices();

        let current = choices.get(self.active_layer).cloned();
        let color = self
            .layers
            .get(self.active_layer)
            .map(|layer| layer.color)
            .unwrap_or_default();

        let picker = pick_list(choices, current, |choice: super::LayerChoice<'_>| {
            Message::LayerActivated(choice.index)
        })
        .font(typography::ui())
        .text_size(typography::body())
        .padding([2, 8])
        .width(Fill)
        .style(style::field::pick_list)
        .menu_style(style::field::menu);

        Group::new("Katmanlar").push(
            Stack::new()
                .width(252)
                .push(Field::new(swatch(color), picker))
                .push(
                    ribbon::Row::new()
                        .push(Button::small(Icon::Eye, "Göster").on_press(Message::ShowAllLayers))
                        .push(Button::small(Icon::EyeOff, "Gizle").on_press(Message::HideAllLayers))
                        .push(
                            Button::small(Icon::Target, "Sığdır")
                                .on_press(Message::ZoomToLayer(self.active_layer)),
                        ),
                ),
        )
    }

    fn selection_group(&self) -> Group<'_, Message> {
        let has_selection = !self.selection.is_empty();

        Group::new("Seçim")
            .push(
                Button::large(Icon::Filter, "Öznitelikle\nseç")
                    .on_press(Message::QueryOpened(QueryPurpose::Select))
                    .tip(
                        Tip::new("Öznitelikle seç")
                            .body("Koşullara uyan öğeleri seçer; seçime ekler ya da çıkarır.")
                            .detail(format!(
                                "Komut: {}",
                                command::name(Command::SelectByAttributes)
                            )),
                    ),
            )
            .push(
                Stack::new()
                    .push(Button::small(Icon::SelectAll, "Tümünü seç").on_press(Message::SelectAll))
                    .push(
                        Button::small(Icon::InvertSelection, "Tersine çevir")
                            .on_press(Message::InvertSelection),
                    )
                    .push(
                        Button::small(Icon::ClearSelection, "Seçimi kaldır")
                            .on_press_maybe(has_selection.then_some(Message::ClearSelection)),
                    ),
            )
            .push(
                Stack::new()
                    .push(
                        Button::small(Icon::Table, "Öznitelik tablosu")
                            .active(self.docks.is_shown(DockPanel::Table))
                            .on_press(Message::TableToggled),
                    )
                    .push(
                        Button::small(Icon::Target, "Seçime odakla")
                            .on_press_maybe(has_selection.then_some(Message::FocusSelection)),
                    )
                    .push(
                        Button::small(Icon::Eraser, "Ölçümü temizle").on_press_maybe(
                            (!self.measurement.is_empty()).then_some(Message::ClearMeasurement),
                        ),
                    ),
            )
    }

    fn interface_group(&self) -> Group<'_, Message> {
        let theme_label = if self.mode.is_dark() {
            "Aydınlık\ntema"
        } else {
            "Koyu\ntema"
        };

        Group::new("Arayüz")
            .push(Button::large(Icon::Contrast, theme_label).on_press(Message::ToggleTheme))
            .push(
                Stack::new()
                    .push(Button::small(Icon::Help, "Kısayollar").on_press(Message::HelpToggled))
                    .push(
                        Button::small(Icon::Terminal, "Komut listesi")
                            .on_press(Message::CommandListRequested),
                    )
                    .push(
                        Button::small(Icon::Type, "Yazı ve boyut")
                            .on_press(Message::RibbonTabSelected(RibbonTab::View))
                            .tip(
                                Tip::new("Yazı ve boyut")
                                    .body("Yazı ailesi ve boyutu Görünüm sekmesinde seçilir.")
                                    .detail("Komut: YAZITIPI, PUNTO"),
                            ),
                    ),
            )
    }

    /// Tema ve vurgu rengi: dört temanın önizleme karoları; sekiz hazır
    /// renk, altında kendi rengini yazdıran düğme.
    fn theme_group(&self) -> Group<'_, Message> {
        let tile = |mode: Mode| theme_tile(mode, self.accent, self.mode == mode);

        let themes = Stack::new()
            .width(78)
            .push(
                ribbon::Row::new()
                    .push(tile(Mode::Dark))
                    .push(tile(Mode::Light)),
            )
            .push(
                ribbon::Row::new()
                    .push(tile(Mode::Night))
                    .push(tile(Mode::HighContrast)),
            )
            // Dar sütunda kısa ad: "Yüksek karşıtlık" iki satıra bölünmesin.
            .push(label::caption(match self.mode {
                Mode::HighContrast => "Karşıtlık",
                mode => mode.label(),
            }));

        let chips = |presets: &[Accent]| {
            presets.iter().fold(ribbon::Row::new(), |row, &accent| {
                row.push(accent_chip(accent, self.mode, self.accent == accent))
            })
        };

        let custom = match self.accent {
            Accent::Custom(_) => Button::small(Icon::Drop, self.accent.name()).active(true),
            _ => Button::small(Icon::Drop, "Özel renk…"),
        }
        .on_press(Message::CommandRun(command::name(Command::Accent).to_owned()))
        .tip(
            Tip::new("Özel vurgu rengi")
                .body("Komut kutusuna #RRGGBB yazın; renk temanın zemininde okunur kalacak kadar ayarlanır.")
                .detail(format!("Komut: {}", command::name(Command::Accent))),
        );

        Group::new("Tema").push(themes).push(
            Stack::new()
                .push(chips(&Accent::PRESETS[..4]))
                .push(chips(&Accent::PRESETS[4..]))
                .push(custom),
        )
    }

    /// Harita zemini: arayüzün temasından bağımsız; önizleme karoları.
    fn backdrop_group(&self) -> Group<'_, Message> {
        let tile = |backdrop: Backdrop| backdrop_tile(backdrop, self.backdrop == backdrop);

        // Karolar sabit boyuttadır; hücreler yığının genişliğini paylaşır.
        Group::new("Harita zemini").push(
            Stack::new()
                .width(78)
                .push(
                    ribbon::Row::new()
                        .push(tile(Backdrop::Theme))
                        .push(tile(Backdrop::Slate)),
                )
                .push(
                    ribbon::Row::new()
                        .push(tile(Backdrop::Black))
                        .push(tile(Backdrop::Paper)),
                )
                .push(label::caption(self.backdrop.name())),
        )
    }

    /// Arayüz metninin ailesi: her karo "Aa" örneğini kendi ailesiyle yazar.
    fn typeface_group(&self) -> Group<'_, Message> {
        let current = self.typography;

        Family::ALL
            .into_iter()
            .fold(Group::new("Yazı tipi"), |group, family| {
                let typography = Typography { family, ..current };

                group.push(typeface_tile(
                    "Aa",
                    family.name(),
                    family_note(family),
                    typography.ui(),
                    current.family == family,
                    Message::TypographyChanged(typography),
                ))
            })
    }

    /// Koordinat, ölçü ve komutların eş aralıklı ailesi.
    fn mono_group(&self) -> Group<'_, Message> {
        let current = self.typography;

        Mono::ALL
            .into_iter()
            .fold(Group::new("Eş aralıklı"), |group, mono| {
                let typography = Typography { mono, ..current };

                group.push(typeface_tile(
                    "41°",
                    mono.name(),
                    "Koordinat, ölçü ve komutların yazısı.",
                    typography.mono(),
                    current.mono == mono,
                    Message::TypographyChanged(typography),
                ))
            })
    }

    /// Gövde metninin boyutu ve adım düğmeleri.
    fn size_group(&self) -> Group<'_, Message> {
        let size = self.typography.size;
        let (smallest, largest) = (*Typography::SIZES.start(), *Typography::SIZES.end());

        let value = container(
            column![
                text(format!("{size}"))
                    .font(typography::ui_strong())
                    .size(typography::scaled(24.0)),
                label::caption("piksel"),
            ]
            .spacing(2)
            .align_x(Center),
        )
        .width(typography::scaled(56.0))
        .height(ribbon::content_height())
        .center_x(typography::scaled(56.0))
        .center_y(ribbon::content_height());

        Group::new("Boyut").push(value).push(
            Stack::new()
                .push(
                    Button::small(Icon::Plus, "Büyüt")
                        .on_press_maybe(
                            (size < largest).then_some(Message::TextSize(SizeStep::Larger)),
                        )
                        .tip(Tip::new("Yazıyı büyüt").detail("Ctrl +")),
                )
                .push(
                    Button::small(Icon::Minus, "Küçült")
                        .on_press_maybe(
                            (size > smallest).then_some(Message::TextSize(SizeStep::Smaller)),
                        )
                        .tip(Tip::new("Yazıyı küçült").detail("Ctrl −")),
                )
                .push(
                    Button::small(Icon::Undo, "Varsayılan")
                        .on_press_maybe(
                            (size != Typography::DEFAULT.size)
                                .then_some(Message::TextSize(SizeStep::Default)),
                        )
                        .tip(
                            Tip::new("Varsayılan boyut")
                                .body(format!("{} piksel", Typography::DEFAULT.size))
                                .detail("Ctrl 0"),
                        ),
                ),
        )
    }

    /// Galeri sayfalarına götüren büyük düğmeler; açık sayfa vurgulanır.
    fn gallery_group(&self, title: &'static str, pages: &[Page]) -> Group<'_, Message> {
        pages.iter().fold(Group::new(title), |group, &page| {
            group.push(
                Button::large(page.icon(), page.label())
                    .active(self.gallery.page == page)
                    .on_press(Message::GalleryPageSelected(page))
                    .tip(Tip::new(page.label()).body(page.description())),
            )
        })
    }

    /// Araç düğmesi: etkin araç vurgulanır; ipucu aracın ne yaptığını ve
    /// komut satırı karşılığını gösterir.
    fn tool_button(&self, tool: Tool, large: bool) -> Button<'static, Message> {
        let button = if large {
            Button::large(tool.icon(), large_label(tool))
        } else {
            Button::small(tool.icon(), tool.label())
        };

        button
            .active(self.tool == tool)
            .on_press(Message::ToolSelected(tool))
            .tip(
                Tip::new(tool.label())
                    .body(tool.description())
                    .detail(format!("Komut: {}", command::name(Command::Tool(tool)))),
            )
    }
}

fn view_group<'a>(title: &'a str) -> Group<'a, Message> {
    Group::new(title)
        .push(Button::large(Icon::ZoomExtents, "Tümünü\ngör").on_press(Message::FitAll))
        .push(
            Stack::new()
                .push(Button::small(Icon::ZoomIn, "Yakınlaştır").on_press(Message::ZoomIn))
                .push(Button::small(Icon::ZoomOut, "Uzaklaştır").on_press(Message::ZoomOut))
                .push(Button::small(Icon::Home, "Sıfırla").on_press(Message::ResetView)),
        )
}

/// Tema karosu: temanın küçük bir penceresi; üstte pencere şeridi, gövdede
/// yazı çizgisi ve vurgu noktası, kenar temanın kenar renginde.
fn theme_tile<'a>(mode: Mode, accent: Accent, selected: bool) -> Element<'a, Message> {
    let tokens = mode.tokens(accent);

    let block = |color: Color, width: f32, height: f32, radius: f32| {
        container(space::horizontal())
            .width(width)
            .height(height)
            .style(move |_: &Theme| container::Style {
                background: Some(color.into()),
                border: iced::border::rounded(radius),
                ..container::Style::default()
            })
    };

    let body = container(
        row![
            block(tokens.text, 10.0, 2.0, 1.0),
            space::horizontal(),
            block(tokens.accent, 5.0, 5.0, 2.5),
        ]
        .align_y(Center),
    )
    .padding([0, 3])
    .width(Fill)
    .center_y(11)
    .style(move |_: &Theme| container::Style {
        background: Some(tokens.surface.into()),
        ..container::Style::default()
    });

    let preview = container(column![block(tokens.window, 26.0, 4.0, 0.0), body])
        .width(28)
        .padding(1)
        .style(move |_: &Theme| container::Style {
            background: Some(tokens.window.into()),
            border: Border {
                color: tokens.border,
                width: 1.0,
                radius: 2.0.into(),
            },
            ..container::Style::default()
        });

    tip(
        button(preview)
            .on_press(Message::ThemeSelected(mode))
            .padding(2)
            .style(style::button::swatch(selected)),
        Tip::new(mode.label()).body(theme_note(mode)),
        tooltip::Position::Bottom,
    )
}

/// Vurgu rengi düğmesi: yuvarlak renk örneği; seçili renk vurgu halkasıyla.
fn accent_chip<'a>(accent: Accent, mode: Mode, selected: bool) -> Element<'a, Message> {
    let color = accent.color(mode);

    let chip = container(space::horizontal())
        .width(14)
        .height(14)
        .style(move |_: &Theme| container::Style {
            background: Some(color.into()),
            border: Border {
                color: Color::BLACK.scale_alpha(0.3),
                width: 1.0,
                radius: 7.0.into(),
            },
            ..container::Style::default()
        });

    tip(
        button(chip)
            .on_press(Message::AccentChanged(accent))
            .padding(2)
            .style(move |theme, status| {
                let mut style = style::button::swatch(selected)(theme, status);
                style.border.radius = 10.0.into();
                style
            }),
        Tip::new(accent.name()).body(format!(
            "Koyu temada {}, aydınlık temada {}.",
            hex_of(accent.color(Mode::Dark)),
            hex_of(accent.color(Mode::Light))
        )),
        tooltip::Position::Bottom,
    )
}

/// Harita zemini karosu: zeminin rengi; "Temaya uy" yarı arduvaz yarı kâğıt.
fn backdrop_tile<'a>(backdrop: Backdrop, selected: bool) -> Element<'a, Message> {
    let fill = |color: Color, width: f32| {
        container(space::horizontal())
            .width(width)
            .height(16)
            .style(move |_: &Theme| container::Style {
                background: Some(color.into()),
                ..container::Style::default()
            })
    };

    let preview: Element<'a, Message> = match backdrop {
        Backdrop::Theme => row![
            fill(model_space::Style::DARK.background, 13.0),
            fill(model_space::Style::LIGHT.background, 13.0),
        ]
        .into(),
        Backdrop::Slate => fill(model_space::Style::DARK.background, 26.0).into(),
        Backdrop::Black => fill(model_space::Style::BLACK.background, 26.0).into(),
        Backdrop::Paper => fill(model_space::Style::LIGHT.background, 26.0).into(),
    };

    let framed = container(preview)
        .padding(1)
        .style(|theme: &Theme| container::Style {
            border: Border {
                color: kentos_rc::theme::Tokens::of(theme).border,
                width: 1.0,
                radius: 2.0.into(),
            },
            ..container::Style::default()
        });

    tip(
        button(framed)
            .on_press(Message::BackdropChanged(backdrop))
            .padding(2)
            .style(style::button::swatch(selected)),
        Tip::new(backdrop.name())
            .body(backdrop_note(backdrop))
            .detail(format!("Komut: {}", command::name(Command::Backdrop))),
        tooltip::Position::Bottom,
    )
}

/// Büyük düğmelerde uzun etiketler iki satıra bölünür.
fn large_label(tool: Tool) -> &'static str {
    match tool {
        Tool::Polyline => "Çoklu\nçizgi",
        other => other.label(),
    }
}

/// Yazı ailesi karosu: örnek metin kendi ailesiyle, adı arayüz yazısıyla.
fn typeface_tile<'a>(
    sample: &'a str,
    name: &'static str,
    note: &'static str,
    font: Font,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let tile = button(
        column![
            text(sample)
                .font(font)
                .size(typography::scaled(22.0))
                .line_height(1.0),
            label::caption(name)
                .style(style::text::default)
                .align_x(Center)
                .width(Fill),
        ]
        .spacing(5)
        .align_x(Center)
        .width(Fill),
    )
    .on_press(on_press)
    .width(typography::scaled(84.0))
    .height(ribbon::content_height())
    .padding([7, 4])
    .style(style::button::tool(active));

    tip(tile, Tip::new(name).body(note), tooltip::Position::Bottom)
}
