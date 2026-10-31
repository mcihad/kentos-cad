//! The project's door and window types (docs/adr/0772): a setting the drawing keeps in its undo history, written in the
//! step with the openings that follow them, so undo gives both back.

use kentos_contracts::{OpeningType, opening_types_problem};

use crate::Refusal;
use crate::document::Document;
use crate::history::Op;

impl Document {
    /// The project's door and window types, in the order they were made.
    pub fn opening_types(&self) -> &[OpeningType] {
        self.settings
            .arch
            .as_ref()
            .map_or(&[], |a| a.opening_types.as_slice())
    }

    /// Gives the project the types `next` as one change named `label` (into the open transaction, else an undo step of
    /// its own). Refused with nothing changed: types with a problem (`opening_types_problem`). Whether anything changed.
    pub fn set_opening_types(
        &mut self,
        next: Vec<OpeningType>,
        label: &str,
    ) -> Result<bool, Refusal> {
        if let Some((_, problem)) = opening_types_problem(&next) {
            return Err(Refusal(problem));
        }
        let before = self.opening_types().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(
                vec![Op::OpeningTypes {
                    before,
                    after: next,
                }],
                label,
            );
            Ok(true)
        })
    }
}
