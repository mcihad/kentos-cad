//! Şerit: sekmeler ve Giriş sekmesinin grupları.

use std::fmt;

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
use crate::message::{Message, RibbonTab};

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
        let choices: Vec<LayerChoice<'_>> = self
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| LayerChoice {
                index,
                name: &layer.name,
            })
            .collect();

        let current = choices.get(self.active_layer).cloned();
        let color = self
            .layers
            .get(self.active_layer)
            .map(|layer| layer.color)
            .unwrap_or_default();

        let picker = pick_list(choices, current, |choice: LayerChoice<'_>| {
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
        let has_selection = self.selection.is_some();

        Group::new("Seçim").push(
            Stack::new()
                .push(
                    Button::small(Icon::Target, "Seçime odakla")
                        .on_press_maybe(has_selection.then_some(Message::FocusSelection)),
                )
                .push(
                    Button::small(Icon::ClearSelection, "Seçimi kaldır")
                        .on_press_maybe(has_selection.then_some(Message::ClearSelection)),
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

/// Şeritteki katman seçim kutusunun seçeneği.
#[derive(Debug, Clone, PartialEq)]
struct LayerChoice<'a> {
    index: usize,
    name: &'a str,
}

impl fmt::Display for LayerChoice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}
