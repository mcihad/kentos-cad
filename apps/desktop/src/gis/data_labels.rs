//! Veri katmanlarının etiketleri (docs/adr/9101 §6, plan/veri.md VY-03): a data layer whose labels are on keeps its
//! drawn read's objects in a document of their own (data_view.rs, up to `LABEL_ROWS`). Here that document gets its
//! store, on the interface's thread (the store keeps the label engine's memory, which stays on it), and the label engine
//! places its labels as it places the drawing's: the same classes, templates, sizes and halos. Each layer's labels lie
//! under the drawing's own and are placed apart from them; a layer hidden, removed or read again without its labels
//! (the view too wide) loses them.

use iced::Element;
use kentos_interaction::Spatial;

use crate::app::{App, Message};

/// A data layer's labels: its drawn read's objects, their store and the label layer's memory.
pub struct DataLabels {
    pub layer: String,
    pub read: u64,
    doc: Box<kentos_domain::Document>,
    spatial: Spatial,
    spots: crate::labels::Spots,
}

impl App {
    /// After the data layers' reads said something: the labels of the reads drawn now, the others let go.
    pub(crate) fn sync_data_labels(&mut self) {
        let view = crate::data_view::service();
        for (layer, read, doc) in view.take_labels() {
            self.data_labels.retain(|d| d.layer != layer);
            let spatial = Spatial::of(&doc);
            self.data_labels.push(DataLabels {
                layer,
                read,
                doc,
                spatial,
                spots: crate::labels::Spots::default(),
            });
        }
        self.data_labels
            .retain(|d| view.drawn_read(&d.layer) == Some(d.read));
    }

    /// The data layers' labels over the drawing area, each of its layer's drawn read.
    pub(crate) fn data_label_layers(&self, format: &kentos_interaction::Format) -> Vec<Element<'_, Message>> {
        if self.data_labels.is_empty() {
            return Vec::new();
        }
        let view = crate::data_view::service();
        let palette = crate::viewport::palette(self.canvas());
        self.data_labels
            .iter()
            .filter(|d| view.drawn_read(&d.layer) == Some(d.read))
            .map(|d| {
                crate::labels::data_layer(
                    &d.doc,
                    d.read,
                    &d.spatial,
                    &d.spots,
                    &self.viewport.camera,
                    self.canvas(),
                    &palette,
                    format,
                    self.color_mode(),
                    self.label_size(),
                    self.label_engine(),
                )
            })
            .collect()
    }
}
