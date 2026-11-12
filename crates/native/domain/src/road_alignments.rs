//! A road project's horizontal alignments (docs/adr/0901 §10): a setting the drawing keeps in its undo history, so
//! every write of the Güzergâh editor and of `road.alignments.*` is one undo step.

use std::sync::Arc;
use kentos_contracts::{AlignmentDef, RoadSettings};

use crate::Refusal;
use crate::document::Document;
use crate::history::Op;
use crate::history::RoadOp;

impl Document {
    /// The project's alignments, in the order the project tree shows them.
    pub fn road_alignments(&self) -> &[AlignmentDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.alignments.as_slice())
    }

    /// Gives the project the alignments `next` as one change named `label` (into the open transaction, else an undo
    /// step of its own). Refused with nothing changed: alignments with a problem (`road_problem`). Whether anything
    /// changed.
    pub fn set_road_alignments(&mut self, next: Vec<AlignmentDef>, label: &str) -> Result<bool, Refusal> {
        let judged = RoadSettings {
            alignments: next.clone(),
            ..self.settings.road.clone().unwrap_or_default()
        };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_alignments().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(
                vec![Op::Road(RoadOp::Alignments {
                    before: Arc::new(before),
                    after: Arc::new(next),
                })],
                label,
            );
            Ok(true)
        })
    }
}
