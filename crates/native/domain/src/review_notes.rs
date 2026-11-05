//! The review notes' undo step (docs/adr/1000 §3). Notes live in the project's settings, which are otherwise outside
//! the model's undo (`Document::set_settings`); a note's change is a step of its own, “İnceleme notu”, so that undo
//! and redo take a note back or bring it again like an object.

use std::sync::Arc;

use kentos_contracts::ReviewNote;

use crate::{Document, history::Op};

impl Document {
    /// Replaces the review notes as one undo step named `label` (inside an open transaction or group, part of it).
    /// Nothing happens when they are the same.
    pub fn set_review_notes(&mut self, notes: Vec<ReviewNote>, label: &str) {
        if self.settings.review_notes == notes {
            return;
        }
        let before = Arc::new(std::mem::replace(&mut self.settings.review_notes, notes));
        let after = Arc::new(self.settings.review_notes.clone());
        self.record(vec![Op::ReviewNotes { before, after }], label);
    }
}
