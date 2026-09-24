//! Şerit: sekmeler ve Giriş sekmesinin grupları.

use iced::widget::{button, column, container, pick_list, text, tooltip};
use iced::{Center, Element, Fill, Font};

use kentos_rc::icon::Icon;
use kentos_rc::label;
use kentos_rc::spatial::Tool;
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::theme::typography::{Family, Mono, Typography};
use kentos_rc::widget::ribbon::{self, AppButton, Button, Field, Group, Ribbon, Stack};
use kentos_rc::widget::{Tip, swatch, tip};

use crate::app::Showcase;
use crate::command::{self, Command};
use crate::gallery::Page;
use crate::message::{Message, Pane, QueryPurpose, RibbonTab, SizeStep};
use crate::view::family_note;

impl Showcase {
    pub(super) fn ribbon(&self) -> Element<'_, Message> {
        let ribbon = Ribbon::new()
            .application(
                AppButton::new("KentOS CAD")
                    .open(self.app_menu_open)
                    .on_press(Message::AppMenuToggled),
            )
            .tabs(RibbonTab::ALL, self.ribbon_tab, Message::RibbonTabSelected)
            .trailing(label::caption("Türkiye örnek verisi"));

        if self.ribbon_tab == RibbonTab::Gallery {
            return ribbon
                .group(self.gallery_group("Temel", &[Page::Colors, Page::Typography, Page::Icons]))
                .group(self.gallery_group("Bileşenler", &[Page::Buttons, Page::Data, Page::Frame]))
                .group(self.gallery_group("CBS ve CAD", &[Page::Attributes, Page::Spatial]))
                .group(self.interface_group())
                .into();
        }

        if self.ribbon_tab == RibbonTab::View {
            return ribbon
                .group(view_group("Harita"))
                .group(self.windows_group())
                .group(self.theme_group())
                .group(self.typeface_group())
                .group(self.mono_group())
                .group(self.size_group())
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

    /// Harita üstündeki kayan pencereler; açık olan vurgulanır. AutoCAD'in
    /// Görünüm sekmesindeki "Paletler" grubu gibi.
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
                            .active(self.table_open)
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

    fn theme_group(&self) -> Group<'_, Message> {
        let (icon, label) = if self.mode.is_dark() {
            (Icon::Contrast, "Aydınlık\ntema")
        } else {
            (Icon::Contrast, "Koyu\ntema")
        };

        Group::new("Tema").push(Button::large(icon, label).on_press(Message::ToggleTheme))
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
