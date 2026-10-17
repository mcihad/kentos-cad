//! The project's relations, kept by the document (docs/adr/0405 §2): when a step deletes a parent
//! object, its children (the objects of the relation's child layer whose field holds its persistent
//! id) follow in the same step: `unlink` empties their field, `cascade` deletes them and follows
//! their own relations; `keep` and `restrict` leave them (a command refuses a `restrict` first). A
//! child on a locked layer is left as it is.
//! Every way of deleting (Sil, Kes, Python, MCP, a layer removed) gets it alike.

use std::collections::HashSet;
use std::sync::Arc;

use kentos_contracts::RelationDelete;
use uuid::Uuid;

use crate::Document;
use crate::history::Op;
use crate::store::Stored;

impl Document {
    /// The ops that follow the parents `ops` deletes, applied: the step takes them in.
    pub(crate) fn follow_relations(&mut self, ops: &[Op]) -> Vec<Op> {
        if self.settings.relations.is_empty() {
            return Vec::new();
        }
        let mut gone: Vec<(String, Uuid)> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Remove(s) => Some((s.entity.base().layer_id.clone(), s.uid)),
                _ => None,
            })
            .collect();
        let mut seen: HashSet<Uuid> = gone.iter().map(|(_, u)| *u).collect();
        let mut out = Vec::new();
        while let Some((layer, parent)) = gone.pop() {
            let id = parent.to_string();
            for r in self.settings.relations.clone() {
                if r.parent != layer
                    || matches!(r.on_delete, RelationDelete::Keep | RelationDelete::Restrict)
                {
                    continue;
                }
                let Some((name, many)) = self
                    .layers
                    .get(&r.child)
                    .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
                    .map(|f| (f.name.clone(), f.multiple))
                else {
                    continue;
                };
                // A locked layer's objects do not change: their links stay (docs/adr/0405 §2).
                if self.layers.is_locked(&r.child) {
                    continue;
                }
                let children: Vec<Stored> = self
                    .store
                    .on_layer(&r.child)
                    .filter(|s| {
                        s.entity
                            .base()
                            .attrs
                            .get(&name)
                            .is_some_and(|v| holds_id(v, &id))
                    })
                    .cloned()
                    .collect();
                for child in children {
                    // A many-to-many child keeps its other parents: the gone one's id leaves its list,
                    // and it is cascaded only when none is left (docs/adr/0409 §2).
                    let left: Option<String> = many.then(|| {
                        child
                            .entity
                            .base()
                            .attrs
                            .get(&name)
                            .map(|v| {
                                v.split(';')
                                    .map(str::trim)
                                    .filter(|h| !h.is_empty() && *h != id)
                                    .collect::<Vec<_>>()
                                    .join(kentos_contracts::MULTIPLE_JOIN)
                            })
                            .unwrap_or_default()
                    });
                    if let Some(rest) = left.as_ref().filter(|rest| !rest.is_empty()) {
                        let mut entity = (*child.entity).clone();
                        entity.base_mut().attrs.insert(name.clone(), rest.clone());
                        let op = Op::Update {
                            after: Stored {
                                uid: child.uid,
                                entity: Arc::new(entity),
                            },
                            before: child,
                        };
                        self.apply_op(&op);
                        out.push(op);
                        continue;
                    }
                    let op = match r.on_delete {
                        RelationDelete::Cascade => {
                            if !seen.insert(child.uid) {
                                continue;
                            }
                            gone.push((r.child.clone(), child.uid));
                            Op::Remove(child)
                        }
                        _ => {
                            let mut entity = (*child.entity).clone();
                            entity.base_mut().attrs.remove(&name);
                            Op::Update {
                                after: Stored {
                                    uid: child.uid,
                                    entity: Arc::new(entity),
                                },
                                before: child,
                            }
                        }
                    };
                    self.apply_op(&op);
                    out.push(op);
                }
            }
        }
        out
    }

    /// What deleting the objects `uids` would do through the project's relations (docs/adr/0440 §1),
    /// as [`Document::follow_relations`] does it, writing nothing: each relation's unlinked and deleted
    /// children, the attachments that would go, the first relation that would touch a locked layer's
    /// child and how many children would follow.
    pub fn relation_effects(&self, uids: &[Uuid]) -> RelationEffects {
        let mut out = RelationEffects::default();
        let mut gone: Vec<(String, Uuid)> = Vec::new();
        let mut seen: HashSet<Uuid> = HashSet::new();
        for uid in uids {
            if let Some(s) = self
                .store
                .slot_of(*uid)
                .and_then(|slot| self.store.get(slot))
                && seen.insert(*uid)
            {
                out.attachments += s.entity.base().attachments.len() as u32;
                gone.push((s.entity.base().layer_id.clone(), *uid));
            }
        }
        // The children's link values as the walk has left them.
        let mut left: std::collections::HashMap<Uuid, String> = std::collections::HashMap::new();
        let mut counts: Vec<(String, u32, u32)> = Vec::new();
        while let Some((layer, parent)) = gone.pop() {
            let id = parent.to_string();
            for r in &self.settings.relations {
                if r.parent != layer
                    || matches!(r.on_delete, RelationDelete::Keep | RelationDelete::Restrict)
                {
                    continue;
                }
                let Some((name, many)) = self
                    .layers
                    .get(&r.child)
                    .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
                    .map(|f| (f.name.clone(), f.multiple))
                else {
                    continue;
                };
                for child in self.store.on_layer(&r.child) {
                    if seen.contains(&child.uid) {
                        continue;
                    }
                    let value = left
                        .get(&child.uid)
                        .cloned()
                        .or_else(|| child.entity.base().attrs.get(&name).cloned());
                    let Some(value) = value.filter(|v| holds_id(v, &id)) else {
                        continue;
                    };
                    if self.layers.is_locked(&r.child) {
                        out.locked
                            .get_or_insert_with(|| (r.name.clone(), r.child.clone()));
                        continue;
                    }
                    let rest = if many {
                        value
                            .split(';')
                            .map(str::trim)
                            .filter(|h| !h.is_empty() && *h != id)
                            .collect::<Vec<_>>()
                            .join(kentos_contracts::MULTIPLE_JOIN)
                    } else {
                        String::new()
                    };
                    let at = match counts.iter().position(|(rid, ..)| *rid == r.id) {
                        Some(i) => i,
                        None => {
                            counts.push((r.id.clone(), 0, 0));
                            counts.len() - 1
                        }
                    };
                    if !rest.is_empty() || r.on_delete != RelationDelete::Cascade {
                        counts[at].1 += 1;
                        left.insert(child.uid, rest);
                    } else {
                        counts[at].2 += 1;
                        seen.insert(child.uid);
                        out.attachments += child.entity.base().attachments.len() as u32;
                        gone.push((r.child.clone(), child.uid));
                    }
                    out.children += 1;
                }
            }
        }
        // The relations' order.
        out.followed = self
            .settings
            .relations
            .iter()
            .filter_map(|r| counts.iter().find(|(id, ..)| *id == r.id).cloned())
            .collect();
        out
    }

    /// The children a `restrict` relation keeps the objects `uids` from being deleted for: each parent
    /// that has one, with the relation's name (docs/adr/0405 §2).
    pub fn restricted_by(&self, uids: &[Uuid]) -> Vec<(Uuid, String)> {
        let mut out = Vec::new();
        for uid in uids {
            let Some(e) = self
                .store
                .slot_of(*uid)
                .and_then(|slot| self.store.get(slot))
            else {
                continue;
            };
            let layer = e.entity.base().layer_id.clone();
            let id = uid.to_string();
            for r in self
                .settings
                .relations
                .iter()
                .filter(|r| r.parent == layer && r.on_delete == RelationDelete::Restrict)
            {
                let Some(name) = self
                    .layers
                    .get(&r.child)
                    .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
                    .map(|f| f.name.as_str())
                else {
                    continue;
                };
                let has = self.store.on_layer(&r.child).any(|s| {
                    !uids.contains(&s.uid)
                        && s.entity
                            .base()
                            .attrs
                            .get(name)
                            .is_some_and(|v| holds_id(v, &id))
                });
                if has {
                    out.push((*uid, r.name.clone()));
                    break;
                }
            }
        }
        out
    }
}

/// What a delete would do through the project's relations (docs/adr/0440 §1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationEffects {
    /// Each relation's id, its unlinked and its deleted children, in the relations' order.
    pub followed: Vec<(String, u32, u32)>,
    pub attachments: u32,
    /// The first relation (its name and child layer) that would change a locked layer's child.
    pub locked: Option<(String, String)>,
    /// The children that would change or go.
    pub children: usize,
}

/// Whether a reference field's value holds `id`: the id itself or one of a many-to-many field's ids
/// (an id has no `;`).
fn holds_id(value: &str, id: &str) -> bool {
    value.split(';').any(|h| h.trim() == id)
}
