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
    /// Its labels are on (else the read is kept for the snap alone).
    labelled: bool,
}

impl DataLabels {
    /// The read rows' document the labels are placed on: Denetle counts them there (docs/adr/9186).
    pub(crate) fn doc_mut(&mut self) -> &mut kentos_domain::Document {
        &mut self.doc
    }

    /// A layer's labels on `doc`, as a read of the view would hold them (the tests of Denetle on a data layer).
    #[cfg(test)]
    pub(crate) fn of(layer: &str, doc: kentos_domain::Document) -> Self {
        let spatial = Spatial::of(&doc);
        Self {
            layer: layer.to_owned(),
            read: 0,
            doc: Box::new(doc),
            spatial,
            spots: crate::labels::Spots::default(),
            labelled: true,
        }
    }
}

impl App {
    /// After the data layers' reads said something: the labels of the reads drawn now, the others let go.
    pub(crate) fn sync_data_labels(&mut self) {
        let view = crate::data_view::service();
        for (layer, read, doc) in view.take_labels() {
            self.gis.data_labels.retain(|d| d.layer != layer);
            let spatial = Spatial::of(&doc);
            let labelled = self
                .document
                .as_ref()
                .and_then(|d| d.model.layers().get(&layer))
                .is_some_and(crate::data_view::labelled);
            self.gis.data_labels.push(DataLabels {
                layer,
                read,
                doc,
                spatial,
                spots: crate::labels::Spots::default(),
                labelled,
            });
        }
        self.gis.data_labels
            .retain(|d| view.drawn_read(&d.layer) == Some(d.read));
    }

    /// The data layers' labels over the drawing area, each of its layer's drawn read.
    pub(crate) fn data_label_layers(&self, format: &kentos_interaction::Format) -> Vec<Element<'_, Message>> {
        if self.gis.data_labels.is_empty() {
            return Vec::new();
        }
        let view = crate::data_view::service();
        let palette = crate::viewport::palette(self.canvas());
        self.gis.data_labels
            .iter()
            .filter(|d| d.labelled && view.drawn_read(&d.layer) == Some(d.read))
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

    /// The snap to the rows of the data layers being edited under the cursor (docs/adr/9103 §5): their drawn read's
    /// objects, whole, snapped as the drawing's are; the nearest point. A snap to no object of the drawing.
    pub(crate) fn data_snap(
        &self,
        at: kentos_interaction::Vec2,
        view: &dyn kentos_interaction::View,
        draft: &kentos_interaction::Draft,
    ) -> Option<kentos_geometry_core::store::snap::SnapHit> {
        let doc = self.document.as_ref()?;
        let shown = crate::data_view::service();
        self.gis.data_labels
            .iter()
            .filter(|d| doc.model.editing(&d.layer) && shown.drawn_read(&d.layer) == Some(d.read))
            .filter_map(|d| self.session.snap(&d.spatial, at, view, draft, &self.tracking))
            .map(|mut hit| {
                hit.id = kentos_geometry_core::store::snap::NO_OBJECT;
                hit
            })
            .min_by(|a, b| {
                let da = (a.point.x - at.x).hypot(a.point.y - at.y);
                let db = (b.point.x - at.x).hypot(b.point.y - at.y);
                da.total_cmp(&db)
            })
    }
}
