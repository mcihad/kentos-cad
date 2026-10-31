//! The project's building materials (docs/adr/0785): a setting the drawing keeps in its undo history, written in the
//! step with the wall styles, walls and slabs bound to them, so undo gives both back.

use kentos_contracts::{ArchMaterial, materials_problem};

use crate::Refusal;
use crate::document::Document;
use crate::history::Op;

impl Document {
    /// The project's materials, in the order they were made.
    pub fn materials(&self) -> &[ArchMaterial] {
        self.settings
            .arch
            .as_ref()
            .map_or(&[], |a| a.materials.as_slice())
    }

    /// Gives the project the materials `next` as one change named `label` (into the open transaction, else an undo
    /// step of its own). Refused with nothing changed: materials with a problem (`materials_problem`). Whether anything
    /// changed.
    pub fn set_materials(&mut self, next: Vec<ArchMaterial>, label: &str) -> Result<bool, Refusal> {
        if let Some((_, problem)) = materials_problem(&next) {
            return Err(Refusal(problem));
        }
        let before = self.materials().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::Materials { before, after: next }], label);
            Ok(true)
        })
    }
}
