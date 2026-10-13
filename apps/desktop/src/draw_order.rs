//! Sırala (docs/adr/0260 §5): Öne getir and Arkaya gönder for the selection,
//! through `cad.entities.order`; what the command answered is said.

use kentos_contracts::{CommandResult, EntitiesOrder, OrderTo};
use kentos_native_application::{ExecutionContext, entities_order};

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
            expected_revision: None,
        };
        let result = entities_order::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match result {
            CommandResult::Completed { output, .. } => {
                let n = output.changed.len();
                let what = entities_order::label(to);
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
