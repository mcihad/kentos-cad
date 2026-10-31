//! What a step's changes would carry along (docs/adr/0297 §3): the linked
//! texts (linked.rs), the associative hatches (hatch_ties.rs) and the
//! associative dimensions (dimension_ties.rs) a step making them would take
//! in at its end, as they would be. Nothing is written: a command's plan and
//! a tool's preview show them before the step.
//!
//! The answer is the step's own rule: the changes are applied to a copy
//! (`reading_copy`, or one the caller keeps between previews), followed as a
//! step's end follows them, and taken back. When nothing in the drawing can
//! follow them (no links or ties, none of the changed objects tied, linked or
//! tying), no copy is made.

use std::collections::HashMap;
use std::sync::Arc;

use kentos_contracts::Entity;

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, new_uid};
use crate::store::Stored;

/// All derived changes, including source-driven membership changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FollowerDelta {
    pub added: Vec<(Slot, Entity)>,
    pub updated: Vec<(Slot, Entity)>,
    pub removed: Vec<Slot>,
}

impl Document {
    /// Whether anything in the drawing follows other objects (a linked text,
    /// an associative hatch or dimension): when not, no change carries any.
    pub fn can_follow(&self) -> bool {
        self.store.has_links() || self.store.has_ties() || !self.dimension_sets.is_empty()
    }

    /// The objects that would follow `changes` (each a slot and the object it
    /// would become), as they would be after following, in the order the
    /// step's end writes them; an object written twice is given once, as it
    /// ends. A changed object that follows too (a dimension one of whose points
    /// is tied) is among them. Nothing is written.
    pub fn followers(&self, changes: &[(Slot, Entity)]) -> Vec<(Slot, Entity)> {
        self.followers_delta(changes).updated
    }

    pub fn followers_with(
        &self,
        scratch: &mut Option<Document>,
        changes: &[(Slot, Entity)],
    ) -> Vec<(Slot, Entity)> {
        self.followers_delta_with(scratch, changes).updated
    }

    pub fn followers_delta(&self, changes: &[(Slot, Entity)]) -> FollowerDelta {
        self.preview_followers(changes, &[], &[])
    }

    pub fn followers_delta_with(
        &self,
        scratch: &mut Option<Document>,
        changes: &[(Slot, Entity)],
    ) -> FollowerDelta {
        self.preview_followers_with(scratch, changes, &[], &[])
    }

    /// Derived geometry of an edit, insertion or deletion. Added source identities are virtual and
    /// exist only in the scratch drawing; executing a command allocates its real persistent identities.
    pub fn preview_followers(
        &self,
        updates: &[(Slot, Entity)],
        adds: &[Entity],
        removes: &[Slot],
    ) -> FollowerDelta {
        if adds.is_empty() && removes.is_empty() && !self.may_follow(updates) {
            return FollowerDelta::default();
        }
        if !self.can_follow() {
            return FollowerDelta::default();
        }
        self.reading_copy().followers_here(updates, adds, removes)
    }

    /// [`preview_followers`] even where no tie asks it: what a change does to everything that follows it (the spaces,
    /// the bound slabs; docs/adr/0771 §2's effects).
    pub fn preview_all_followers(&self, updates: &[(Slot, Entity)]) -> FollowerDelta {
        if !self.can_follow() {
            return FollowerDelta::default();
        }
        self.reading_copy().followers_here(updates, &[], &[])
    }

    pub fn preview_followers_with(
        &self,
        scratch: &mut Option<Document>,
        updates: &[(Slot, Entity)],
        adds: &[Entity],
        removes: &[Slot],
    ) -> FollowerDelta {
        if adds.is_empty() && removes.is_empty() && !self.may_follow(updates) {
            return FollowerDelta::default();
        }
        if !self.can_follow() {
            return FollowerDelta::default();
        }
        let current = scratch
            .as_ref()
            .is_some_and(|s| s.generation == self.generation && s.edits == self.edits);
        if !current {
            *scratch = Some(self.reading_copy());
        }
        match scratch {
            Some(copy) => copy.followers_here(updates, adds, removes),
            None => FollowerDelta::default(),
        }
    }

    /// Whether anything could follow `changes`: an object tied or linked to a
    /// changed one, or a changed object that ties or links itself.
    fn may_follow(&self, changes: &[(Slot, Entity)]) -> bool {
        if !self.can_follow() {
            return false;
        }
        changes.iter().any(|(slot, after)| {
            let Some(before) = self.store.get(*slot) else {
                return false;
            };
            let ties = |e: &Entity| match e {
                Entity::Dimension(d) => d.refs.is_some(),
                Entity::Text(t) => t.label_of.is_some(),
                Entity::Hatch(h) => h.assoc.is_some(),
                Entity::Opening(_) => true,
                Entity::Column(c) => c.column.grids.is_some(),
                Entity::Tag(_) => true,
                _ => false,
            };
            (!self.dimension_sets.is_empty()
                && (matches!(&*before.entity, Entity::Wall(_) | Entity::Opening(_))
                    || self.dimension_set_of_item(before.uid).is_some()))
                || self.store.linked_to(before.uid).next().is_some()
                || self.store.tied_to(before.uid).next().is_some()
                || ties(&before.entity)
                || ties(after)
        })
    }

    /// Applies `changes`, follows them as a step's end would, reads what
    /// followed and takes it all back.
    fn followers_here(
        &mut self,
        changes: &[(Slot, Entity)],
        adds: &[Entity],
        removes: &[Slot],
    ) -> FollowerDelta {
        if self.next_slot + adds.len() as u64 > u64::from(u32::MAX) + 1 {
            return FollowerDelta::default();
        }
        let first_slot = self.next_slot;
        let clock = self.store.preview_clock();
        let candidates = self.store.wall_candidates.clone();
        let relocated = self.relocated.clone();
        let relocated_ends = self.relocated_ends.clone();
        // Preview writes are invisible even to readers holding the scratch drawing's change mark.
        let journal = std::mem::take(&mut self.history.journal);
        let mut ops: Vec<Op> = Vec::with_capacity(changes.len() + adds.len() + removes.len());
        for slot in removes {
            if let Some(stored) = self.store.get(*slot).cloned() {
                let op = Op::Remove(stored);
                self.apply_op(&op);
                ops.push(op);
            }
        }
        for entity in adds {
            let mut entity = entity.clone();
            entity.base_mut().id = self.next_slot as u32;
            self.next_slot += 1;
            let op = Op::Add(Stored {
                uid: new_uid(),
                entity: Arc::new(entity),
            });
            self.apply_op(&op);
            ops.push(op);
        }
        let mut latest: HashMap<Slot, Stored> = HashMap::new();
        for (slot, entity) in changes {
            let Some(before) = latest.get(slot).or_else(|| self.store.get(*slot)).cloned() else {
                continue;
            };
            let after = Stored {
                uid: before.uid,
                entity: Arc::new(crate::edit::changed(entity.clone(), *slot, &before.entity)),
            };
            if after.entity == before.entity {
                continue;
            }
            latest.insert(*slot, after.clone());
            let op = Op::Update { before, after };
            self.apply_op(&op);
            ops.push(op);
        }
        let mark = ops.len();
        self.follow_into(&mut ops);
        // First and final states of each derived slot. A new source itself belongs to the
        // caller's input; only new followers are emitted as added geometry.
        let mut order = Vec::new();
        let mut initial: HashMap<Slot, Option<Arc<Entity>>> = HashMap::new();
        for op in &ops[mark..] {
            let (slot, before) = match op {
                Op::Add(s) => (s.slot(), None),
                Op::Remove(s) => (s.slot(), Some(s.entity.clone())),
                Op::Update { before, .. } => (before.slot(), Some(before.entity.clone())),
                _ => continue,
            };
            initial.entry(slot).or_insert_with(|| {
                order.push(slot);
                before
            });
        }
        let mut delta = FollowerDelta::default();
        for slot in order {
            let before = initial.remove(&slot).flatten();
            let after = self.get(slot);
            match (before, after) {
                (None, Some(after)) => delta.added.push((slot, after.clone())),
                (Some(_), None) => delta.removed.push(slot),
                (Some(before), Some(after)) if *before != *after => {
                    delta.updated.push((slot, after.clone()))
                }
                _ => {}
            }
        }
        let end_slot = self.next_slot;
        self.revert(&ops);
        self.store
            .restore_preview_clock(clock, first_slot, end_slot);
        self.store.wall_candidates = candidates;
        self.next_slot = first_slot;
        self.relocated = relocated;
        self.relocated_ends = relocated_ends;
        self.history.journal = journal;
        delta
    }
}
