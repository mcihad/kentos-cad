//! Tags go with their objects (docs/adr/0344 §1). Before a step is recorded (`commit`, after the columns), every tag
//! the step added or changed, and every tag of an object the step added, changed or removed, has its words and the
//! point it ties to written again from its object (`arch_values`). A tag the step did not move itself goes as far as
//! that point went; one it did keeps the place it was given. An object gone breaks its tags (`targetGone`), their last
//! words kept; a break that no longer holds goes.
//!
//! The changes join the step: one undo takes them back with the rest. `scripts/fixtures/tag_command_cases.py` writes
//! the same without KentOS's code.

use std::collections::BTreeSet;
use std::sync::Arc;

use kentos_contracts::{Entity, TagFields};
use crate::tag_read::TagRead;

use crate::document::Document;
use crate::history::Op;
use crate::identity::Uuid;
use crate::store::Stored;

impl Document {
    /// The object a tag's `of` names, as the document holds it.
    pub fn tag_target(&self, of: &str) -> Option<Entity> {
        let slot = self.store.slot_of(Uuid::parse_str(of).ok()?)?;
        Some((*self.store.get(slot)?.entity).clone())
    }

    /// The spaces that have a tag (docs/adr/0344's 27 October addendum), in slot order: their tag is their label, they
    /// draw none of their own (as an object whose label a text writes, docs/adr/0175 §4).
    pub fn tagged_spaces(&self) -> Vec<crate::identity::Slot> {
        let mut out: Vec<crate::identity::Slot> = self
            .store
            .tag_slots()
            .filter_map(|t| {
                let Entity::Tag(tag) = &*self.store.get(t)?.entity else {
                    return None;
                };
                let slot = self.store.slot_of(Uuid::parse_str(&tag.tag.of).ok()?)?;
                matches!(&*self.store.get(slot)?.entity, Entity::Space(_)).then_some(slot)
            })
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// `t` written again from its object: its words, the point it ties to and, when `follow`, its place moved as far as
    /// that point went.
    pub fn tag_settled(&self, t: &TagFields, follow: bool) -> TagFields {
        TagRead::new(self).settled(t, follow)
    }

    /// What keeps the tags with their objects after `ops`, applied: the step takes them in.
    pub(crate) fn follow_tags(&mut self, ops: &[Op]) -> Vec<Op> {
        if !self.store.has_tags() {
            return Vec::new();
        }
        let mut own: BTreeSet<Uuid> = BTreeSet::new();
        let mut touched: BTreeSet<Uuid> = BTreeSet::new();
        for op in ops {
            let (Op::Add(s) | Op::Remove(s) | Op::Update { after: s, .. }) = op else {
                continue;
            };
            if matches!(&*s.entity, Entity::Tag(_)) {
                if !matches!(op, Op::Remove(_)) {
                    own.insert(s.uid);
                }
            } else {
                touched.insert(s.uid);
            }
        }
        let mut tags: BTreeSet<Uuid> = own.clone();
        for u in &touched {
            for slot in self.store.tied_to(*u).collect::<Vec<_>>() {
                if let Some(s) = self.store.get(slot)
                    && matches!(*s.entity, Entity::Tag(_))
                {
                    tags.insert(s.uid);
                }
            }
        }
        let mut out = Vec::new();
        let read = TagRead::new(self);
        for uid in tags {
            let Some(current) = self
                .store
                .slot_of(uid)
                .and_then(|s| self.store.get(s))
                .cloned()
            else {
                continue;
            };
            let Entity::Tag(t) = &*current.entity else {
                continue;
            };
            let settled = read.settled(&t.tag, !own.contains(&uid));
            if settled != t.tag {
                let mut e = t.clone();
                e.tag = settled;
                let op = Op::Update {
                    after: Stored {
                        uid: current.uid,
                        entity: Arc::new(Entity::Tag(e)),
                    },
                    before: current,
                };
                out.push(op);
            }
        }
        for op in &out { self.apply_op(op); }
        let settings = ops.iter().any(|op| matches!(op, Op::TemplateSettings { .. }));
        let sources = ops.iter().any(|op| match op {
            Op::Add(s) | Op::Remove(s) => crate::space_follow::is_source(&s.entity),
            Op::Update { before, after } => crate::space_follow::is_source(&before.entity) || crate::space_follow::is_source(&after.entity),
            _ => false,
        });
        // A column may change net area while a gross room's stored parts stay identical.
        if settings || sources { out.extend(self.derived_tag_updates(!settings)); }
        out
    }

    /// These templates depend on settings or room sources outside the stored room outline.
    /// Refresh their derived words only: manually placed geometry stays intact.
    fn derived_tag_updates(&mut self, net_only: bool) -> Vec<Op> {
        let slots: Vec<_> = self.store.tag_slots().collect();
        let mut ops = Vec::new();
        let read = TagRead::new(self);
        for slot in slots {
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Tag(t) = &*current.entity else {
                continue;
            };
            let content = t.tag.content.as_deref().unwrap_or_default();
            if !(content.contains("{netArea}") || (!net_only && (content.contains("{floorElevation}") || content.contains("{ceilingElevation}")))) {
                continue;
            }
            let Some(target) = self.tag_target(&t.tag.of) else {
                continue;
            };
            let text = read.text(&target, t.tag.content.as_deref());
            if t.tag.text == text {
                continue;
            }
            let mut e = t.clone();
            e.tag.text = text;
            let op = Op::Update {
                after: Stored {
                    uid: current.uid,
                    entity: Arc::new(Entity::Tag(e)),
                },
                before: current,
            };
            ops.push(op);
        }
        for op in &ops { self.apply_op(op); }
        ops
    }

    /// Settings, rollback and history may leave a stored derived string stale.
    /// Like view marks this updates the derived cache without another undo step.
    pub(crate) fn refresh_derived_tags(&mut self) -> usize {
        self.derived_tag_updates(false).len()
    }
}
