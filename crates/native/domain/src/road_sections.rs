//! A road project's templates and sample line sets (docs/adr/0903 §6): settings the drawing keeps in its undo
//! history, so every write of the Enkesit window and of `road.templates.*` and `road.sections.*` is one undo step.

use kentos_contracts::{CorridorDef, HaulDef, RoadSettings, SectionSetDef, SuperelevationDef, TemplateDef, VolumeDef};

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

    /// The project's superelevation schedules (docs/adr/0904).
    pub fn road_superelevations(&self) -> &[SuperelevationDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.superelevations.as_slice())
    }

    /// The project's earthwork volumes (docs/adr/0906).
    pub fn road_volumes(&self) -> &[VolumeDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.volumes.as_slice())
    }

    /// Gives the project the earthwork volumes `next` as one change named `label`, as [`Self::set_road_templates`].
    pub fn set_road_volumes(&mut self, next: Vec<VolumeDef>, label: &str) -> Result<bool, Refusal> {
        let judged = RoadSettings { volumes: next.clone(), ..self.settings.road.clone().unwrap_or_default() };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_volumes().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::RoadVolumes { before, after: next }], label);
            Ok(true)
        })
    }

    /// The project's plan–profile sheet sets (docs/adr/0908 §10).
    pub fn road_sheet_sets(&self) -> &[kentos_contracts::RoadSheetSetDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.sheet_sets.as_slice())
    }

    /// Gives the project the sheet sets `next` as one change named `label`, as [`Self::set_road_templates`]; inside an
    /// open group it joins the group's step.
    pub fn set_road_sheet_sets(
        &mut self,
        next: Vec<kentos_contracts::RoadSheetSetDef>,
        label: &str,
    ) -> Result<bool, Refusal> {
        let judged = RoadSettings { sheet_sets: next.clone(), ..self.settings.road.clone().unwrap_or_default() };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_sheet_sets().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::RoadSheetSets { before, after: next }], label);
            Ok(true)
        })
    }

    /// The project's guardrails (docs/adr/0909 §1).
    pub fn road_guardrails(&self) -> &[kentos_contracts::GuardrailDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.guardrails.as_slice())
    }

    /// Gives the project the guardrails `next` as one change named `label`, as [`Self::set_road_templates`].
    pub fn set_road_guardrails(&mut self, next: Vec<kentos_contracts::GuardrailDef>, label: &str) -> Result<bool, Refusal> {
        let judged = RoadSettings { guardrails: next.clone(), ..self.settings.road.clone().unwrap_or_default() };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_guardrails().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::RoadGuardrails { before, after: next }], label);
            Ok(true)
        })
    }

    /// The project's mass haul diagrams (docs/adr/0907).
    pub fn road_hauls(&self) -> &[HaulDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.hauls.as_slice())
    }

    /// Gives the project the mass haul diagrams `next` as one change named `label`, as [`Self::set_road_templates`].
    pub fn set_road_hauls(&mut self, next: Vec<HaulDef>, label: &str) -> Result<bool, Refusal> {
        let judged = RoadSettings { hauls: next.clone(), ..self.settings.road.clone().unwrap_or_default() };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_hauls().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::RoadHauls { before, after: next }], label);
            Ok(true)
        })
    }

    /// The project's corridors (docs/adr/0905).
    pub fn road_corridors(&self) -> &[CorridorDef] {
        self.settings.road.as_ref().map_or(&[], |r| r.corridors.as_slice())
    }

    /// Gives the project the corridors `next` as one change named `label`, as [`Self::set_road_templates`].
    pub fn set_road_corridors(&mut self, next: Vec<CorridorDef>, label: &str) -> Result<bool, Refusal> {
        let judged = RoadSettings { corridors: next.clone(), ..self.settings.road.clone().unwrap_or_default() };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_corridors().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::RoadCorridors { before, after: next }], label);
            Ok(true)
        })
    }

    /// Gives the project the superelevation schedules `next` as one change named `label`, as
    /// [`Self::set_road_templates`].
    pub fn set_road_superelevations(&mut self, next: Vec<SuperelevationDef>, label: &str) -> Result<bool, Refusal> {
        let judged = RoadSettings {
            superelevations: next.clone(),
            ..self.settings.road.clone().unwrap_or_default()
        };
        if let Some((path, problem)) = judged.problem() {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.road_superelevations().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::RoadSuperelevations { before, after: next }], label);
            Ok(true)
        })
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
