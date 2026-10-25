//! Spaces go with their walls (docs/adr/0342 §3). Before a step is recorded (`commit`, after the openings), every space
//! found from the walls (it has a seed) that the step added or changed, or that a wall the step added, changed or
//! removed reaches, has its boundary found again round its seed by its method: a wall moved, stretched or thickened
//! moves its sides; a seed the walls no longer close round breaks it (`lostEnclosure`) where it stands, and a break
//! that no longer holds goes. A drawn space (no seed) stays as drawn.
//!
//! The changes join the step: one undo takes them back with the rest. `scripts/fixtures/space_command_cases.py`
//! writes the same without KentOS's code.

use std::collections::BTreeMap;
use std::sync::Arc;

use kentos_contracts::{Entity, SpaceBreak, SpaceFields, SpaceMethod, SpacePart, SpaceRing, Vec2};
use kentos_geometry_core::geom::{arch, region};
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

fn column_box(e: &Entity) -> Option<Box4> {
    let Entity::Column(c) = e else { return None };
    let c = &c.column;
    let r = c.width.hypot(c.depth()) / 2.;
    let spec = arch::ColumnSpec {round:c.shape == kentos_contracts::ColumnShape::Round,
        width:c.width,depth:c.depth,rotation:c.rotation,..Default::default()};
    if c.problem().is_some() || arch::column_area(CoreVec2::new(c.at.x,c.at.y),&spec).is_none()
        || ![c.at.x,c.at.y,r].iter().all(|n|n.is_finite()) {
        // An invalid changed source must reach every opting-in space, never
        // disappear because its numeric bounds cannot be indexed.
        return Some([f64::NEG_INFINITY,f64::NEG_INFINITY,f64::INFINITY,f64::INFINITY]);
    }
    Some([c.at.x-r,c.at.y-r,c.at.x+r,c.at.y+r])
}

fn core_parts(parts: &[SpacePart]) -> Vec<Area> {
    let ring = |r: &SpaceRing| Ring { pts: r.pts.iter().map(|p| CoreVec2::new(p.x,p.y)).collect(), bulges:r.bulges.clone() };
    parts.iter().map(|p|Area {outer:ring(&p.outer),holes:p.holes.iter().map(ring).collect()}).collect()
}

// The old seed search box is deliberately unchanged. Column invalidation and
// footprint pruning need the analytic extrema, including two-vertex circles.
fn column_space_box(s: &SpaceFields) -> Box4 {
    let mut out = [f64::INFINITY,f64::INFINITY,f64::NEG_INFINITY,f64::NEG_INFINITY];
    for a in core_parts(&s.parts) {
        for e in region::ring_edges(&a.outer) {
            let b = kentos_geometry_core::geom::arrangement::tight_box(&e);
            out = [out[0].min(b.min_x),out[1].min(b.min_y),out[2].max(b.max_x),out[3].max(b.max_y)];
        }
    }
    if let Some(p) = s.seed { out=[out[0].min(p.x),out[1].min(p.y),out[2].max(p.x),out[3].max(p.y)]; }
    out
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

fn takes_level(e: &Entity, level: Option<&str>) -> bool {
    level.is_none_or(|id| matches!(e, Entity::Wall(w) if w.wall.arch.level.as_deref() == Some(id)))
}

type ColumnAreas = Result<Vec<(Area, Box4)>, SpaceBreak>;

/// An immutable source view for one plan or follower pass. Borrowing the
/// document prevents wall edits from invalidating a cached boundary network.
pub struct SpaceSearch<'a> {
    doc: &'a Document,
    indexes: BTreeMap<Vec<Slot>, kentos_geometry_core::geom::space_index::BoundaryIndex>,
    columns: BTreeMap<Option<String>, ColumnAreas>,
}

impl<'a> SpaceSearch<'a> {
    pub fn new(doc: &'a Document) -> Self {
        Self {
            doc,
            indexes: BTreeMap::new(),
            columns: BTreeMap::new(),
        }
    }

    fn among(&mut self, slots: Vec<Slot>, seed: Vec2, how: SpaceMethod) -> Option<SpacePart> {
        let index = self.indexes.entry(slots.clone()).or_insert_with(|| {
            let walls: Vec<_> = slots
                .iter()
                .filter_map(|&s| self.doc.get(s))
                .filter_map(core_wall)
                .collect();
            let ins: Vec<_> = walls
                .iter()
                .map(|w| WallIn {
                    pts: &w.pts,
                    bulges: w.bulges.as_deref(),
                    spec: &w.spec,
                })
                .collect();
                kentos_geometry_core::geom::space_index::BoundaryIndex::for_queries(&ins)
        });
        index
            .found(CoreVec2::new(seed.x, seed.y), method(how))
            .map(|a| part(&a))
    }

    /// The selected physical footprints in this immutable source view. Geometry
    /// stays analytic; render fill, layer visibility and grid-break flags do not
    /// change the occupied footprint. A broken grid keeps its last real place.
    pub fn deduct_columns(&mut self, s: &SpaceFields) -> Result<Vec<SpacePart>, SpaceBreak> {
        if !s.columns.applies(s.method) { return Ok(s.parts.clone()); }
        let level = s.arch.level.clone();
        let columns = self.columns.entry(level.clone()).or_insert_with(|| {
            self.doc.entities().filter_map(|e| match e {
                Entity::Column(c) if level.as_ref().is_none_or(|id| c.column.arch.level.as_ref() == Some(id)) => Some(&c.column),
                _ => None,
            }).map(|c| {
                if c.problem().is_some() { return Err(SpaceBreak::InvalidBoundarySource); }
                let spec = arch::ColumnSpec { round: c.shape == kentos_contracts::ColumnShape::Round,
                    width: c.width, depth: c.depth, rotation: c.rotation, ..Default::default() };
                let at = CoreVec2::new(c.at.x,c.at.y);
                let area = arch::column_area(at,&spec).ok_or(SpaceBreak::InvalidBoundarySource)?;
                let r = if spec.round { c.width / 2. } else { c.width.hypot(c.depth()) / 2. };
                Ok((area,[at.x-r,at.y-r,at.x+r,at.y+r]))
            }).collect()
        });
        let columns = columns.as_ref().map_err(|e| *e)?;
        let bounds = column_space_box(s);
        let cutters: Vec<_> = columns.iter().filter(|(_,b)| meets(b,&bounds)).map(|(a,_)|a.clone()).collect();
        if cutters.is_empty() { return Ok(s.parts.clone()); }
        let areas = core_parts(&s.parts);
        let cut = region::subtract_areas(&areas,&cutters);
        if cut.is_empty() { return Err(SpaceBreak::NoFreeArea); }
        Ok(cut.iter().map(part).collect())
    }

    /// Preserve the existing near-first/full-fallback search, sharing a network
    /// only when its exact ordered set of wall sources is identical.
    pub fn found(&mut self, s: &SpaceFields) -> SpaceFields {
        let mut out = s.clone();
        let Some(seed) = s.seed else { return out };
        let eligible =
            |e: &Entity| takes_level(e, s.arch.level.as_deref()) && matches!(e, Entity::Wall(_));
        let near = if s.parts.is_empty() {
            None
        } else {
            let area = grow(grow(space_box(s), 2.0), 1.0);
            let slots = self
                .doc
                .entities()
                .filter(|e| eligible(e) && wall_box(e).is_some_and(|b| meets(&b, &area)))
                .map(|e| Slot(e.base().id))
                .collect();
            self.among(slots, seed, s.method)
        };
        let found = near.or_else(|| {
            let slots = self
                .doc
                .entities()
                .filter(|e| eligible(e))
                .map(|e| Slot(e.base().id))
                .collect();
            self.among(slots, seed, s.method)
        });
        match found {
            Some(p) => {
                out.parts = vec![p];
                match self.deduct_columns(&out) {
                    Ok(parts) => { out.parts = parts; out.broken = None; }
                    Err(why) => { out.parts.clone_from(&s.parts); out.broken = Some(why); }
                }
            }
            None => out.broken = Some(SpaceBreak::LostEnclosure),
        }
        out
    }
}

impl Document {
    /// The drawing's walls whose reach meets `area` (grown by a metre), as the core reads them.
    pub fn walls_near(&self, area: Box4) -> Vec<CoreWall> {
        self.space_walls_near(area, None)
    }

    fn space_walls_near(&self, area: Box4, level: Option<&str>) -> Vec<CoreWall> {
        let area = grow(area, 1.0);
        self.entities()
            .filter(|e| takes_level(e, level))
            .filter(|e| wall_box(e).is_some_and(|b| meets(&b, &area)))
            .filter_map(core_wall)
            .collect()
    }

    /// Find the seeded boundary among its explicit level's walls (all levels for legacy unassigned spaces).
    /// Search near the previous boundary first, then the whole eligible level; drawn spaces stay unchanged.
    pub fn space_found(&self, s: &SpaceFields) -> SpaceFields {
        if !s.columns.is_ignore() { return SpaceSearch::new(self).found(s); }
        let mut out = s.clone();
        let Some(seed) = s.seed else { return out };
        // An explicit semantic level never borrows a boundary from another
        // level, even when their elevations coincide. Unassigned legacy spaces
        // retain the original whole-drawing lookup; no saved space is migrated.
        let level = s.arch.level.as_deref();
        let near = (!s.parts.is_empty())
            .then(|| {
                found_among(
                    &self.space_walls_near(grow(space_box(s), 2.0), level),
                    seed,
                    s.method,
                )
            })
            .flatten();
        let found = near.or_else(|| {
            let all: Vec<CoreWall> = self
                .entities()
                .filter(|e| takes_level(e, level))
                .filter_map(core_wall)
                .collect();
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
        let mut column_boxes: Vec<Box4> = Vec::new();
        let mut own: Vec<Slot> = Vec::new();
        for op in ops {
            match op {
                Op::Add(s) | Op::Remove(s) => {
                    boxes.extend(wall_box(&s.entity));
                    column_boxes.extend(column_box(&s.entity));
                    if matches!(*s.entity, Entity::Space(_)) && matches!(op, Op::Add(_)) {
                        own.push(s.slot());
                    }
                }
                Op::Update { before, after } => {
                    boxes.extend(wall_box(&before.entity));
                    boxes.extend(wall_box(&after.entity));
                    column_boxes.extend(column_box(&before.entity));
                    column_boxes.extend(column_box(&after.entity));
                    if let (Entity::Space(b), Entity::Space(a)) = (&*before.entity, &*after.entity)
                        && (a.space.seed != b.space.seed
                            || a.space.method != b.space.method
                            || a.space.arch.level != b.space.arch.level
                            || a.space.parts != b.space.parts
                            || a.space.columns != b.space.columns)
                    {
                        own.push(after.slot());
                    }
                }
                _ => {}
            }
        }
        if boxes.is_empty() && column_boxes.is_empty() && own.is_empty() {
            return Vec::new();
        }
        let mut slots: Vec<Slot> = self
            .store
            .iter()
            .filter(|s| match &*s.entity {
                Entity::Space(x) => {
                    x.space.seed.is_some()
                        && (boxes.iter().any(|b| meets(b, &grow(space_box(&x.space), 0.5)))
                            || (x.space.columns.applies(x.space.method)
                                && column_boxes.iter().any(|b| meets(b, &grow(column_space_box(&x.space), 0.5)))))
                }
                _ => false,
            })
            .map(Stored::slot)
            .collect();
        slots.extend(own);
        slots.sort();
        slots.dedup();
        let mut out = Vec::new();
        let mut search = SpaceSearch::new(self);
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
            let found = search.found(&s.space);
            if found != s.space {
                let mut e = s.clone();
                e.space = found;
                let op = updated(current, Entity::Space(e));
                out.push(op);
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}
