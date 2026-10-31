//! A slab's outline bound to its source (Kaynağa bağlı sınır, docs/adr/0771 §4). Before a step is recorded (after the
//! spaces, before the slabs' edges), every bound slab the step added or changed, whose space it changed, or that a wall
//! it added, changed or removed comes near, has its outline found again: round its seed among the walls on its level
//! (their outer faces, or the room's inner faces) or as its space's rings, every edge moved by its offset. What it
//! cannot find (the walls no longer close round the seed, the offset turns a ring over, the space is gone) leaves the
//! last outline as it was and marks it broken; a break that no longer holds goes. The changes join the step: one undo
//! takes them back with the rest, and the slab's edge follows them (`slab_follow`). A slab without `bound` stays as
//! drawn. `scripts/fixtures/bound_slab_cases.py` writes the same without KentOS's code.

use std::sync::Arc;

use kentos_contracts::{
    Entity, SlabBound, SlabBreak, SlabFace, SlabFields, SlabSourceKind, SpacePart,
};

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::slab_follow::{core_parts, slab_box};
use crate::store::Stored;

/// A box: min x, min y, max x, max y.
type Box4 = [f64; 4];

fn grow(b: Box4, by: f64) -> Box4 {
    [b[0] - by, b[1] - by, b[2] + by, b[3] + by]
}

fn meets(a: &Box4, b: &Box4) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

fn wall_box(e: &Entity) -> Option<Box4> {
    let Entity::Wall(w) = e else { return None };
    let r = w.wall.width + w.wall.offset.abs();
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in &w.wall.pts {
        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
    }
    Some(grow(b, r))
}

fn parts_box(parts: &[SpacePart]) -> Box4 {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in parts.iter().flat_map(|p| &p.outer.pts) {
        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
    }
    b
}

impl Document {
    /// A bound slab's outline as its source gives it now, and the walls whose faces bound it (by their persistent ids,
    /// in the drawing's order); or why it cannot be found. `level` is the slab's.
    pub fn bound_outline(
        &self,
        b: &SlabBound,
        level: Option<&str>,
    ) -> Result<(Vec<SpacePart>, Vec<String>), SlabBreak> {
        let found = match b.source {
            SlabSourceKind::Walls => {
                let seed = b.seed.ok_or(SlabBreak::LostEnclosure)?;
                let part = self
                    .slab_from_walls(seed, level, b.face == SlabFace::Room)
                    .ok_or(SlabBreak::LostEnclosure)?;
                vec![part]
            }
            SlabSourceKind::Space => {
                let uid = b
                    .space
                    .as_deref()
                    .and_then(|u| Uuid::parse_str(u).ok())
                    .ok_or(SlabBreak::SourceGone)?;
                match self.slot_of(uid).and_then(|s| self.get(s)) {
                    Some(Entity::Space(s)) => s.space.parts.clone(),
                    _ => return Err(SlabBreak::SourceGone),
                }
            }
        };
        let probe = SlabFields {
            parts: found,
            ..SlabFields::default()
        };
        let parts = if b.offset == 0.0 {
            probe.parts
        } else {
            kentos_geometry_core::geom::slab::offset(&core_parts(&probe), b.offset)
                .map_err(|_| SlabBreak::LostEnclosure)?
                .iter()
                .map(crate::space_follow::part)
                .collect()
        };
        let uids = match b.source {
            SlabSourceKind::Walls => {
                let near = grow(parts_box(&parts), 1e-6);
                self.entities()
                    .filter(|e| crate::space_follow::takes_level(e, level))
                    .filter(|e| wall_box(e).is_some_and(|w| meets(&w, &near)))
                    .filter_map(|e| self.uid(Slot(e.base().id)).map(|u| u.to_string()))
                    .collect()
            }
            SlabSourceKind::Space => Vec::new(),
        };
        Ok((parts, uids))
    }

    /// `s` with its bound outline found again; the same when it has none or it is as found.
    pub fn slab_bound_again(&self, s: &SlabFields) -> SlabFields {
        let Some(b) = &s.bound else { return s.clone() };
        let mut out = s.clone();
        let mut bound = b.clone();
        match self.bound_outline(b, s.arch.level.as_deref()) {
            Ok((parts, uids)) => {
                out.parts = parts;
                bound.uids = uids;
                bound.broken = None;
            }
            Err(why) => bound.broken = Some(why),
        }
        out.bound = Some(bound);
        out
    }

    pub(crate) fn follow_bound_slabs(&mut self, ops: &[Op]) -> Vec<Op> {
        let mut boxes: Vec<Box4> = Vec::new();
        let mut spaces: Vec<String> = Vec::new();
        let mut own: Vec<Slot> = Vec::new();
        let mut rebound: Vec<Slot> = Vec::new();
        let mut note = |s: &Stored, added: bool| {
            boxes.extend(wall_box(&s.entity));
            if let Entity::Space(_) = &*s.entity {
                spaces.push(s.uid.to_string());
            }
            if added && matches!(&*s.entity, Entity::Slab(x) if x.slab.bound.is_some()) {
                own.push(s.slot());
            }
        };
        for op in ops {
            match op {
                Op::Add(s) => note(s, true),
                Op::Remove(s) => note(s, false),
                Op::Update { before, after } => {
                    note(before, false);
                    note(after, false);
                    if let (Entity::Slab(b), Entity::Slab(a)) = (&*before.entity, &*after.entity)
                        && a.slab.bound.is_some()
                        && (b.slab.bound != a.slab.bound || b.slab.arch.level != a.slab.arch.level)
                    {
                        rebound.push(after.slot());
                    }
                }
                _ => {}
            }
        }
        own.extend(rebound);
        if boxes.is_empty() && spaces.is_empty() && own.is_empty() {
            return Vec::new();
        }
        let mut slots: Vec<Slot> = self
            .store
            .iter()
            .filter(|s| match &*s.entity {
                Entity::Slab(x) => x.slab.bound.as_ref().is_some_and(|b| {
                    b.space.as_ref().is_some_and(|u| spaces.contains(u))
                        || (b.source == SlabSourceKind::Walls
                            && boxes
                                .iter()
                                .any(|w| meets(w, &grow(slab_box(&x.slab), 0.5))))
                }),
                _ => false,
            })
            .map(Stored::slot)
            .collect();
        slots.extend(own);
        slots.sort();
        slots.dedup();
        let mut out = Vec::new();
        for slot in slots {
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Slab(s) = &*current.entity else {
                continue;
            };
            let followed = self.slab_bound_again(&s.slab);
            if followed != s.slab {
                let mut e = s.clone();
                e.slab = followed;
                out.push(Op::Update {
                    after: Stored {
                        uid: current.uid,
                        entity: Arc::new(Entity::Slab(e)),
                    },
                    before: current,
                });
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}
