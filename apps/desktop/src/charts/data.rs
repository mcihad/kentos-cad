//! A layer's data for the İstatistik and Grafik tabs (docs/adr/0227 §2; the
//! web's `ui/charts/layerData.ts`): its objects in the drawing's order (those
//! its filter keeps, as the Tablo tab shows them), their columns and their
//! geometry's measures, read into the core's work once. Rows are the
//! objects' places; the tabs ask by rows and select by slots.

use std::collections::HashMap;

use kentos_contracts::{Entity, LayerNodeType};
use kentos_domain::Slot;
use kentos_geometry_core::ops::charts::Work;
use kentos_geometry_core::ops::charts::data::DataSet;
use kentos_interaction::chart_data::layer_columns;

use crate::app::App;
use crate::document::Document;
use crate::features::Show;

/// A layer's data in the core.
pub(crate) struct LayerData {
    pub slots: Vec<Slot>,
    pub work: Work,
    at: HashMap<Slot, u32>,
}

impl LayerData {
    pub fn of(app: &App, doc: &Document, layer: &str) -> LayerData {
        let model = &doc.model;
        let entities: Vec<&Entity> = model
            .by_layer(layer)
            .filter(|e| app.spatial.filter_shown(Slot(e.base().id)))
            .collect();
        let slots: Vec<Slot> = entities.iter().map(|e| Slot(e.base().id)).collect();
        let ids: Vec<f64> = entities.iter().map(|e| e.base().id as f64).collect();
        let records = app.spatial.store().measures(&ids);
        let fields = model
            .layers()
            .get(layer)
            .map(|n| n.fields.clone())
            .unwrap_or_default();
        let columns = layer_columns(&fields, &entities, Some(&records));
        let at = slots
            .iter()
            .enumerate()
            .map(|(i, s)| (*s, i as u32))
            .collect();
        LayerData {
            work: Work::new(DataSet {
                rows: slots.len(),
                columns,
            }),
            slots,
            at,
        }
    }

    /// Göster's rows: every object, the selected ones or the ones in the view.
    pub fn rows(&self, app: &App, show: Show) -> Vec<u32> {
        match show {
            Show::All => (0..self.slots.len() as u32).collect(),
            Show::Selected => self.selected_rows(app),
            Show::InView => {
                let view = app.viewport.camera.visible_bounds();
                let a = kentos_interaction::Vec2 {
                    x: view.min_x,
                    y: view.min_y,
                };
                let b = kentos_interaction::Vec2 {
                    x: view.max_x,
                    y: view.max_y,
                };
                let visible: std::collections::HashSet<Slot> =
                    app.spatial.in_rect(a, b, true).into_iter().collect();
                self.slots
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| visible.contains(s))
                    .map(|(i, _)| i as u32)
                    .collect()
            }
        }
    }

    /// The rows of the drawing's selected objects, in the rows' order.
    pub fn selected_rows(&self, app: &App) -> Vec<u32> {
        let mut out: Vec<u32> = app
            .selection
            .ids()
            .iter()
            .filter_map(|s| self.at.get(s).copied())
            .collect();
        out.sort_unstable();
        out
    }

    pub fn slots_of(&self, rows: &[u32]) -> Vec<Slot> {
        rows.iter()
            .filter_map(|&r| self.slots.get(r as usize).copied())
            .collect()
    }
}

/// The layer the tabs show: the one chosen while it is a layer, else the
/// first selected object's, else the active one, else the first.
pub(crate) fn layer_at_hand(app: &App, doc: &Document, kept: Option<&str>) -> Option<String> {
    let layers = doc.model.layers();
    let is_layer = |id: &str| {
        layers
            .get(id)
            .is_some_and(|n| n.kind == LayerNodeType::Layer)
    };
    if let Some(l) = kept.filter(|l| is_layer(l)) {
        return Some(l.to_owned());
    }
    app.selection
        .ids()
        .iter()
        .find_map(|&s| doc.model.get(s))
        .map(|e| e.base().layer_id.clone())
        .filter(|l| is_layer(l))
        .or_else(|| Some(layers.active().to_owned()).filter(|l| is_layer(l)))
        .or_else(|| layers.leaves().first().map(|n| n.id.clone()))
}

/// The drawing's layers for Katman ▾: id, path and object count (a filtered
/// layer counts what passes its filter).
pub(crate) fn layer_list(app: &App, doc: &Document) -> Vec<(String, String, usize)> {
    let model = &doc.model;
    model
        .layers()
        .leaves()
        .iter()
        .map(|n| {
            (
                n.id.clone(),
                model.layers().path(&n.id),
                app.spatial
                    .filter_counts(&n.id)
                    .map_or_else(|| model.by_layer(&n.id).count(), |(passed, _)| passed),
            )
        })
        .collect()
}

impl App {
    /// A click's way with the drawing's selection: replace, add (Shift) or
    /// take away (Ctrl, ⌘).
    pub(crate) fn apply_pick(&mut self, slots: Vec<Slot>) {
        if self.modifiers.control() || self.modifiers.logo() {
            let drop: std::collections::HashSet<Slot> = slots.into_iter().collect();
            let kept: Vec<Slot> = self
                .selection
                .ids()
                .iter()
                .copied()
                .filter(|s| !drop.contains(s))
                .collect();
            self.selection.set(kept);
        } else if self.modifiers.shift() {
            self.selection.add(slots);
        } else {
            self.selection.set(slots);
        }
    }
}
