//! Altlık modeller (docs/adr/8101; plan/ifc-uyum.md IFC-09): the drawing's underlay models, a setting it keeps in its
//! undo history, a step of its own (`cad.underlay.edit`).

use kentos_contracts::ModelUnderlay;

use crate::Refusal;
use crate::document::Document;
use crate::history::ArchOp;
use crate::history::Op;

impl Document {
    /// The underlay models, in the order they were added; empty when none is shown.
    pub fn underlays(&self) -> &[ModelUnderlay] {
        self.settings
            .arch
            .as_ref()
            .map_or(&[], |a| a.underlays.as_slice())
    }

    /// Gives the drawing `next` as one change named `label` (into the open transaction, else an undo step of its own).
    /// Refused with nothing changed: a list with a problem. Whether anything changed.
    pub fn set_underlays(
        &mut self,
        next: Vec<ModelUnderlay>,
        label: &str,
    ) -> Result<bool, Refusal> {
        if let Some((_, why)) = ModelUnderlay::list_problem(&next) {
            return Err(Refusal(why));
        }
        if self.underlays() == next.as_slice() {
            return Ok(false);
        }
        let before = std::sync::Arc::new(self.underlays().to_vec());
        self.transact(label, |doc| {
            doc.record(
                vec![Op::Arch(ArchOp::Underlays {
                    before,
                    after: std::sync::Arc::new(next),
                })],
                label,
            );
            Ok(true)
        })
    }
}
