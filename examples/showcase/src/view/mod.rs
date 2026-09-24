//! Görünüm: bileşenlerin yerleşimi.
//!
//! ```text
//! ┌ Şerit ─────────────────────────────────────────────────┐
//! ├ Model alanı ────────────────────────────┬ Yan paneller ┤
//! │                                         │ Katmanlar    │
//! │                                         │ Özellikler   │
//! ├ Öznitelik tablosu ──────────────────────┤ (nesne       │
//! ├ Komut satırı ───────────────────────────┤  inceleyici) │
//! ├ Durum çubuğu ───────────────────────────┴──────────────┤
//! ```
//!
//! Galeri sekmesinde model alanı ve yan paneller yerini bileşen kataloğuna
//! bırakır. Üst katmanlar önceliğe göre tek tek açılır: uygulama menüsü,
//! sorgu penceresi, kısayollar.

mod app_menu;
mod attribute_table;
mod dock;
mod gallery;
mod help;
mod query;
mod ribbon;
mod status;

use std::fmt;

use iced::widget::{Column, column, container, row, stack};
use iced::{Element, Fill};

use kentos_rc::icon::Icon;
use kentos_rc::spatial::{Layer, ModelSpace, ViewCube};
use kentos_rc::style;
use kentos_rc::widget::{CommandLine, NavigationBar, horizontal_divider, vertical_divider};

use crate::app::{DRAWING_LAYER, Showcase};
use crate::message::{Message, RibbonTab};

impl Showcase {
    pub fn view(&self) -> Element<'_, Message> {
        let workspace: Element<'_, Message> = if self.ribbon_tab == RibbonTab::Gallery {
            column![self.gallery(), self.command_line()]
                .height(Fill)
                .into()
        } else {
            let mut drawing = Column::new().push(self.model_space());

            if self.table_open {
                drawing = drawing
                    .push(horizontal_divider())
                    .push(self.attribute_table());
            }

            row![
                drawing.push(self.command_line()).width(Fill).height(Fill),
                vertical_divider(),
                self.dock()
            ]
            .height(Fill)
            .into()
        };

        let base = container(column![self.ribbon(), workspace, self.status_bar()])
            .width(Fill)
            .height(Fill)
            .style(style::container::window);

        let overlay = if self.app_menu_open {
            Some(self.app_menu())
        } else if let Some(dialog) = &self.query {
            Some(self.query_dialog(dialog))
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
            .selection(&self.selection)
            .hover(self.hover)
            .measurement(self.measurement.points())
            .draft(self.draft.points(), drawing_color)
            .options(self.options)
            .prompt(self.picking.as_ref().map(|pick| pick.prompt.as_str()))
            .view_cube(self.view_cube.then(|| ViewCube::new(self.cube_rotation)))
            .navigation(navigation)
            .into()
    }

    fn command_line(&self) -> Element<'_, Message> {
        CommandLine::new(&self.history, &self.command_input)
            .placeholder("Komut yazın, ör. CIZGI, SORGU, TABLO, YARDIM")
            .on_input(Message::CommandInput)
            .on_submit(Message::CommandSubmitted)
            .into()
    }

    /// Katman seçim kutularının seçenekleri.
    fn layer_choices(&self) -> Vec<LayerChoice<'_>> {
        self.layers
            .iter()
            .enumerate()
            .map(|(index, layer)| LayerChoice::new(index, layer))
            .collect()
    }
}

/// Katman seçim kutusunun seçeneği.
#[derive(Debug, Clone, PartialEq)]
struct LayerChoice<'a> {
    index: usize,
    name: &'a str,
}

impl<'a> LayerChoice<'a> {
    fn new(index: usize, layer: &'a Layer) -> Self {
        Self {
            index,
            name: &layer.name,
        }
    }
}

impl fmt::Display for LayerChoice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}
