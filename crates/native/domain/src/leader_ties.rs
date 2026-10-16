//! Multileaders (docs/adr/0326 §2, §3): a leader's arm tips may follow places on objects, and its content blocks (inserts)
//! stand in a row at its landing's end. Before a step is recorded (`commit`, after the arrays, before the
//! dimensions), its changes are read:
//!
//! - a tip whose object the step changed or removed follows the core's rule (`ops::subref::follow`): it goes where the
//!   place went, or keeps its point and says why (`broken`); a tip the step itself moved, its object unchanged, lets
//!   its place go;
//! - a content block the step removed leaves its leader's list; a leader the step removed takes its blocks with it;
//! - a leader the step added or changed (by itself or by a tip above) has its blocks written where they stand: the
//!   first one's definition box's side towards the landing at the landing's end, its box's middle on the landing's
//!   line, each after the one before, turned as the note.
//!
//! The changes join the step: one undo takes them back with the rest. `scripts/fixtures/leader_command_cases.py`
//! writes the same without KentOS's code.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use kentos_contracts::{BlockDefinition, Entity, InsertEntity, LeaderEntity, SubRef, Vec2};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::{Entity as CoreEntity, entity_bounds};
use kentos_geometry_core::geom::leader::layout_of;
use kentos_geometry_core::jsmath::{PI, cos, sin};
use kentos_geometry_core::ops::subref::{Followed, follow};

use crate::dimension_ties::{contract_place, core_place};
use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

/// The geometry core's reading of an object (the contract's own JSON).
fn core(e: &Entity) -> Option<CoreEntity> {
    let json = serde_json::to_string(e).ok()?;
    CoreEntity::from_json(&Json::parse(&json).ok()?).ok()
}

/// A block definition's box about its base point: `[x₀, x₁, y₀, y₁]`; none for one that draws nothing.
pub fn definition_box(b: &BlockDefinition) -> Option<[f64; 4]> {
    let (mut x0, mut x1, mut y0, mut y1) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for e in &b.entities {
        let Some(c) = core(e) else { continue };
        let r = entity_bounds(&c.shape);
        if !(r.min_x.is_finite()
            && r.max_x.is_finite()
            && r.min_y.is_finite()
            && r.max_y.is_finite())
        {
            continue;
        }
        x0 = x0.min(r.min_x - b.base.x);
        x1 = x1.max(r.max_x - b.base.x);
        y0 = y0.min(r.min_y - b.base.y);
        y1 = y1.max(r.max_y - b.base.y);
    }
    (x0 <= x1 && y0 <= y1).then_some([x0, x1, y0, y1])
}

/// Where a leader's content blocks stand (docs/adr/0326 §2): for each insert, its point and turn; none for a leader
/// without a landing. `boxes` gives an insert's definition box.
pub fn block_places(
    l: &LeaderEntity,
    inserts: &[&InsertEntity],
    boxes: &dyn Fn(&InsertEntity) -> Option<[f64; 4]>,
) -> Option<Vec<(Vec2, f64)>> {
    let shape = core(&Entity::Leader(l.clone()))?.shape;
    let layout = layout_of(&shape)?;
    let [_, end] = layout.landing?;
    let r = l.rotation * PI / 180.0;
    let (u, v) = ((cos(r), sin(r)), (-sin(r), cos(r)));
    let mut acc = 0.0;
    let mut out = Vec::with_capacity(inserts.len());
    for ins in inserts {
        let [x0, x1, y0, y1] = boxes(ins).unwrap_or([0.0, 0.0, 0.0, 0.0]);
        let (s, sy) = (ins.scale, ins.y_scale.unwrap_or(ins.scale));
        let along = if layout.side > 0.0 {
            acc - s * x0
        } else {
            -acc - s * x1
        };
        let up = -sy * (y0 + y1) / 2.0;
        out.push((
            Vec2 {
                x: end.x + u.0 * along + v.0 * up,
                y: end.y + u.1 * along + v.1 * up,
            },
            l.rotation,
        ));
        acc += s * (x1 - x0);
    }
    Some(out)
}

/// A leader's tips: the first arm's, then each other arm's.
fn tips(l: &LeaderEntity) -> Vec<Option<&SubRef>> {
    std::iter::once(l.tip.as_ref())
        .chain(l.arms.iter().map(|a| a.tip.as_ref()))
        .collect()
}

/// Arm `k`'s tip point (0 the first arm's).
fn tip_point(l: &LeaderEntity, k: usize) -> Option<Vec2> {
    if k == 0 {
        l.pts.first().copied()
    } else {
        l.arms.get(k - 1)?.pts.first().copied()
    }
}

fn set_tip(l: &mut LeaderEntity, k: usize, p: Vec2, r: Option<SubRef>) {
    if k == 0 {
        if let Some(first) = l.pts.first_mut() {
            *first = p;
        }
        l.tip = r;
    } else if let Some(a) = l.arms.get_mut(k - 1) {
        if let Some(first) = a.pts.first_mut() {
            *first = p;
        }
        a.tip = r;
    }
}

fn updated(before: Stored, after: Entity) -> Op {
    Op::Update {
        after: Stored {
            uid: before.uid,
            entity: Arc::new(after),
        },
        before,
    }
}

impl Document {
    /// A leader with its tips placed on their objects now: each bound tip at its place's point (none for a place the
    /// object does not have).
    pub fn leader_tips_placed(&self, l: &LeaderEntity) -> Option<LeaderEntity> {
        let mut out = l.clone();
        for (k, t) in tips(l).into_iter().enumerate() {
            let Some(t) = t.filter(|t| t.live()) else {
                continue;
            };
            let e = self
                .store
                .slot_of(Uuid::from_bytes(t.uid.0))
                .and_then(|s| self.store.get(s))?;
            let p =
                kentos_geometry_core::ops::subref::point(&core(&e.entity)?, core_place(t.place))?;
            set_tip(&mut out, k, Vec2 { x: p.x, y: p.y }, Some(t.clone()));
        }
        Some(out)
    }

    /// Where the content blocks of `l` stand in this document: each insert's slot with itself placed.
    pub fn leader_blocks_placed(&self, l: &LeaderEntity) -> Vec<(Slot, InsertEntity)> {
        let found: Vec<(Slot, InsertEntity)> = l
            .blocks
            .iter()
            .filter_map(|b| {
                let slot = self.store.slot_of(Uuid::from_bytes(b.0))?;
                match &*self.store.get(slot)?.entity {
                    Entity::Insert(i) => Some((slot, i.clone())),
                    _ => None,
                }
            })
            .collect();
        let boxes = |i: &InsertEntity| self.block(i.block).and_then(definition_box);
        let refs: Vec<&InsertEntity> = found.iter().map(|(_, i)| i).collect();
        let Some(places) = block_places(l, &refs, &boxes) else {
            return Vec::new();
        };
        found
            .into_iter()
            .zip(places)
            .map(|((slot, i), (p, rotation))| (slot, InsertEntity { p, rotation, ..i }))
            .collect()
    }

    /// What keeps the leaders' tips and content blocks with their objects after `ops`, applied: the step takes them in.
    pub(crate) fn follow_leaders(&mut self, ops: &[Op]) -> Vec<Op> {
        // The objects the step changed or removed, as they were first and are now; the leaders it changed itself.
        let mut changed: BTreeMap<Uuid, (Arc<Entity>, Option<Arc<Entity>>)> = BTreeMap::new();
        let mut own: BTreeMap<Uuid, Arc<Entity>> = BTreeMap::new();
        let mut added: BTreeSet<Uuid> = BTreeSet::new();
        let mut gone_leaders: Vec<LeaderEntity> = Vec::new();
        let mut any_leader = false;
        for op in ops {
            match op {
                Op::Add(s) => {
                    if matches!(*s.entity, Entity::Leader(_)) {
                        added.insert(s.uid);
                        any_leader = true;
                    }
                }
                Op::Update { before, after } => {
                    changed
                        .entry(after.uid)
                        .or_insert_with(|| (Arc::clone(&before.entity), None))
                        .1 = Some(Arc::clone(&after.entity));
                    if matches!(*before.entity, Entity::Leader(_)) {
                        own.entry(after.uid)
                            .or_insert_with(|| Arc::clone(&before.entity));
                        any_leader = true;
                    }
                }
                Op::Remove(s) => {
                    changed
                        .entry(s.uid)
                        .or_insert_with(|| (Arc::clone(&s.entity), None))
                        .1 = None;
                    if let Entity::Leader(l) = &*s.entity {
                        gone_leaders.push(l.clone());
                    }
                }
                _ => {}
            }
        }
        if !any_leader && gone_leaders.is_empty() && !self.store.has_ties() {
            return Vec::new();
        }
        // The leaders to look at: tied to a changed object, changed or added themselves.
        let mut leaders: BTreeSet<Uuid> =
            own.keys().copied().chain(added.iter().copied()).collect();
        for uid in changed.keys() {
            for slot in self.store.tied_to(*uid).collect::<Vec<_>>() {
                if let Some(s) = self.store.get(slot)
                    && matches!(*s.entity, Entity::Leader(_))
                {
                    leaders.insert(s.uid);
                }
            }
        }
        let mut out = Vec::new();
        for uid in leaders {
            let Some(current) = self
                .store
                .slot_of(uid)
                .and_then(|s| self.store.get(s))
                .cloned()
            else {
                continue;
            };
            let Entity::Leader(l) = &*current.entity else {
                continue;
            };
            let was = own.get(&uid).and_then(|e| match &**e {
                Entity::Leader(w) => Some(w.clone()),
                _ => None,
            });
            let mut next = l.clone();
            for (k, t) in tips(l).into_iter().enumerate() {
                let Some(t) = t.filter(|t| t.live()).cloned() else {
                    continue;
                };
                let of = Uuid::from_bytes(t.uid.0);
                match changed.get(&of) {
                    Some((before, after)) => {
                        let followed = match (core(before), after.as_ref().map(|a| core(a))) {
                            (Some(b), Some(Some(a))) => follow(core_place(t.place), &b, Some(&a)),
                            (Some(b), None) => follow(core_place(t.place), &b, None),
                            _ => continue,
                        };
                        match followed {
                            Followed::At(place, p) => {
                                let r = SubRef {
                                    place: contract_place(place),
                                    ..t
                                };
                                set_tip(&mut next, k, Vec2 { x: p.x, y: p.y }, Some(r));
                            }
                            Followed::Broken(reason) => {
                                let p = tip_point(&next, k).unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                                set_tip(
                                    &mut next,
                                    k,
                                    p,
                                    Some(SubRef {
                                        broken: Some(reason.code().to_owned()),
                                        ..t
                                    }),
                                );
                            }
                        }
                    }
                    // Its tip moved by the step itself, its object unchanged, its place the one it had: the place lets
                    // go. A step that gives the place too (Bağla) keeps it.
                    None => {
                        if let Some(w) = &was
                            && tip_point(w, k) != tip_point(l, k)
                            && tips(w).get(k).copied().flatten() == Some(&t)
                        {
                            let p = tip_point(&next, k).unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                            set_tip(&mut next, k, p, None);
                        }
                    }
                }
            }
            // Its blocks the step removed leave it.
            next.blocks
                .retain(|b| self.store.slot_of(Uuid::from_bytes(b.0)).is_some());
            if next != *l {
                let op = updated(current.clone(), Entity::Leader(next.clone()));
                self.apply_op(&op);
                out.push(op);
            }
            // Its blocks where they stand.
            for (slot, placed) in self.leader_blocks_placed(&next) {
                let Some(now) = self.store.get(slot).cloned() else {
                    continue;
                };
                if *now.entity != Entity::Insert(placed.clone()) {
                    let op = updated(now, Entity::Insert(placed));
                    self.apply_op(&op);
                    out.push(op);
                }
            }
        }
        // A leader removed takes its content blocks with it, those another leader holds now kept (Topla).
        for l in gone_leaders {
            for b in &l.blocks {
                let of = Uuid::from_bytes(b.0);
                let held = self.store.tied_to(of).any(|t| {
                    matches!(self.store.get(t).map(|s| &*s.entity), Some(Entity::Leader(o)) if o.blocks.contains(b))
                });
                if held {
                    continue;
                }
                if let Some(s) = self
                    .store
                    .slot_of(of)
                    .and_then(|s| self.store.get(s))
                    .cloned()
                {
                    let op = Op::Remove(s);
                    self.apply_op(&op);
                    out.push(op);
                }
            }
        }
        out
    }
}
