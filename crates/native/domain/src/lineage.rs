//! What the split and merged objects came from (docs/adr/0598): a setting the drawing keeps in its undo history,
//! written in the step whose split or merge made the records, so that undo takes them back with the pieces.

use std::sync::Arc;

use kentos_contracts::{LineageRecord, lineage_problem, lineage_with};

use crate::Refusal;
use crate::document::Document;
use crate::history::Op;

impl Document {
    /// The records of what the split and merged objects came from, in the order the steps made them.
    pub fn lineage(&self) -> &[LineageRecord] {
        &self.settings.lineage
    }

    /// Adds `records` as one change named `label` (into the open transaction, else an undo step of its own); a record
    /// already kept stays once, the oldest go past the limit. Refused with nothing changed: a record with a problem.
    /// Whether anything changed.
    pub fn note_lineage(
        &mut self,
        records: &[LineageRecord],
        label: &str,
    ) -> Result<bool, Refusal> {
        if records.is_empty() {
            return Ok(false);
        }
        let before = self.settings.lineage.clone();
        let after = lineage_with(&before, records);
        if let Some(problem) = lineage_problem(&after) {
            return Err(Refusal(problem));
        }
        if after == before {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(
                vec![Op::Lineage {
                    before: Arc::new(before),
                    after: Arc::new(after),
                }],
                label,
            );
            Ok(true)
        })
    }
}
