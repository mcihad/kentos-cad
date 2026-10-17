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
