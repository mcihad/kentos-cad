//! Sırala (docs/adr/0260 §5, 0310 §3): Öne getir and Arkaya gönder for the
//! selection, and Yazıları öne getir and Taramaları arkaya gönder for every
//! shown object of those kinds (AutoCAD's TEXTTOFRONT, HATCHTOBACK): found by
//! `cad.entities.select`, moved by `cad.entities.order`; what the command
//! answered is said. Nesnenin üstüne and altına are tools
//! (`kentos_interaction::draw_order`).

use kentos_commands::{
    CommandResult, EntitiesOrder, EntitiesSelect, OrderTo, SelectKind, SelectRegion,
};
use kentos_native_application::{ExecutionContext, entities_order, entities_select};

/// Yazıları öne getir's kinds: what carries words (AutoCAD's TEXTTOFRONT All).
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
        self.order(input, entities_order::label(to));
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
