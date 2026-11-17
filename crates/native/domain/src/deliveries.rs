//! Teslim kaydı (docs/adr/8099; plan/bim-plan.md BIM-15): the drawing's delivery record, a setting it keeps in its undo
//! history, a step of its own (`cad.delivery.edit`).

use kentos_contracts::Delivery;

use crate::Refusal;
use crate::document::Document;
use crate::history::ArchOp;
use crate::history::Op;

impl Document {
    /// The delivery record, in the order the packages went; empty when none is kept.
    pub fn deliveries(&self) -> &[Delivery] {
        self.settings.arch.as_ref().map_or(&[], |a| a.deliveries.as_slice())
    }

    /// Gives the drawing `next` as one change named `label` (into the open transaction, else an undo step of its own).
    /// Refused with nothing changed: a record with a problem. Whether anything changed.
    pub fn set_deliveries(&mut self, next: Vec<Delivery>, label: &str) -> Result<bool, Refusal> {
        if let Some((_, why)) = Delivery::list_problem(&next) {
            return Err(Refusal(why));
        }
        if self.deliveries() == next.as_slice() {
            return Ok(false);
        }
        let before = std::sync::Arc::new(self.deliveries().to_vec());
        self.transact(label, |doc| {
            doc.record(vec![Op::Arch(ArchOp::Deliveries { before, after: std::sync::Arc::new(next) })], label);
            Ok(true)
        })
    }
}
