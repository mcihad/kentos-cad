//! Görünüm: bileşenlerin yerleşimi.
//!
//! ```text
//! ┌ Şerit ──────────────────────────────────────────────┐
//! ├ Model alanı ──────────────────────────┬ Yan paneller ┤
//! │                                       │ Katmanlar    │
//! │                                       │ Özellikler   │
//! ├ Komut satırı ─────────────────────────┤ Öğe tablosu  │
//! ├ Durum çubuğu ─────────────────────────┴──────────────┤
//! ```

mod app_menu;
mod dock;
mod help;
mod ribbon;
mod status;

use iced::widget::{column, container, row, stack};
use iced::{Element, Fill};

use kentos_rc::icon::Icon;
use kentos_rc::spatial::{ModelSpace, ViewCube};
use kentos_rc::style;
use kentos_rc::widget::{CommandLine, NavigationBar, vertical_divider};

use crate::app::{DRAWING_LAYER, Showcase};
use crate::message::Message;

impl Showcase {
    pub fn view(&self) -> Element<'_, Message> {
        let drawing = column![self.model_space(), self.command_line()]
            .width(Fill)
            .height(Fill);

        let workspace = row![drawing, vertical_divider(), self.dock()].height(Fill);

        let base = container(column![self.ribbon(), workspace, self.status_bar()])
            .width(Fill)
            .height(Fill)
            .style(style::container::window);

        let overlay = if self.app_menu_open {
            Some(self.app_menu())
        } else if self.help_open {
            Some(self.help())
        } else {
            None
        };

        match overlay {
            Some(overlay) => stack![base, overlay].into(),
            None => base.into(),
        }
    }

    fn model_space(&self) -> Element<'_, Message> {
        let drawing_color = self
            .layers
            .get(DRAWING_LAYER)
            .map(|layer| layer.color)
            .unwrap_or_default();

        let navigation = NavigationBar::new()
            .button(Icon::ZoomIn, "Yakınlaştır", Message::ZoomIn)
            .button(Icon::ZoomOut, "Uzaklaştır", Message::ZoomOut)
            .separator()
            .button(Icon::ZoomExtents, "Tümünü gör", Message::FitAll)
            .button(
                Icon::Target,
                "Aktif katmana sığdır",
                Message::ZoomToLayer(self.active_layer),
            )
            .button(Icon::Home, "Başlangıç görünümü", Message::ResetView);

        ModelSpace::new(self.viewport, &self.layers, Message::ModelSpace)
            .tool(self.tool)
            .selection(self.selection)
            .hover(self.hover)
            .measurement(self.measurement.points())
            .draft(self.draft.points(), drawing_color)
            .options(self.options)
            .view_cube(self.view_cube.then(|| ViewCube::new(self.cube_rotation)))
            .navigation(navigation)
            .into()
    }

    fn command_line(&self) -> Element<'_, Message> {
        CommandLine::new(&self.history, &self.command_input)
            .placeholder("Komut yazın, ör. CIZGI, TUMUNU, OLC, YARDIM")
            .on_input(Message::CommandInput)
            .on_submit(Message::CommandSubmitted)
            .into()
    }
}
