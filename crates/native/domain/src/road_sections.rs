//! A road project's templates and sample line sets (docs/adr/0903 §6): settings the drawing keeps in its undo
//! history, so every write of the Enkesit window and of `road.templates.*` and `road.sections.*` is one undo step.

use kentos_contracts::{RoadSettings, SectionSetDef, TemplateDef};

use crate::Refusal;
use crate::document::Document;
use crate::history::Op;

impl Document {
    /// The project's templates, in the order the project tree shows them.
    pub fn road_templates(&self) -> &[TemplateDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.templates.as_slice())
    }

    /// The project's sample line sets.
    pub fn road_sections(&self) -> &[SectionSetDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.sections.as_slice())
    }

    /// Gives the project the templates `next` as one change named `label`. Refused with nothing changed: road settings
    /// with a problem (`road_problem`). Whether anything changed.
    pub fn set_road_templates(&mut self, next: Vec<TemplateDef>, label: &str) -> Result<bool, Refusal> {
        let judged = RoadSettings {
            templates: next.clone(),
            ..self.settings.road.clone().unwrap_or_default()
        };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_templates().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::RoadTemplates { before, after: next }], label);
            Ok(true)
        })
    }

    /// Gives the project the sample line sets `next` as one change named `label`, as [`Self::set_road_templates`].
    pub fn set_road_sections(&mut self, next: Vec<SectionSetDef>, label: &str) -> Result<bool, Refusal> {
        let judged = RoadSettings {
            sections: next.clone(),
            ..self.settings.road.clone().unwrap_or_default()
        };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_sections().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::RoadSections { before, after: next }], label);
            Ok(true)
        })
    }
}
