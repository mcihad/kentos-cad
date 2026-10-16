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
                let Some(name) = self
                    .layers
                    .get(&r.child)
                    .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
                    .map(|f| f.name.clone())
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
                            .is_some_and(|v| v.trim() == id)
                    })
                    .cloned()
                    .collect();
                for child in children {
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
                            .is_some_and(|v| v.trim() == id)
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
