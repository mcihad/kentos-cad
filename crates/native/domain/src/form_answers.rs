//! The author's answers to the model quality control form (docs/adr/8077; EK-9 Tablo 9.1): a setting the drawing keeps
//! in its undo history, a step of its own (`cad.form.answer`).

use kentos_contracts::FormAnswer;

use crate::Refusal;
use crate::document::Document;
use crate::history::ArchOp;
use crate::history::Op;

impl Document {
    /// The author's answers, by question; empty when none is given.
    pub fn form_answers(&self) -> &[FormAnswer] {
        self.settings
            .arch
            .as_ref()
            .map_or(&[], |a| a.form_answers.as_slice())
    }

    /// Gives the drawing `next` (by question) as one change named `label` (into the open transaction, else an undo step
    /// of its own). Refused with nothing changed: an answer with a problem, a question twice. Whether anything changed.
    pub fn set_form_answers(
        &mut self,
        mut next: Vec<FormAnswer>,
        label: &str,
    ) -> Result<bool, Refusal> {
        if let Some((_, why)) = FormAnswer::list_problem(&next) {
            return Err(Refusal(why));
        }
        next.sort_by_key(|a| a.no);
        if self.form_answers() == next.as_slice() {
            return Ok(false);
        }
        let before = std::sync::Arc::new(self.form_answers().to_vec());
        self.transact(label, |doc| {
            doc.record(
                vec![Op::Arch(ArchOp::FormAnswers {
                    before,
                    after: std::sync::Arc::new(next),
                })],
                label,
            );
            Ok(true)
        })
    }
}
