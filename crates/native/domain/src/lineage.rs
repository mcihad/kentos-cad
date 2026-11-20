//! What the split and merged objects came from (docs/adr/0598): a setting the drawing keeps in its undo history,
//! written in the step whose split or merge made the records, so that undo takes them back with the pieces.

use std::sync::Arc;

use kentos_contracts::{LineageRecord, lineage_problem, lineage_with};

use crate::Refusal;
use crate::document::Document;
use crate::history::GisOp;
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
                vec![Op::Gis(GisOp::Lineage {
                    before: Arc::new(before),
                    after: Arc::new(after),
                })],
                label,
            );
            Ok(true)
        })
    }
}

impl Document {
    /// Gives the project the relations `next` as one change named `label` (into the open transaction or group, else an
    /// undo step of its own; GIS-051): undo takes them back with the layers of the same step. Whether anything changed.
    pub fn set_relations(&mut self, next: Vec<kentos_contracts::ProjectRelation>, label: &str) -> bool {
        let before = self.settings.relations.clone();
        if before == next {
            return false;
        }
        let _ = self.transact(label, |doc| -> Result<(), ()> {
            doc.record(vec![Op::Gis(GisOp::Relations { before: Arc::new(before), after: Arc::new(next) })], label);
            Ok(())
        });
        true
    }

    /// Gives the project the address locators `next`, as [`Document::set_relations`] gives relations.
    pub fn set_address_locators(&mut self, next: Vec<kentos_contracts::AddressLocator>, label: &str) -> bool {
        let before = self.settings.address_locators.clone();
        if before == next {
            return false;
        }
        let _ = self.transact(label, |doc| -> Result<(), ()> {
            doc.record(
                vec![Op::Gis(GisOp::AddressLocators { before: Arc::new(before), after: Arc::new(next) })],
                label,
            );
            Ok(())
        });
        true
    }
}
