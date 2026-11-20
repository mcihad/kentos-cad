//! Seçili katmanlarda önle's layers (gis-usage-report GIS-050, docs/adr/1107): chosen from the ribbon with no layer
//! yet, the mode asks for them at once, in a window over the drawing — each layer of the drawing with a tick, the
//! chosen ones counted — rather than going back to the drawing with nothing to avoid. The Çakışma cell's right-click
//! menu still ticks them one by one (docs/adr/0162 §1).

use iced::widget::{Column, scrollable};
use iced::{Element, Fill};
use kentos_ui::label;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::tree_view::{Check, check_box};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

/// The window's title.
pub const TITLE: &str = "Seçili katmanlarda önle";

impl App {
    /// Seçili katmanlarda önle from the ribbon: the mode on, and the layers asked when none is chosen yet.
    pub(crate) fn choose_overlap_layers(&mut self) {
        self.choose_overlap(kentos_interaction::Overlap::Layers);
        let held = |app: &App| {
            app.document
                .as_ref()
                .is_some_and(|d| app.overlap_layers.iter().any(|l| d.model.layers().get(l).is_some()))
        };
        if self.document.is_some() && !held(self) {
            self.dialog = Some(Dialog::OverlapLayers);
        }
    }

    /// The layers, each with its tick; the count and what the mode does, said over them.
    pub(crate) fn overlap_layers_view(&self) -> Element<'_, Message> {
        let mut list = Column::new().spacing(4);
        if let Some(doc) = &self.document {
            let layers = doc.model.layers();
            for node in layers.leaves() {
                let on = self.overlap_layers.contains(&node.id);
                list = list.push(
                    iced::widget::row![
                        check_box(
                            if on { Check::Checked } else { Check::Unchecked },
                            Some(Message::OverlapLayer(node.id.clone()))
                        ),
                        label::body(layers.path(&node.id)),
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                );
            }
        }
        let chosen = self
            .document
            .as_ref()
            .map_or(0, |d| self.overlap_layers.iter().filter(|l| d.model.layers().get(l).is_some()).count());
        let said = if chosen == 0 {
            words::text_line(
                Kind::Warn,
                "Katman seçilmedi: yeni alanlar hiçbir katmanla çakışma denetiminden geçmez. Önlenecek katmanları işaretleyin.",
            )
        } else {
            words::text_line(
                Kind::Ok,
                format!("{chosen} katman seçili: yeni bir alan bu katmanların alanlarıyla çakışan yerinden kırpılır."),
            )
        };
        let body = Column::new()
            .spacing(10)
            .push(said)
            .push(scrollable(list).height(iced::Length::Shrink).width(Fill));
        Frame::new(TITLE)
            .push(body)
            .action(words::primary("Tamam", Some(Message::OverlapLayersDone)))
            .width(460.0)
            .modal(Message::OverlapLayersDone)
    }
}
