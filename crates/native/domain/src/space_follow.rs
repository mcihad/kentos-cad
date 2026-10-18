//! Spaces go with their walls (docs/adr/0342 §3). Before a step is recorded (`commit`, after the openings), every space
//! found from the walls (it has a seed) that the step added or changed, or that a wall the step added, changed or
//! removed reaches, has its boundary found again round its seed by its method: a wall moved, stretched or thickened
//! moves its sides; a seed the walls no longer close round breaks it (`lostEnclosure`) where it stands, and a break
//! that no longer holds goes. A drawn space (no seed) stays as drawn.
//!
//! The changes join the step: one undo takes them back with the rest. `scripts/fixtures/space_command_cases.py`
//! writes the same without KentOS's code.

use std::sync::Arc;

use kentos_contracts::{Entity, SpaceBreak, SpaceFields, SpaceMethod, SpacePart, SpaceRing, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::space::{self, Method};
use kentos_geometry_core::geom::wall::Spec as WallSpec;
use kentos_geometry_core::geom::wall_join::WallIn;

use crate::document::Document;
use crate::history::Op;
use crate::identity::Slot;
use crate::store::Stored;

/// A box: min x, min y, max x, max y.
type Box4 = [f64; 4];

/// A wall as the core reads it: its path, bulges and spec.
pub struct CoreWall {
    pub pts: Vec<CoreVec2>,
    pub bulges: Option<Vec<f64>>,
    pub spec: WallSpec,
}

/// The core's reading of a wall; none for another object.
pub fn core_wall(e: &Entity) -> Option<CoreWall> {
    if !matches!(e, Entity::Wall(_)) {
        return None;
    }
    let json = serde_json::to_string(e).ok()?;
    let Shape::Wall { pts, bulges, wall } =
        CoreEntity::from_json(&Json::parse(&json).ok()?).ok()?.shape
    else {
        return None;
    };
    Some(CoreWall {
        pts,
        bulges,
        spec: wall,
    })
}

/// The core's method.
pub fn method(m: SpaceMethod) -> Method {
    match m {
        SpaceMethod::Net => Method::Net,
        SpaceMethod::Centerline => Method::Centerline,
        SpaceMethod::Gross => Method::Gross,
    }
}

/// The core's area back as the contract's part.
pub fn part(a: &Area) -> SpacePart {
    let ring = |r: &Ring| SpaceRing {
        pts: r.pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect(),
        bulges: r.bulges.clone(),
    };
    SpacePart {
        outer: ring(&a.outer),
        holes: a.holes.iter().map(ring).collect(),
    }
}

/// The space's boundary round `seed` by `method` among `walls`; none when they close nothing round it.
pub fn found_among(walls: &[CoreWall], seed: Vec2, method_of: SpaceMethod) -> Option<SpacePart> {
    let ins: Vec<WallIn<'_>> = walls
        .iter()
        .map(|w| WallIn {
            pts: &w.pts,
            bulges: w.bulges.as_deref(),
            spec: &w.spec,
        })
        .collect();
    space::found(&ins, CoreVec2::new(seed.x, seed.y), method(method_of)).map(|a| part(&a))
}

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

fn space_box(s: &SpaceFields) -> Box4 {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in s
        .parts
        .iter()
        .map(|p| &p.outer)
        .flat_map(|r| &r.pts)
        .chain(s.seed.as_ref())
    {
        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
    }
    b
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
    /// The drawing's walls whose reach meets `area` (grown by a metre), as the core reads them.
    pub fn walls_near(&self, area: Box4) -> Vec<CoreWall> {
        let area = grow(area, 1.0);
        self.entities()
            .filter(|e| wall_box(e).is_some_and(|b| meets(&b, &area)))
            .filter_map(core_wall)
            .collect()
    }

    /// `s` with its boundary found again from the walls round its seed (a drawn space as it is): among the walls near
    /// its boundary first, then, when they close nothing or it has none yet, among all the drawing's walls.
    pub fn space_found(&self, s: &SpaceFields) -> SpaceFields {
        let mut out = s.clone();
        let Some(seed) = s.seed else { return out };
        let near = (!s.parts.is_empty())
            .then(|| found_among(&self.walls_near(grow(space_box(s), 2.0)), seed, s.method))
            .flatten();
        let found = near.or_else(|| {
            let all: Vec<CoreWall> = self.entities().filter_map(core_wall).collect();
            found_among(&all, seed, s.method)
        });
        match found {
            Some(p) => {
                out.parts = vec![p];
                out.broken = None;
            }
            None => out.broken = Some(SpaceBreak::LostEnclosure),
        }
        out
    }

    /// What keeps the spaces with their walls after `ops`, applied: the step takes them in.
    pub(crate) fn follow_spaces(&mut self, ops: &[Op]) -> Vec<Op> {
        let mut boxes: Vec<Box4> = Vec::new();
        let mut own: Vec<Slot> = Vec::new();
        for op in ops {
            match op {
                Op::Add(s) | Op::Remove(s) => {
                    boxes.extend(wall_box(&s.entity));
                    if matches!(*s.entity, Entity::Space(_)) && matches!(op, Op::Add(_)) {
                        own.push(s.slot());
                    }
                }
                Op::Update { before, after } => {
                    boxes.extend(wall_box(&before.entity));
                    boxes.extend(wall_box(&after.entity));
                    if let (Entity::Space(b), Entity::Space(a)) = (&*before.entity, &*after.entity)
                        && (a.space.seed != b.space.seed
                            || a.space.method != b.space.method
                            || a.space.parts != b.space.parts)
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
                Entity::Space(x) => {
                    x.space.seed.is_some()
                        && boxes
                            .iter()
                            .any(|b| meets(b, &grow(space_box(&x.space), 0.5)))
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
            let Entity::Space(s) = &*current.entity else {
                continue;
            };
            if s.space.seed.is_none() {
                continue;
            }
            let found = self.space_found(&s.space);
            if found != s.space {
                let mut e = s.clone();
                e.space = found;
                let op = updated(current, Entity::Space(e));
                self.apply_op(&op);
                out.push(op);
            }
        }
        out
    }
}
