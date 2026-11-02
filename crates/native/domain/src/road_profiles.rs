//! A road project's vertical alignments (docs/adr/0902 §9): a setting the drawing keeps in its undo history, so every
//! write of the Boykesit editor and of `road.profiles.*` is one undo step.

use kentos_contracts::{ProfileDef, RoadSettings};

use crate::Refusal;
use crate::document::Document;
use crate::history::Op;

impl Document {
    /// The project's profiles, in the order the project tree shows them.
    pub fn road_profiles(&self) -> &[ProfileDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.profiles.as_slice())
    }

    /// Gives the project the profiles `next` as one change named `label` (into the open transaction, else an undo
    /// step of its own). Refused with nothing changed: profiles with a problem (`road_problem`, with the project's
    /// alignments). Whether anything changed.
    pub fn set_road_profiles(&mut self, next: Vec<ProfileDef>, label: &str) -> Result<bool, Refusal> {
        let judged = RoadSettings {
            alignments: self.road_alignments().to_vec(),
            profiles: next.clone(),
            ..RoadSettings::default()
        };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_profiles().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::RoadProfiles { before, after: next }], label);
            Ok(true)
        })
    }
}
