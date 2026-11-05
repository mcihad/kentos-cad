//! The review notes' undo step (docs/adr/1000 §3) and the pattern tables' (docs/adr/1002 §4). Notes live in the project's settings, which are otherwise outside
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

impl Document {
    /// Replaces the project's line types and hatch patterns as one undo step named `label` (inside an open step,
    /// part of it; docs/adr/1002 §4). Nothing happens when they are the same.
    pub fn set_pattern_tables(
        &mut self,
        line_types: Vec<kentos_contracts::LineTypeDef>,
        hatch_patterns: Vec<kentos_contracts::HatchPatternDef>,
        label: &str,
    ) {
        if self.settings.line_types == line_types && self.settings.hatch_patterns == hatch_patterns {
            return;
        }
        let before = Arc::new((self.settings.line_types.clone(), self.settings.hatch_patterns.clone()));
        let after = Arc::new((line_types, hatch_patterns));
        self.record(vec![Op::PatternTables { before, after }], label);
    }

    /// Replaces the approved sheet template revisions as one undo step named `label` (inside an open step, part of
    /// it; docs/adr/1100 §3). Nothing happens when they are the same.
    pub fn set_approved_sheet_templates(
        &mut self,
        templates: Vec<kentos_contracts::ApprovedSheetTemplate>,
        label: &str,
    ) {
        if self.settings.approved_sheet_templates == templates {
            return;
        }
        let before = Arc::new(std::mem::replace(&mut self.settings.approved_sheet_templates, templates));
        let after = Arc::new(self.settings.approved_sheet_templates.clone());
        self.record(vec![Op::ApprovedSheetTemplates { before, after }], label);
    }
}
