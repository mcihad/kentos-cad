//! Şerit: sekmeler ve Giriş sekmesinin grupları.

use iced::widget::pick_list;
use iced::{Element, Fill};

use kentos_rc::icon::Icon;
use kentos_rc::label;
use kentos_rc::spatial::Tool;
use kentos_rc::style;
use kentos_rc::widget::ribbon::{self, AppButton, Button, Field, Group, Ribbon, Stack};
use kentos_rc::widget::{Tip, swatch};

use crate::app::Showcase;
use crate::command::{self, Command};
use crate::gallery::Page;
use crate::message::{Message, QueryPurpose, RibbonTab};

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
            .group(view_group())
            .group(self.layers_group())
            .group(self.selection_group())
            .group(self.interface_group())
            .into()
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
        .text_size(12.0)
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

fn view_group<'a>() -> Group<'a, Message> {
    Group::new("Görünüm")
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
