//! Sırala (docs/adr/0260 §5, 0310 §3): Öne getir and Arkaya gönder for the
//! selection, and Yazıları öne getir and Taramaları arkaya gönder for every
//! shown object of those kinds: found by
//! `cad.entities.select`, moved by `cad.entities.order`; what the command
//! answered is said. Nesnenin üstüne and altına are tools
//! (`kentos_interaction::draw_order`).

use kentos_commands::{
    CommandResult, EntitiesOrder, EntitiesSelect, OrderTo, SelectKind, SelectRegion,
};
use kentos_native_application::{ExecutionContext, entities_order, entities_select};

/// Yazıları öne getir's kinds: what carries words.
pub(crate) const WORDS: [SelectKind; 4] = [
    SelectKind::Text,
    SelectKind::Dimension,
    SelectKind::Leader,
    SelectKind::Table,
];

use crate::app::App;

impl App {
    /// The selected objects to the front or the back of their layers.
    pub(crate) fn order_selection(&mut self, to: OrderTo) {
        if self.selection.is_empty() {
            self.warn("Önce sırası değişecek nesneleri seçin.");
            return;
        }
        let ids = self.selection.ids();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = EntitiesOrder {
            uids: kentos_interaction::properties::uids_of(&doc.model, ids),
            to,
            reference: None,
            expected_revision: None,
        };
        let crossing = self.other_layers_crossing();
        let words = self.words_over_mask();
        self.order(input, entities_order::label(to));
        if let Some(note) = words {
            self.output(note);
        }
        // The order is within a layer (docs/adr/0310 §2–3): an object of another layer over the selection's place stays
        // where its layer's order puts it, which the result alone does not say.
        if crossing > 0 {
            self.output(format!(
                "Sıra yalnız nesnenin kendi katmanı içinde değişir; seçimin yerindeki {crossing} nesne başka katmanda ve \
                 katmanların sırasıyla çizilir. Onun üstüne ya da altına geçmek için Katmanlar panelinde katmanın sırasını \
                 değiştirin ya da nesneyi o katmana taşıyın."
            ));
        }
    }

    /// When the selection is or carries words, or is a mask, and words and a mask meet where it stands: that the mask
    /// does not cover the words whatever the order (docs/adr/0310 §8).
    fn words_over_mask(&mut self) -> Option<String> {
        let doc = self.document.as_ref()?;
        self.spatial.sync(&doc.model);
        let ids = self.selection.ids();
        let concerned = ids.iter().filter_map(|s| doc.model.get(*s)).any(|e| {
            kentos_interaction::draw_order::carries_words(e)
                || matches!(e, kentos_commands::Entity::Polygon(p) if p.base.look.wipeout.is_some())
        });
        if !concerned {
            return None;
        }
        let numbers: Vec<f64> = ids.iter().map(|s| f64::from(s.0)).collect();
        let b = self.spatial.store().extent(Some(&numbers))?;
        kentos_interaction::draw_order::words_over_mask_note(
            &doc.model,
            &self.spatial,
            kentos_interaction::Vec2::new(b.min_x, b.min_y),
            kentos_interaction::Vec2::new(b.max_x, b.max_y),
            true,
        )
    }

    /// How many shown objects of other layers than the selection's stand where the selection does (its box): the
    /// ones Öne getir and Arkaya gönder cannot pass.
    pub(crate) fn other_layers_crossing(&mut self) -> usize {
        let Some(doc) = self.document.as_ref() else {
            return 0;
        };
        self.spatial.sync(&doc.model);
        let ids = self.selection.ids();
        let numbers: Vec<f64> = ids.iter().map(|s| f64::from(s.0)).collect();
        let Some(b) = self.spatial.store().extent(Some(&numbers)) else {
            return 0;
        };
        let layers: std::collections::HashSet<&str> = ids
            .iter()
            .filter_map(|s| doc.model.get(*s))
            .map(|e| e.base().layer_id.as_str())
            .collect();
        let query = EntitiesSelect {
            model_scope: self.spatial.sheet_model_scope(),
            region: SelectRegion::Crossing {
                a: kentos_commands::Vec2 { x: b.min_x, y: b.min_y },
                b: kentos_commands::Vec2 { x: b.max_x, y: b.max_y },
            },
            kinds: None,
            layers: None,
        };
        match entities_select::select(&doc.model, self.spatial.store(), &query) {
            Ok(found) => found
                .slots
                .iter()
                .filter(|s| !ids.contains(s))
                .filter_map(|s| doc.model.get(*s))
                .filter(|e| !layers.contains(e.base().layer_id.as_str()))
                .count(),
            Err(_) => 0,
        }
    }

    /// Every shown object of `kinds` to the front or the back of its layer
    /// (Yazıları öne getir, Taramaları arkaya gönder); `what` names it.
    pub(crate) fn order_kinds(&mut self, kinds: &[SelectKind], to: OrderTo, what: &str) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let query = EntitiesSelect {
            model_scope:self.spatial.sheet_model_scope(),
            region: SelectRegion::All,
            kinds: Some(kinds.to_vec()),
            layers: None,
        };
        let uids = match entities_select::select(&doc.model, self.spatial.store(), &query) {
            Ok(found) => kentos_interaction::properties::uids_of(&doc.model, &found.slots),
            Err(error) => {
                self.warn(error.message);
                return;
            }
        };
        if uids.is_empty() {
            self.output(format!("{what}: çizimde böyle nesne yok."));
            return;
        }
        let input = EntitiesOrder {
            uids,
            to,
            reference: None,
            expected_revision: None,
        };
        self.order(input, what);
    }

    /// Runs `input` and says what it did as `what`.
    fn order(&mut self, input: EntitiesOrder, what: &str) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let result = entities_order::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match result {
            CommandResult::Completed { output, .. } => {
                let n = output.changed.len();
                if n == 0 {
                    self.output(format!("{what}: nesneler zaten yerinde."));
                } else {
                    self.output(format!("{what}: {n} nesnenin çizim sırası değişti."));
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }
}

#[cfg(test)]
#[path = "../tests/draw_order_tests.rs"]
mod tests;
