//! A slab's edge follows its walls (docs/adr/0722 §3). Before a step is recorded, every slab with an edge that the step
//! added or changed (its rings, its level or its edge), or that a wall the step added, changed or removed comes near,
//! has its edge's runs written again: the edges of its rings whose middle no wall's face on its level runs along (all
//! walls when it has no level; `geom::slab::free_runs`). The changes join the step: one undo takes them back with the
//! rest. `scripts/fixtures/slab_cases.py` writes the same without KentOS's code.

use std::sync::Arc;

use kentos_contracts::{Entity, SlabFields, SlabRun, SpacePart, SpaceRing, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::wall_join::WallIn;

use crate::document::Document;
use crate::history::Op;
use crate::identity::Slot;
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
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for p in &w.wall.pts {
        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
    }
    Some(grow(b, r))
}

/// A slab's box: its outer rings'.
pub fn slab_box(s: &SlabFields) -> Box4 {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for p in s.parts.iter().flat_map(|p| &p.outer.pts) {
        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
    }
    b
}

fn core_ring(r: &SpaceRing) -> Ring {
    Ring {
        pts: r.pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect(),
        bulges: r.bulges.clone(),
    }
}

/// A part as the core's area.
pub fn core_area(p: &SpacePart) -> Area {
    Area {
        outer: core_ring(&p.outer),
        holes: p.holes.iter().map(core_ring).collect(),
    }
}

/// The core's areas of a slab's parts.
pub fn core_parts(s: &SlabFields) -> Vec<Area> {
    s.parts.iter().map(core_area).collect()
}

/// The core's runs as the contract's.
pub fn contract_runs(rs: &[kentos_geometry_core::geom::slab::Run]) -> Vec<SlabRun> {
    rs.iter()
        .map(|r| SlabRun {
            pts: r.pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect(),
            bulges: r.bulges.clone(),
        })
        .collect()
}

/// Whether the step's change to a slab asks its runs again.
fn edge_asks(before: &SlabFields, after: &SlabFields) -> bool {
    after.edge.is_some()
        && (before.parts != after.parts
            || before.arch.level != after.arch.level
            || before.edge.as_ref().map(|e| (e.kind, e.width)) != after.edge.as_ref().map(|e| (e.kind, e.width))
            || before.edge.as_ref().map(|e| &e.runs) != after.edge.as_ref().map(|e| &e.runs))
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
    /// The outline a slab found from the walls round `seed` has (docs/adr/0722 §2), among the walls on `level` (every
    /// wall when it has none): with `room` the room round it to their inner faces (with its holes), else the outermost
    /// outline of their line work round it to their outer faces. None when they enclose nothing round it.
    pub fn slab_from_walls(&self, seed: Vec2, level: Option<&str>, room: bool) -> Option<SpacePart> {
        // Only the pieces of wall work round the seed (docs/adr/0782 §2): the same answer, to the bit.
        match crate::wall_pieces::walls_round(self, seed, level) {
            Some(walls) => Self::slab_among(&walls, seed, room),
            None => self.slab_from_every_wall(seed, level, room),
        }
    }

    /// [`Self::slab_from_walls`] read among every wall of the level (what the pieces must answer alike).
    pub fn slab_from_every_wall(&self, seed: Vec2, level: Option<&str>, room: bool) -> Option<SpacePart> {
        let walls: Vec<crate::space_follow::CoreWall> = self
            .entities()
            .filter(|e| crate::space_follow::takes_level(e, level))
            .filter_map(crate::space_follow::core_wall)
            .collect();
        Self::slab_among(&walls, seed, room)
    }

    fn slab_among(walls: &[crate::space_follow::CoreWall], seed: Vec2, room: bool) -> Option<SpacePart> {
        let ins: Vec<WallIn<'_>> = walls
            .iter()
            .map(|w| WallIn {
                pts: &w.pts,
                bulges: w.bulges.as_deref(),
                spec: &w.spec,
            })
            .collect();
        let at = CoreVec2::new(seed.x, seed.y);
        let found = if room {
            kentos_geometry_core::geom::slab::room(&ins, at)
        } else {
            kentos_geometry_core::geom::slab::outline(&ins, at)
        }?;
        Some(crate::space_follow::part(&found))
    }

    /// The runs of a slab's free sides among the drawing's walls on its level (every wall when it has none): the edges
    /// of its rings whose middle no wall's face runs along.
    pub fn slab_runs(&self, s: &SlabFields) -> Vec<SlabRun> {
        let walls = self.space_walls_near(slab_box(s), s.arch.level.as_deref());
        let ins: Vec<WallIn<'_>> = walls
            .iter()
            .map(|w| WallIn {
                pts: &w.pts,
                bulges: w.bulges.as_deref(),
                spec: &w.spec,
            })
            .collect();
        let faces = kentos_geometry_core::geom::space::wall_faces(&ins);
        contract_runs(&kentos_geometry_core::geom::slab::free_runs(&core_parts(s), &faces))
    }

    /// `s` with its edge's runs written again; the same when it has no edge.
    pub fn slab_with_runs(&self, s: &SlabFields) -> SlabFields {
        let mut out = s.clone();
        if out.edge.is_some() {
            let runs = self.slab_runs(s);
            if let Some(e) = &mut out.edge {
                e.runs = runs;
            }
        }
        out
    }

    pub(crate) fn follow_slabs(&mut self, ops: &[Op]) -> Vec<Op> {
        let mut boxes: Vec<Box4> = Vec::new();
        let mut own: Vec<Slot> = Vec::new();
        for op in ops {
            match op {
                Op::Add(s) | Op::Remove(s) => {
                    boxes.extend(wall_box(&s.entity));
                    if matches!(&*s.entity, Entity::Slab(x) if x.slab.edge.is_some()) && matches!(op, Op::Add(_)) {
                        own.push(s.slot());
                    }
                }
                Op::Update { before, after } => {
                    boxes.extend(wall_box(&before.entity));
                    boxes.extend(wall_box(&after.entity));
                    if let (Entity::Slab(b), Entity::Slab(a)) = (&*before.entity, &*after.entity)
                        && edge_asks(&b.slab, &a.slab)
                    {
                        own.push(after.slot());
                    }
                }
                _ => {}
            }
        }
        if boxes.is_empty() && own.is_empty() {
            return Vec::new();
        }
        let mut slots: Vec<Slot> = self
            .store
            .iter()
            .filter(|s| match &*s.entity {
                Entity::Slab(x) => {
                    x.slab.edge.is_some() && boxes.iter().any(|b| meets(b, &grow(slab_box(&x.slab), 0.5)))
                }
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
            let followed = self.slab_with_runs(&s.slab);
            if followed != s.slab {
                let mut e = s.clone();
                e.slab = followed;
                out.push(updated(current, Entity::Slab(e)));
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}
