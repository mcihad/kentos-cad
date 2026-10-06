//! Associative hatches (docs/adr/0186 §6): a hatch made inside a closed
//! object with İlişkili on knows its objects (`assoc`: the closed object,
//! its islands, the texts and inserts left open, and the point clicked
//! inside). Before a step is recorded (`commit`), after the linked texts,
//! its changes are read:
//!
//! - a hatch one of whose objects the step changed or removed has its region
//!   cut again by the core's rule (`ops::hatch_region`), its pattern and
//!   colour kept; a removed island or cutout leaves the list;
//! - a hatch whose closed object is gone, no longer closed, or leaves no
//!   region stays where it is and follows nothing;
//! - a hatch whose ring or holes the step itself changed (a grip, the vertex
//!   table) follows nothing, unless its objects changed in the same step
//!   (then the rule writes it).
//!
//! The changes join the step: one undo takes them back with the rest. The
//! web's is model/hatchTies.ts; both run
//! `fixtures/document-ops/v1/hatch-ties.json`.

use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use kentos_contracts::{Entity, EntityId, HatchAssoc, HatchEntity, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::block::Blocks;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::ops::hatch_region::{cutout, region};

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::linked::font_of;
use crate::store::Stored;

/// The geometry core's reading of an object (the contract's own JSON).
fn core(e: &Entity) -> Option<CoreEntity> {
    let json = serde_json::to_string(e).ok()?;
    CoreEntity::from_json(&Json::parse(&json).ok()?).ok()
}

/// A hatch that follows nothing: it stays as it is.
fn untied(h: &HatchEntity) -> HatchEntity {
    HatchEntity {
        assoc: None,
        ..h.clone()
    }
}

fn point(p: &CoreVec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

impl Document {
    /// The hatch `h` cut again from its objects now, or none when the rule
    /// gives no region (its closed object gone or open, nothing left).
    fn retied(&self, h: &HatchEntity) -> Option<HatchEntity> {
        let a = h.assoc.as_ref()?;
        let object = |id: &EntityId| {
            self.store
                .slot_of(Uuid::from_bytes(id.0))
                .and_then(|s| self.store.get(s))
                .map(|s| Arc::clone(&s.entity))
        };
        let outer = core(&*object(&a.outer)?)?;
        let present = |ids: &[EntityId]| -> Vec<(EntityId, Arc<Entity>)> {
            ids.iter()
                .filter_map(|id| object(id).map(|e| (*id, e)))
                .collect()
        };
        let islands = present(&a.islands);
        let cutouts = present(&a.cutouts);
        let shapes: Vec<Shape> = islands
            .iter()
            .filter_map(|(_, e)| core(e).map(|c| c.shape))
            .collect();
        // An insert's box needs its block's definition.
        let blocks = if cutouts
            .iter()
            .any(|(_, e)| matches!(**e, Entity::Insert(_)))
        {
            let defs: Vec<&kentos_contracts::BlockDefinition> =
                self.blocks().iter().map(|b| &**b).collect();
            serde_json::to_string(&defs)
                .ok()
                .and_then(|t| Json::parse(&t).ok())
                .and_then(|j| Blocks::from_json(&j).ok())
                .unwrap_or_default()
        } else {
            Blocks::default()
        };
        let font = font_of(self.settings().drawing_font);
        let boxes: Vec<Vec<CoreVec2>> = cutouts
            .iter()
            .map(|(_, e)| {
                core(e)
                    .and_then(|c| cutout(&c.shape, &blocks, font))
                    .unwrap_or_default()
            })
            .collect();
        let r = region(
            &outer.shape,
            &shapes,
            &boxes,
            CoreVec2::new(a.seed.x, a.seed.y),
        )?;
        Some(HatchEntity {
            ring: r.ring.iter().map(point).collect(),
            holes: (!r.holes.is_empty()).then(|| {
                r.holes
                    .iter()
                    .map(|h| h.iter().map(point).collect())
                    .collect()
            }),
            assoc: Some(HatchAssoc {
                outer: a.outer,
                islands: islands.into_iter().map(|(id, _)| id).collect(),
                cutouts: cutouts.into_iter().map(|(id, _)| id).collect(),
                seed: a.seed,
            }),
            ..h.clone()
        })
    }

    /// What keeps the associative hatches with their objects after `ops` (a
    /// step about to be recorded), applied: the step takes them in.
    pub(crate) fn follow_hatches(&mut self, ops: &[Op]) -> Vec<Op> {
        if !self.store.has_ties() {
            return Vec::new();
        }
        // The objects the step changed or removed, by persistent id, and the
        // hatches whose ring or holes it changed itself, by slot.
        let mut objects: Vec<Uuid> = Vec::new();
        let mut seen: HashSet<Uuid> = HashSet::new();
        let mut edited: BTreeSet<Slot> = BTreeSet::new();
        for op in ops {
            let (stored, before) = match op {
                Op::Add(s) | Op::Remove(s) => (s, None),
                Op::Update { before, after } => (after, Some(before)),
                _ => continue,
            };
            if seen.insert(stored.uid) {
                objects.push(stored.uid);
            }
            if let (Some(before), Entity::Hatch(after)) = (before, &*stored.entity)
                && let Entity::Hatch(was) = &*before.entity
                && after.assoc.is_some()
                && (was.ring != after.ring || was.holes != after.holes)
            {
                edited.insert(stored.slot());
            }
        }
        let mut out = Vec::new();
        let mut done: HashSet<Slot> = HashSet::new();
        for uid in objects {
            let tied: Vec<Slot> = self.store.tied_to(uid).collect();
            for slot in tied {
                if !done.insert(slot) {
                    continue;
                }
                let Some(current) = self.store.get(slot).cloned() else {
                    continue;
                };
                let Entity::Hatch(h) = &*current.entity else {
                    continue;
                };
                let next = self.retied(h).unwrap_or_else(|| untied(h));
                if next != *h {
                    out.push(updated(current, next));
                }
            }
        }
        // A hatch edited by itself, its objects unchanged: it follows nothing now.
        for slot in edited {
            if done.contains(&slot) {
                continue;
            }
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            if let Entity::Hatch(h) = &*current.entity
                && h.assoc.is_some()
            {
                let next = untied(h);
                out.push(updated(current, next));
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}

fn updated(before: Stored, after: HatchEntity) -> Op {
    Op::Update {
        after: Stored {
            uid: before.uid,
            entity: Arc::new(Entity::Hatch(after)),
        },
        before,
    }
}
