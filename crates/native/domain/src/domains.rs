//! The project's value lists (docs/adr/0430): a setting the drawing keeps in its undo history, since a
//! list's change rewrites the copies its linked fields hold and undo must give both back.

use kentos_contracts::{FieldDomain, domains_problem, follow_domains};

use crate::document::Document;
use crate::history::Op;
use crate::{Refusal, Slot};

impl Document {
    /// Gives the project the value lists `next` as one change named `label` (into the open
    /// transaction, else an undo step of its own): the lists, and every linked field's copy written
    /// anew in the same step. Refused with nothing changed: lists with a problem
    /// (`kentos_contracts::domains_problem`). Whether anything changed.
    pub fn set_domains(&mut self, next: Vec<FieldDomain>, label: &str) -> Result<bool, Refusal> {
        if let Some((_, problem)) = domains_problem(&next) {
            return Err(Refusal(problem));
        }
        let before = self.settings.domains.clone();
        let followed: Vec<(String, Vec<kentos_contracts::LayerField>)> = self
            .layers
            .leaves()
            .into_iter()
            .filter_map(|n| {
                let (fields, changed) = follow_domains(&n.fields, &next);
                changed.then(|| (n.id.clone(), fields))
            })
            .collect();
        if before == next && followed.is_empty() {
            return Ok(false);
        }
        self.transact(label, |doc| {
            if before != next {
                doc.record(
                    vec![Op::Domains {
                        before,
                        after: next,
                    }],
                    label,
                );
            }
            for (layer, fields) in followed {
                doc.set_layer_fields(&layer, fields, &[])?;
            }
            Ok(true)
        })
    }

    /// The objects of `layer` whose `key` holds a value, with it (a list's codes are read off it).
    pub fn values_of(&self, layer: &str, name: &str) -> Vec<(Slot, String)> {
        self.by_layer(layer)
            .filter_map(|e| {
                let v = e.base().attrs.get(name)?;
                (!v.is_empty()).then(|| (Slot(e.base().id), v.clone()))
            })
            .collect()
    }
}

impl Document {
    /// Gives the project Mimari denetim's exceptions `next` (docs/adr/0732 §3) as one change named `label` (into the
    /// open transaction, else an undo step of its own). Refused with nothing changed: exceptions with a problem
    /// (`kentos_contracts::audit_exceptions_problem`). Whether anything changed.
    pub fn set_audit_exceptions(
        &mut self,
        next: Vec<kentos_contracts::ArchAuditException>,
        label: &str,
    ) -> Result<bool, Refusal> {
        if let Some((_, problem)) = kentos_contracts::audit_exceptions_problem(&next) {
            return Err(Refusal(problem));
        }
        let before = self
            .settings
            .arch
            .as_ref()
            .map(|a| a.audit_exceptions.clone())
            .unwrap_or_default();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::AuditExceptions { before, after: next }], label);
            Ok(true)
        })
    }
}

impl Document {
    /// Gives the project the building program `next` (docs/adr/0773) and the spaces' groups `units` (a space's slot and
    /// its group, none out of any) as one change named `label` (into the open transaction, else an undo step of its
    /// own). Refused with nothing changed: a program with a problem. Whether anything changed.
    pub fn set_program(
        &mut self,
        next: kentos_contracts::BuildingProgram,
        units: Vec<(Slot, Option<String>)>,
        label: &str,
    ) -> Result<bool, Refusal> {
        if let Some((_, problem)) = next.problem() {
            return Err(Refusal(problem));
        }
        let before = self.settings.arch.as_ref().map(|a| a.program.clone()).unwrap_or_default();
        let changes: Vec<(Slot, kentos_contracts::Entity)> = units
            .into_iter()
            .filter_map(|(slot, unit)| {
                let kentos_contracts::Entity::Space(mut s) = self.get(slot)?.clone() else {
                    return None;
                };
                (s.space.unit != unit).then(|| {
                    s.space.unit = unit;
                    (slot, kentos_contracts::Entity::Space(s))
                })
            })
            .collect();
        if before == next && changes.is_empty() {
            return Ok(false);
        }
        self.transact(label, |doc| {
            if before != next {
                doc.record(vec![Op::BuildingProgram { before, after: next }], label);
            }
            if !changes.is_empty() {
                doc.update_many(changes, label);
            }
            Ok(true)
        })
    }

    /// The building program (docs/adr/0773); empty when the project has none.
    pub fn program(&self) -> kentos_contracts::BuildingProgram {
        self.settings.arch.as_ref().map(|a| a.program.clone()).unwrap_or_default()
    }
}
