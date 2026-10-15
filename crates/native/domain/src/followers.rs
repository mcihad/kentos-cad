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
use crate::identity::Slot;
use crate::store::Stored;

impl Document {
    /// Whether anything in the drawing follows other objects (a linked text,
    /// an associative hatch or dimension): when not, no change carries any.
    pub fn can_follow(&self) -> bool {
        self.store.has_links() || self.store.has_ties()
    }

    /// The objects that would follow `changes` (each a slot and the object it
    /// would become), as they would be after following, in the order the
    /// step's end writes them; an object written twice is given once, as it
    /// ends. A changed object that follows too (a dimension one of whose points
    /// is tied) is among them. Nothing is written.
    pub fn followers(&self, changes: &[(Slot, Entity)]) -> Vec<(Slot, Entity)> {
        if !self.may_follow(changes) {
            return Vec::new();
        }
        self.reading_copy().followers_here(changes)
    }

    /// [`Document::followers`] on `scratch`, a copy of this drawing the
    /// caller keeps between calls (a tool's preview at every move), left as it
    /// was; the copy is made the first time it is needed or when it is not
    /// this drawing's as it is now (another generation).
    pub fn followers_with(
        &self,
        scratch: &mut Option<Document>,
        changes: &[(Slot, Entity)],
    ) -> Vec<(Slot, Entity)> {
        if !self.may_follow(changes) {
            return Vec::new();
        }
        let current = scratch
            .as_ref()
            .is_some_and(|s| s.generation == self.generation && s.edits == self.edits);
        if !current {
            *scratch = Some(self.reading_copy());
        }
        match scratch {
            Some(copy) => copy.followers_here(changes),
            None => Vec::new(),
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
                _ => false,
            };
            self.store.linked_to(before.uid).next().is_some()
                || self.store.tied_to(before.uid).next().is_some()
                || ties(&before.entity)
                || ties(after)
        })
    }

    /// Applies `changes`, follows them as a step's end would, reads what
    /// followed and takes it all back.
    fn followers_here(&mut self, changes: &[(Slot, Entity)]) -> Vec<(Slot, Entity)> {
        let mut ops: Vec<Op> = Vec::with_capacity(changes.len());
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
        // The last of each slot, in the order they first came.
        let mut order: Vec<Slot> = Vec::new();
        let mut last: HashMap<Slot, Entity> = HashMap::new();
        for op in &ops[mark..] {
            if let Op::Update { after, .. } = op {
                let slot = after.slot();
                if last.insert(slot, (*after.entity).clone()).is_none() {
                    order.push(slot);
                }
            }
        }
        self.revert(&ops);
        order
            .into_iter()
            .filter_map(|s| last.remove(&s).map(|e| (s, e)))
            .collect()
    }
}
