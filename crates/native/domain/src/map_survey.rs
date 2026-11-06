//! A map project's GNSS campaign archive's undo step (plan/map-cad.md MAP-03, docs/adr/1340). The archive lives in the
//! project's settings, which are otherwise outside the model's undo; its change is a step of its own, or a part of the
//! step it is written in (accepting a point also draws its object).

use std::sync::Arc;

use kentos_contracts::MapSurvey;

use crate::{Document, history::Op};

impl Document {
    /// The project's GNSS archive; none in a project without one.
    pub fn map_survey(&self) -> Option<&MapSurvey> {
        self.settings.map_survey.as_ref()
    }

    /// Replaces the archive as one undo step named `label` (inside an open transaction or group, part of it); an
    /// empty one is none. Nothing happens when it is the same.
    pub fn set_map_survey(&mut self, next: Option<MapSurvey>, label: &str) {
        let next = next.filter(|w| !w.is_empty());
        if self.settings.map_survey == next {
            return;
        }
        let before = Arc::new(std::mem::replace(&mut self.settings.map_survey, next));
        let after = Arc::new(self.settings.map_survey.clone());
        self.record(vec![Op::MapSurvey { before, after }], label);
    }
}
