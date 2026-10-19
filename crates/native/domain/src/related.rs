//! What the project's relations tie to some objects (docs/adr/0525 §1): a network trace's reach as
//! the records it touches. Through a relation whose parent layer holds an object reached, the
//! children whose field holds its persistent id are attached to it (a pipe's subscribers); through
//! one whose child layer holds it, the parent its field names holds it (a valve's chamber). Read
//! only; the relations' own rules (docs/adr/0405) are not asked.

use std::collections::HashSet;

use crate::Document;
use crate::identity::Slot;

/// Which way a relation ties a related record to the objects asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tie {
    /// A child of an object asked (its field names the object).
    Attached,
    /// The parent an object asked names (it holds the object).
    Container,
}

/// The records a relation ties to the objects asked: the relation's id and name, which way, and the
/// records (slots in their layer's document order, each once).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Related {
    pub relation: String,
    pub name: String,
    pub tie: Tie,
    pub slots: Vec<Slot>,
}

impl Document {
    /// The records the project's relations tie to `slots`, relation by relation in the project's
    /// order (attached children, then containers); relations that tie none are left out.
    pub fn related_of(&self, slots: &[Slot]) -> Vec<Related> {
        let asked: HashSet<Slot> = slots.iter().copied().collect();
        let mut out = Vec::new();
        for r in &self.settings.relations {
            let Some((field, many)) = self
                .layers
                .get(&r.child)
                .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
                .map(|f| (f.name.clone(), f.multiple))
            else {
                continue;
            };
            let ids = |value: &str| -> Vec<String> {
                if many {
                    value
                        .split(';')
                        .map(str::trim)
                        .filter(|h| !h.is_empty())
                        .map(str::to_owned)
                        .collect()
                } else {
                    let v = value.trim();
                    if v.is_empty() {
                        Vec::new()
                    } else {
                        vec![v.to_owned()]
                    }
                }
            };
            // Attached: the children of the parents asked.
            let parents: HashSet<String> = self
                .store
                .on_layer(&r.parent)
                .filter(|s| asked.contains(&s.slot()))
                .map(|s| s.uid.to_string())
                .collect();
            if !parents.is_empty() {
                let children: Vec<Slot> = self
                    .store
                    .on_layer(&r.child)
                    .filter(|c| {
                        c.entity
                            .base()
                            .attrs
                            .get(&field)
                            .is_some_and(|v| ids(v).iter().any(|id| parents.contains(id)))
                    })
                    .map(|c| c.slot())
                    .collect();
                if !children.is_empty() {
                    out.push(Related {
                        relation: r.id.clone(),
                        name: r.name.clone(),
                        tie: Tie::Attached,
                        slots: children,
                    });
                }
            }
            // Containers: the parents the children asked name.
            let named: HashSet<String> = self
                .store
                .on_layer(&r.child)
                .filter(|c| asked.contains(&c.slot()))
                .filter_map(|c| c.entity.base().attrs.get(&field))
                .flat_map(|v| ids(v))
                .collect();
            if !named.is_empty() {
                let holders: Vec<Slot> = self
                    .store
                    .on_layer(&r.parent)
                    .filter(|p| named.contains(&p.uid.to_string()))
                    .map(|p| p.slot())
                    .collect();
                if !holders.is_empty() {
                    out.push(Related {
                        relation: r.id.clone(),
                        name: r.name.clone(),
                        tie: Tie::Container,
                        slots: holders,
                    });
                }
            }
        }
        out
    }
}
