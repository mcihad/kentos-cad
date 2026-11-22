//! The table's conditions by layer (docs/adr/9132): Ara, İfade süzgeci and the order are what the reader asked of one
//! layer, not of the table. Leaving a layer keeps them under it, by its id and its source (a data layer's file or
//! database and its table: another source is another table); coming to a layer brings its own back, none for a layer not
//! shown before. A condition written for one table is never asked of another, where it could keep nothing and the
//! answer would look like the data's. Whichever way the table comes to another layer (Katman ▾, the selection, the
//! active layer) they follow it. They last while the drawing is on screen: another drawing starts the table again
//! (GIS-022); the project file does not keep them.

use super::{FeaturesPanel, data::DataConditions};
use crate::app::App;

/// What a layer's conditions are kept under: its id and its source (none for a layer of the drawing's own).
pub(crate) type Key = (String, String);

/// A layer's conditions: typed in the bar and, on a data layer, asked of its source.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Conditions {
    search: String,
    filter: String,
    sort: Option<String>,
    descending: bool,
    data: DataConditions,
}

impl FeaturesPanel {
    /// The conditions the table has, taken out: it has none until others are put.
    fn take_conditions(&mut self) -> Conditions {
        Conditions {
            search: std::mem::take(&mut self.search),
            filter: std::mem::take(&mut self.filter),
            sort: self.sort.take(),
            descending: std::mem::take(&mut self.descending),
            data: self.data.take_conditions(),
        }
    }

    fn put_conditions(&mut self, c: Conditions) {
        (self.search, self.filter, self.sort, self.descending) = (c.search, c.filter, c.sort, c.descending);
        self.data.put_conditions(c.data);
    }
}

impl App {
    /// The key of the layer the table shows now.
    fn table_key(&self) -> Option<Key> {
        let doc = self.document.as_ref()?;
        let layer = self.feature_layer(doc)?;
        let source = doc
            .model
            .layers()
            .get(&layer)
            .and_then(|n| n.data.as_ref())
            .map(|d| format!("{}\u{1f}{}", d.address, d.table))
            .unwrap_or_default();
        Some((layer, source))
    }

    /// The conditions follow the table's layer: the ones left kept under the layer left, the layer come to brings its
    /// own. After every message, and at once when Katman ▾ picks a layer.
    pub(crate) fn table_conditions_follow(&mut self) {
        let now = self.table_key();
        let panel = &mut self.gis.features;
        if panel.conditions_of == now {
            return;
        }
        // The first layer the table shows: the conditions typed are its own.
        if let Some(left) = panel.conditions_of.take() {
            let kept = panel.take_conditions();
            if kept == Conditions::default() {
                panel.kept.remove(&left);
            } else {
                panel.kept.insert(left, kept);
            }
            if let Some(key) = &now {
                let back = panel.kept.remove(key).unwrap_or_default();
                panel.put_conditions(back);
            }
            panel.changed();
        }
        panel.conditions_of = now;
    }
}

#[cfg(test)]
#[path = "conditions_tests.rs"]
mod conditions_tests;
