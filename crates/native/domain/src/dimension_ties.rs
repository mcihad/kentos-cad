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
//!   point go: its reference is dropped, the point is free; a step that
//!   gives the point a new reference too (`cad.dimensions.reassociate`)
//!   keeps it.
//!
//! A reference to a piece inside an insert (its block `path`, docs/adr/0263
//! §3) follows the piece: when the step changed the insert, or any block
//! definition, the piece as it was (the definitions before the step) and as
//! it is now are followed by the core's rule (`ops::subref::follow_in`).
//!
//! The changes join the step: one undo takes them back with the rest.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

use kentos_contracts::BlockDefinition;
use kentos_contracts::{
    DimensionEntity, DimensionRefs, DimensionRule, DimensionStyle, Entity, EntityId, SubPlace,
    SubRef, Vec2,
};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::block::Blocks;
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::geom::intersect::line_line;
use kentos_geometry_core::jsmath::{atan2, cos, js_hypot, sin};
use kentos_geometry_core::ops::subref::{
    Followed, Place, Reason, curve_centre, edge_line, follow, follow_path, piece, place_near, point,
};

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
        SubPlace::Along {
            part,
            ring,
            index,
            t,
        } => Place::Along {
            part: n(part),
            ring: n(ring),
            index: n(index),
            t,
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
        Place::Along {
            part,
            ring,
            index,
            t,
        } => SubPlace::Along {
            part: n(part),
            ring: n(ring),
            index: n(index),
            t,
        },
    }
}

/// The definitions as the geometry core takes them (the contract's own JSON).
pub fn core_blocks(blocks: &[Arc<BlockDefinition>]) -> Blocks {
    let list: Vec<&BlockDefinition> = blocks.iter().map(|b| &**b).collect();
    let text = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
    Json::parse(&text)
        .ok()
        .and_then(|v| Blocks::from_json(&v).ok())
        .unwrap_or_default()
}

/// A dimension's point by its name.
fn point_of(d: &DimensionEntity, name: &str) -> Option<Vec2> {
    match name {
        "a" => Some(d.a),
        "b" => Some(d.b),
        _ => d.c,
    }
}

fn reference_of<'a>(d: &'a DimensionEntity, name: &str) -> Option<&'a SubRef> {
    let r = d.refs.as_ref()?;
    match name {
        "a" => r.a.as_ref(),
        "b" => r.b.as_ref(),
        _ => r.c.as_ref(),
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

/// How far a relocated point may lie from the object it is looked for on, metres (docs/adr/0296): the pieces are
/// cut where the point was, so it lies on one of them but for rounding.
const RELOCATE_TOLERANCE: f64 = 1e-6;

/// Where a step left each object it changed or removed: before and after (none: removed).
type Changed = BTreeMap<Uuid, (Arc<Entity>, Option<Arc<Entity>>)>;

impl Document {
    /// What keeps the associative dimensions with their objects after `ops` (a
    /// step about to be recorded), applied: the step takes them in.
    pub(crate) fn follow_dimensions(
        &mut self,
        ops: &[Op],
        relocated: &BTreeMap<Uuid, Vec<Uuid>>,
        ends: &BTreeSet<Uuid>,
    ) -> Vec<Op> {
        if !self.store.has_ties() {
            return Vec::new();
        }
        // The objects the step changed or removed, as they were first and are now; and the dimensions it
        // changed itself, as they were.
        let mut changed: Changed = BTreeMap::new();
        let mut own: BTreeMap<Slot, Arc<Entity>> = BTreeMap::new();
        let mut redefined = false;
        for op in ops {
            match op {
                Op::BlockAdd { .. } | Op::BlockRemove { .. } | Op::BlockUpdate { .. } => {
                    redefined = true;
                }
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
        // A changed definition moves the pieces of every insert of it: every dimension that names a piece.
        if redefined {
            for slot in self.store.tied_slots() {
                if seen.insert(slot) {
                    slots.push(slot);
                }
            }
        }
        // The definitions before the step and now, built once when a piece is followed.
        let mut definitions: Option<(Blocks, Blocks)> = None;
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
                if !r.path.is_empty() && (redefined || changed.contains_key(&of)) {
                    let (before, after) = match changed.get(&of) {
                        Some((b, a)) => (Arc::clone(b), a.clone()),
                        None => match self.slot_of(of).and_then(|s| self.store.get(s)) {
                            Some(now) => (Arc::clone(&now.entity), Some(Arc::clone(&now.entity))),
                            None => continue,
                        },
                    };
                    let (was, now) = definitions.get_or_insert_with(|| {
                        (
                            core_blocks(&self.blocks_before(ops)),
                            core_blocks(&self.blocks),
                        )
                    });
                    let Some(b) = core(&before) else {
                        continue;
                    };
                    let a = after.as_ref().and_then(|a| core(a));
                    let (followed, path) = if after.is_some() && a.is_none() {
                        (
                            Followed::Broken(kentos_geometry_core::ops::subref::Reason::KindChanged),
                            r.path.clone(),
                        )
                    } else {
                        follow_path(
                            core_place(r.place),
                            &r.path,
                            (was, &b),
                            a.as_ref().map(|a| (&*now, a)),
                        )
                    };
                    let slot_ref = reference_mut(&mut next_refs, name);
                    match followed {
                        Followed::At(place, p) => {
                            set_point(&mut next, name, Vec2 { x: p.x, y: p.y });
                            if let Some(s) = slot_ref {
                                s.place = contract_place(place);
                                // The piece found again by its local id where it is now (docs/adr/0315 §1).
                                s.path = path;
                            }
                        }
                        Followed::Broken(reason) => {
                            if let Some(s) = slot_ref {
                                s.broken = Some(reason.code().to_owned());
                            }
                        }
                    }
                    continue;
                }
                // Kır's pieces, Birleştir's joined object, Yönü çevir's object (docs/adr/0296): the point
                // keeps its coordinates and is looked for on them.
                if changed.contains_key(&of)
                    && let Some(onto) = relocated.get(&of)
                    && let Some(p) = point_of(d, name)
                {
                    let found = onto.iter().find_map(|u| {
                        let slot = self.slot_of(*u)?;
                        let e = core(&self.store.get(slot)?.entity)?;
                        let at = kentos_geometry_core::Vec2::new(p.x, p.y);
                        place_near(&e, at, RELOCATE_TOLERANCE).map(|(place, _)| (*u, place))
                    });
                    // Buda (docs/adr/0320 §2): an end the cut moved is followed to where it is now.
                    let kept_end = (found.is_none()
                        && ends.contains(&of)
                        && matches!(r.place, SubPlace::Start | SubPlace::End | SubPlace::Vertex { .. }))
                    .then(|| {
                        let (before, after) = changed.get(&of)?;
                        at_of(follow(
                            core_place(r.place),
                            &core(before)?,
                            Some(&core(after.as_ref()?)?),
                        ))
                    })
                    .flatten();
                    if let Some((place, p)) = kept_end {
                        set_point(&mut next, name, Vec2 { x: p.x, y: p.y });
                        if let Some(s) = reference_mut(&mut next_refs, name) {
                            s.place = contract_place(place);
                        }
                        continue;
                    }
                    if let Some(s) = reference_mut(&mut next_refs, name) {
                        match found {
                            Some((u, place)) => {
                                s.uid = EntityId(u.into_bytes());
                                s.place = contract_place(place);
                            }
                            None => s.broken = Some(Reason::VertexGone.code().to_owned()),
                        }
                    }
                    continue;
                }
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
                        // Its object did not change: a point the step moved itself, its reference left as it
                        // was, is free now (a step that binds the point anew keeps the new reference).
                        if let Some(w) = &was
                            && point_of(w, name) != point_of(d, name)
                            && reference_of(w, name) == Some(r)
                        {
                            *reference_mut(&mut next_refs, name) = None;
                        }
                    }
                }
            }
            // A rule whose places a point let go of no longer holds (docs/adr/0320 §1): the dimension is free.
            if next_refs.rule.is_some() && next_refs.rule_problem(d.style).is_some() {
                next_refs = Box::default();
            }
            if !next_refs.is_empty()
                && let Some(rule) = next_refs.rule
            {
                // An angle's arms are not its edges' places, nor need a radius's point be its curve's place (it may lie
                // past an arc's end): the rule alone moves them.
                match rule {
                    DimensionRule::Edges => {
                        next.a = d.a;
                        next.b = d.b;
                    }
                    DimensionRule::Curve
                        if matches!(
                            d.style,
                            Some(
                                DimensionStyle::Radius
                                    | DimensionStyle::Diameter
                                    | DimensionStyle::Jogged
                            )
                        ) =>
                    {
                        next.b = d.b
                    }
                    DimensionRule::Curve => {}
                }
                let prior = was.as_ref().unwrap_or(d);
                self.apply_rule(rule, prior, &mut next, &next_refs, &changed);
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

/// Where a followed place is, if it still is.
fn at_of(f: Followed) -> Option<(Place, kentos_geometry_core::Vec2)> {
    match f {
        Followed::At(place, p) => Some((place, p)),
        Followed::Broken(_) => None,
    }
}

fn wire(p: kentos_geometry_core::Vec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

/// The unit direction from `a` to `b`; none when they meet.
fn unit(a: kentos_geometry_core::Vec2, b: kentos_geometry_core::Vec2) -> Option<(f64, f64)> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l = js_hypot(dx, dy);
    (l > 0.0).then(|| (dx / l, dy / l))
}

impl Document {
    /// The object a reference names as the geometry core reads it now: the object, or the piece its block path
    /// names inside the insert.
    fn object_now(&self, r: &SubRef) -> Option<CoreEntity> {
        let e = core(&self.store.get(self.slot_of(Uuid::from_bytes(r.uid.0))?)?.entity)?;
        if r.path.is_empty() {
            return Some(e);
        }
        piece(&core_blocks(&self.blocks), &e, &r.path)
    }

    /// The object a reference named before the step: as the step found it when it changed it, else as it is.
    fn object_before(&self, r: &SubRef, changed: &Changed) -> Option<CoreEntity> {
        match changed.get(&Uuid::from_bytes(r.uid.0)) {
            Some((before, _)) if r.path.is_empty() => core(before),
            _ => self.object_now(r),
        }
    }

    /// The points a dimension's rule derives (docs/adr/0320 §1) from its bound places as the step left them: a
    /// curve's centre, or two edges' crossing and the arms along them (each arm's length and side as `was`, the
    /// dimension before the step, had them against its edge then). A place that broke, or a curve that is no longer
    /// one, leaves the derived points where they were.
    fn apply_rule(
        &self,
        rule: DimensionRule,
        was: &DimensionEntity,
        next: &mut DimensionEntity,
        refs: &DimensionRefs,
        changed: &Changed,
    ) {
        let live = |r: &Option<SubRef>| r.as_ref().filter(|r| r.live()).cloned();
        match rule {
            DimensionRule::Curve => {
                let radial = matches!(
                    was.style,
                    Some(DimensionStyle::Radius | DimensionStyle::Diameter | DimensionStyle::Jogged)
                );
                let on = if radial {
                    live(&refs.b)
                } else {
                    live(&refs.a).or_else(|| live(&refs.b))
                };
                let Some(r) = on else {
                    return;
                };
                let Some(c) = self
                    .object_now(&r)
                    .and_then(|e| curve_centre(&e, core_place(r.place)))
                else {
                    return;
                };
                if radial {
                    // The point on the curve: its place's, when it was there; else as far round from it as it was.
                    let Some(q) = self
                        .object_now(&r)
                        .and_then(|e| point(&e, core_place(r.place)))
                    else {
                        return;
                    };
                    let q_was = reference_of(was, "b").and_then(|w| {
                        point(&self.object_before(w, changed)?, core_place(w.place))
                    });
                    next.b = match q_was {
                        Some(qw) if (qw.x, qw.y) != (was.b.x, was.b.y) => {
                            let turn = |p: Vec2, o: Vec2| atan2(p.y - o.y, p.x - o.x);
                            let off = turn(was.b, was.a) - turn(wire(qw), was.a);
                            let at = atan2(q.y - c.y, q.x - c.x) + off;
                            let r = js_hypot(q.x - c.x, q.y - c.y);
                            Vec2 {
                                x: c.x + r * cos(at),
                                y: c.y + r * sin(at),
                            }
                        }
                        _ => wire(q),
                    };
                    // A jogged radius's centre shown goes with the centre (docs/adr/0147).
                    if was.style == Some(DimensionStyle::Jogged)
                        && let Some(shown) = next.c
                    {
                        next.c = Some(Vec2 {
                            x: shown.x + (c.x - next.a.x),
                            y: shown.y + (c.y - next.a.y),
                        });
                    }
                    next.a = wire(c);
                } else {
                    next.c = Some(wire(c));
                }
            }
            DimensionRule::Edges => {
                let (Some(ra), Some(rb), Some(c_was)) = (live(&refs.a), live(&refs.b), was.c) else {
                    return;
                };
                let line = |r: &SubRef| {
                    self.object_now(r)
                        .and_then(|e| edge_line(&e, core_place(r.place)))
                };
                let (Some(la), Some(lb)) = (line(&ra), line(&rb)) else {
                    return;
                };
                let Some(hit) = line_line(la.0, la.1, lb.0, lb.1) else {
                    return;
                };
                let c = hit.p;
                // Each arm keeps its length and its side of the vertex along its edge.
                let before = |name: &str| {
                    let r = reference_of(was, name)?;
                    let e = self.object_before(r, changed)?;
                    edge_line(&e, core_place(r.place))
                };
                let arm = |name: &str, now: (kentos_geometry_core::Vec2, kentos_geometry_core::Vec2)| {
                    let p = point_of(was, name)?;
                    let (vx, vy) = (p.x - c_was.x, p.y - c_was.y);
                    let length = js_hypot(vx, vy);
                    let (ox, oy) = before(name).and_then(|(a, b)| unit(a, b))?;
                    let (ux, uy) = unit(now.0, now.1)?;
                    let side = if vx * ox + vy * oy < 0.0 { -1.0 } else { 1.0 };
                    Some(Vec2 {
                        x: c.x + side * length * ux,
                        y: c.y + side * length * uy,
                    })
                };
                let (Some(a), Some(b)) = (arm("a", la), arm("b", lb)) else {
                    return;
                };
                next.a = a;
                next.b = b;
                next.c = Some(wire(c));
            }
        }
    }
}

impl Document {
    /// The block definitions as they were before `ops` (applied): each block step taken back, last first.
    fn blocks_before(&self, ops: &[Op]) -> Vec<Arc<BlockDefinition>> {
        let mut list = self.blocks.clone();
        for op in ops.iter().rev() {
            match op {
                Op::BlockAdd { block, .. } => list.retain(|b| b.id != block.id),
                Op::BlockRemove { index, block } => {
                    list.insert((*index).min(list.len()), Arc::clone(block));
                }
                Op::BlockUpdate { before, after } => {
                    if let Some(b) = list.iter_mut().find(|b| b.id == after.id) {
                        *b = Arc::clone(before);
                    }
                }
                _ => {}
            }
        }
        list
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
