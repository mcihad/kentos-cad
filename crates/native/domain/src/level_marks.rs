//! A Kot bound to a level (docs/adr/0721 §2): an elevation mark whose value is its level's floor elevation plus its
//! offset. The levels are project settings (no undo step), so the value is derived, like a view mark's words: written
//!
//! - at a step's end, for the marks the step added or changed (inside the step);
//! - outside any step, in place, when the settings change, and after an undo, a redo or a rollback: every mark whose
//!   value no longer holds. Such a write is no undo step of its own.
//!
//! A level the project lacks leaves the last value (a level an object names cannot be taken away, docs/adr/0720 §3).

use std::collections::BTreeSet;
use std::sync::Arc;

use kentos_contracts::{ArchLevel, DimensionEntity, Entity};

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

/// `d` with its value read from its level among `levels`; none when it already holds or its level is gone.
pub fn settled(d: &DimensionEntity, levels: &[ArchLevel]) -> Option<DimensionEntity> {
    let z = d.level.as_ref()?.value(levels)?;
    (d.za != Some(z)).then(|| DimensionEntity { za: Some(z), ..d.clone() })
}

impl Document {
    /// The updates that write `slots`' Kots again from their levels.
    fn level_mark_updates(&self, slots: impl IntoIterator<Item = Slot>) -> Vec<Op> {
        let levels = self.settings().arch.as_ref().map_or(&[][..], |a| a.levels.as_slice());
        let mut out = Vec::new();
        for slot in slots {
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Dimension(d) = &*current.entity else {
                continue;
            };
            if let Some(next) = settled(d, levels) {
                out.push(Op::Update {
                    after: Stored {
                        uid: current.uid,
                        entity: Arc::new(Entity::Dimension(next)),
                    },
                    before: current,
                });
            }
        }
        out
    }

    /// What writes the Kots `ops` added or changed from their levels, applied: the step takes them in.
    pub(crate) fn follow_level_marks(&mut self, ops: &[Op]) -> Vec<Op> {
        if !self.store.has_level_marks() {
            return Vec::new();
        }
        let own: BTreeSet<Uuid> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Add(s) | Op::Update { after: s, .. }
                    if matches!(&*s.entity, Entity::Dimension(d) if d.level.is_some()) =>
                {
                    Some(s.uid)
                }
                _ => None,
            })
            .collect();
        let slots: Vec<_> = own.iter().filter_map(|u| self.store.slot_of(*u)).collect();
        let out = self.level_mark_updates(slots);
        for op in &out {
            self.apply_op(op);
        }
        out
    }

    /// Every Kot bound to a level written again in place, outside the undo history: after the settings changed, an
    /// undo, a redo or a rollback. Returns how many changed.
    pub(crate) fn refresh_level_marks(&mut self) -> usize {
        if !self.store.has_level_marks() {
            return 0;
        }
        let slots: Vec<_> = self.store.level_mark_slots().collect();
        let ops = self.level_mark_updates(slots);
        for op in &ops {
            self.apply_op(op);
        }
        ops.len()
    }
}
