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

use kentos_contracts::{Entity, TagBreak, TagFields, Vec2};

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

    /// `t` written again from its object: its words, the point it ties to and, when `follow`, its place moved as far as
    /// that point went.
    pub fn tag_settled(&self, t: &TagFields, follow: bool) -> TagFields {
        let mut out = t.clone();
        match self.tag_target(&t.of) {
            Some(target) => {
                out.text = self.tag_text(&target, t.content.as_deref());
                let anchor = self.tag_anchor(&target);
                if follow && let (Some(old), Some(new)) = (t.anchor, anchor) {
                    out.at = Vec2 {
                        x: t.at.x + (new.x - old.x),
                        y: t.at.y + (new.y - old.y),
                    };
                }
                out.anchor = anchor;
                out.broken = None;
            }
            None => out.broken = Some(TagBreak::TargetGone),
        }
        out
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
            let settled = self.tag_settled(&t.tag, !own.contains(&uid));
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
                self.apply_op(&op);
                out.push(op);
            }
        }
        if ops
            .iter()
            .any(|op| matches!(op, Op::TemplateSettings { .. }))
        {
            out.extend(self.elevation_tag_updates());
        }
        out
    }

    /// These new templates depend on settings outside ordinary model undo.
    /// Refresh their derived words only: manually placed geometry stays intact.
    fn elevation_tag_updates(&mut self) -> Vec<Op> {
        let slots: Vec<_> = self.store.tag_slots().collect();
        let mut ops = Vec::new();
        for slot in slots {
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Tag(t) = &*current.entity else {
                continue;
            };
            let content = t.tag.content.as_deref().unwrap_or_default();
            if !(content.contains("{floorElevation}") || content.contains("{ceilingElevation}")) {
                continue;
            }
            let Some(target) = self.tag_target(&t.tag.of) else {
                continue;
            };
            let text = self.tag_text(&target, t.tag.content.as_deref());
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
            self.apply_op(&op);
            ops.push(op);
        }
        ops
    }

    /// Settings, rollback and history may leave a stored derived string stale.
    /// Like view marks this updates the derived cache without another undo step.
    pub(crate) fn refresh_elevation_tags(&mut self) -> usize {
        self.elevation_tag_updates().len()
    }
}
