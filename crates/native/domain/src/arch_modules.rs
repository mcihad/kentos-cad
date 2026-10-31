//! The project's room modules (docs/adr/0775): a setting the drawing keeps in its undo history, written in the step with
//! the objects a module places or updates, so undo gives both back.

use kentos_contracts::{ArchModule, arch_modules_problem};

use crate::Refusal;
use crate::document::Document;
use crate::history::Op;

impl Document {
    /// The project's room modules, in the order they were made.
    pub fn arch_modules(&self) -> &[ArchModule] {
        self.settings
            .arch
            .as_ref()
            .map_or(&[], |a| a.modules.as_slice())
    }

    /// Gives the project the modules `next` as one change named `label` (into the open transaction, else an undo step
    /// of its own). Refused with nothing changed: modules with a problem (`arch_modules_problem`). Whether anything
    /// changed.
    pub fn set_arch_modules(
        &mut self,
        next: Vec<ArchModule>,
        label: &str,
    ) -> Result<bool, Refusal> {
        if let Some((path, problem)) = arch_modules_problem(&next) {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.arch_modules().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(
                vec![Op::ArchModules {
                    before,
                    after: next,
                }],
                label,
            );
            Ok(true)
        })
    }
}
