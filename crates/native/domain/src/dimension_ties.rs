//! Associative dimensions (docs/adr/0249 §3): a dimension's points `a`, `b`
//! and `c` may follow places on objects (`refs`). Before a step is recorded
//! (`commit`), after the linked texts and the associative hatches, its
//! changes are read:
//!
//! - a dimension one of whose objects the step changed or removed has each
//!   live reference to that object followed by the core's rule
//!   (`ops::subref::follow`): the point goes where the place went, the place
//!   is rebound when the object's structure changed, and a reference that
//!   breaks keeps its point and says why (`broken`);
//! - a dimension whose point the step itself moved (moved alone, its end
//!   dragged by a grip) while that point's object did not change lets that
//!   point go: its reference is dropped, the point is free.
//!
//! The changes join the step: one undo takes them back with the rest.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use kentos_contracts::{DimensionEntity, DimensionRefs, Entity, SubPlace, SubRef, Vec2};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::ops::subref::{Followed, Place, follow};

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

/// The geometry core's reading of an object (the contract's own JSON).
fn core(e: &Entity) -> Option<CoreEntity> {
    let json = serde_json::to_string(e).ok()?;
    CoreEntity::from_json(&Json::parse(&json).ok()?).ok()
}

/// The core's place of a contract's place.
pub fn core_place(p: SubPlace) -> Place {
    let n = |x: u32| x as usize;
    match p {
        SubPlace::Node { part } => Place::Node { part: n(part) },
        SubPlace::Start => Place::Start,
        SubPlace::End => Place::End,
        SubPlace::Center => Place::Center,
        SubPlace::Vertex { part, ring, index } => Place::Vertex {
            part: n(part),
            ring: n(ring),
            index: n(index),
        },
        SubPlace::Mid { part, ring, index } => Place::Mid {
            part: n(part),
            ring: n(ring),
            index: n(index),
        },
    }
}

/// The contract's place of a core's place.
pub fn contract_place(p: Place) -> SubPlace {
    let n = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
    match p {
        Place::Node { part } => SubPlace::Node { part: n(part) },
        Place::Start => SubPlace::Start,
        Place::End => SubPlace::End,
        Place::Center => SubPlace::Center,
        Place::Vertex { part, ring, index } => SubPlace::Vertex {
            part: n(part),
            ring: n(ring),
            index: n(index),
        },
        Place::Mid { part, ring, index } => SubPlace::Mid {
            part: n(part),
            ring: n(ring),
            index: n(index),
        },
    }
}

/// A dimension's point by its name.
fn point_of(d: &DimensionEntity, name: &str) -> Option<Vec2> {
    match name {
        "a" => Some(d.a),
        "b" => Some(d.b),
        _ => d.c,
    }
}

fn set_point(d: &mut DimensionEntity, name: &str, p: Vec2) {
    match name {
        "a" => d.a = p,
        "b" => d.b = p,
        _ => d.c = Some(p),
    }
}

fn reference_mut<'a>(r: &'a mut DimensionRefs, name: &str) -> &'a mut Option<SubRef> {
    match name {
        "a" => &mut r.a,
        "b" => &mut r.b,
        _ => &mut r.c,
    }
}

/// Where a step left each object it changed or removed: before and after (none: removed).
type Changed = BTreeMap<Uuid, (Arc<Entity>, Option<Arc<Entity>>)>;

impl Document {
    /// What keeps the associative dimensions with their objects after `ops` (a
    /// step about to be recorded), applied: the step takes them in.
    pub(crate) fn follow_dimensions(&mut self, ops: &[Op]) -> Vec<Op> {
        if !self.store.has_ties() {
            return Vec::new();
        }
        // The objects the step changed or removed, as they were first and are now; and the dimensions it
        // changed itself, as they were.
        let mut changed: Changed = BTreeMap::new();
        let mut own: BTreeMap<Slot, Arc<Entity>> = BTreeMap::new();
        for op in ops {
            match op {
                Op::Update { before, after } => {
                    changed
                        .entry(after.uid)
                        .or_insert_with(|| (Arc::clone(&before.entity), None))
                        .1 = Some(Arc::clone(&after.entity));
                    if matches!(*before.entity, Entity::Dimension(_)) {
                        own.entry(after.slot())
                            .or_insert_with(|| Arc::clone(&before.entity));
                    }
                }
                Op::Remove(s) => {
                    changed
                        .entry(s.uid)
                        .or_insert_with(|| (Arc::clone(&s.entity), None))
                        .1 = None;
                }
                _ => {}
            }
        }
        // The dimensions to look at: tied to a changed object, or changed themselves.
        let mut slots: Vec<Slot> = Vec::new();
        let mut seen: HashSet<Slot> = HashSet::new();
        for uid in changed.keys() {
            for slot in self.store.tied_to(*uid).collect::<Vec<_>>() {
                if seen.insert(slot) {
                    slots.push(slot);
                }
            }
        }
        for slot in own.keys() {
            if seen.insert(*slot) {
                slots.push(*slot);
            }
        }
        let mut out = Vec::new();
        for slot in slots {
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Dimension(d) = &*current.entity else {
                continue;
            };
            let Some(refs) = &d.refs else {
                continue;
            };
            let was = own.get(&slot).and_then(|e| match &**e {
                Entity::Dimension(w) => Some(w.clone()),
                _ => None,
            });
            let mut next = d.clone();
            let mut next_refs = refs.clone();
            for (name, r) in refs.each() {
                if !r.live() {
                    continue;
                }
                let of = Uuid::from_bytes(r.uid.0);
                match changed.get(&of) {
                    Some((before, after)) => {
                        let (Some(b), a) = (core(before), after.as_ref().and_then(|a| core(a)))
                        else {
                            continue;
                        };
                        // An object the step removed has no after; one it replaced by another kind breaks.
                        let followed = if after.is_some() && a.is_none() {
                            Followed::Broken(kentos_geometry_core::ops::subref::Reason::KindChanged)
                        } else {
                            follow(core_place(r.place), &b, a.as_ref())
                        };
                        let slot_ref = reference_mut(&mut next_refs, name);
                        match followed {
                            Followed::At(place, p) => {
                                set_point(&mut next, name, Vec2 { x: p.x, y: p.y });
                                if let Some(s) = slot_ref {
                                    s.place = contract_place(place);
                                }
                            }
                            Followed::Broken(reason) => {
                                if let Some(s) = slot_ref {
                                    s.broken = Some(reason.code().to_owned());
                                }
                            }
                        }
                    }
                    None => {
                        // Its object did not change: a point the step moved itself is free now.
                        if let Some(w) = &was
                            && point_of(w, name) != point_of(d, name)
                        {
                            *reference_mut(&mut next_refs, name) = None;
                        }
                    }
                }
            }
            next.refs = (!next_refs.is_empty()).then_some(next_refs);
            if next != *d {
                out.push(updated(current, next));
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}

fn updated(before: Stored, after: DimensionEntity) -> Op {
    Op::Update {
        after: Stored {
            uid: before.uid,
            entity: Arc::new(Entity::Dimension(after)),
        },
        before,
    }
}
