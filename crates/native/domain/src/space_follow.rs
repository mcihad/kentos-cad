//! Spaces go with their walls (docs/adr/0342 §3). Before a step is recorded (`commit`, after the openings), every space
//! found from the walls (it has a seed) that the step added or changed, or that a wall the step added, changed or
//! removed reaches, has its boundary found again round its seed by its method: a wall moved, stretched or thickened
//! moves its sides; a seed the walls no longer close round breaks it (`lostEnclosure`) where it stands, and a break
//! that no longer holds goes. A drawn space (no seed) stays as drawn.
//!
//! The changes join the step: one undo takes them back with the rest. `scripts/fixtures/space_command_cases.py`
//! writes the same without KentOS's code.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use kentos_contracts::{Entity, SpaceBreak, SpaceFields, SpaceMethod, SpacePart, SpaceRing, SpaceScope, Vec2};
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

/// Only an explicitly typed polyline is a virtual boundary. Ordinary linework
/// and visual attributes never opt into room calculations.
pub fn separator_path(e: &Entity) -> Option<&kentos_contracts::PathEntity> {
    match e { Entity::Polyline(p) if p.space_separator.is_some()=>Some(p), _=>None }
}
pub fn is_source(e: &Entity) -> bool {
    matches!(e,Entity::Wall(_) | Entity::Column(_)) || separator_path(e).is_some()
}
pub fn core_separator(e:&Entity)->Option<Shape> {
    let p=separator_path(e)?;
    if p.extras_problem(false).is_some(){return None;}
    Some(Shape::Polyline {pts:p.pts.iter().map(|p|CoreVec2::new(p.x,p.y)).collect(),
        bulges:p.bulges.clone(),closed:p.closed,holes:None,parts:None,widths:None})
}

fn boundary_box(e:&Entity)->Option<Box4> {
    if separator_path(e).is_none(){return wall_box(e);}
    let Some(shape)=core_separator(e) else{return Some([f64::NEG_INFINITY,f64::NEG_INFINITY,f64::INFINITY,f64::INFINITY]);};
    let mut b=[f64::INFINITY,f64::INFINITY,f64::NEG_INFINITY,f64::NEG_INFINITY];
    for edge in kentos_geometry_core::ops::edges::entity_edges(&shape) {
        let r=kentos_geometry_core::geom::arrangement::tight_box(&edge);
        b=[b[0].min(r.min_x),b[1].min(r.min_y),b[2].max(r.max_x),b[3].max(r.max_y)];
    }
    Some(b)
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

/// A column's footprint fields as the core takes them: its shape, size, turn, L legs and anchor (docs/adr/0725).
fn column_spec(c: &kentos_contracts::ColumnFields) -> arch::ColumnSpec {
    arch::ColumnSpec {
        round: c.shape == kentos_contracts::ColumnShape::Round,
        width: c.width,
        depth: c.depth,
        legs: c.legs,
        rotation: c.rotation,
        anchor: c.anchor.unit(),
        ..Default::default()
    }
}

fn column_box(e: &Entity) -> Option<Box4> {
    let Entity::Column(c) = e else { return None };
    let c = &c.column;
    let r = c.width.hypot(c.depth()) / 2.;
    let spec = column_spec(c);
    let middle = arch::column_center(CoreVec2::new(c.at.x,c.at.y),&spec);
    if c.problem().is_some() || arch::column_area(CoreVec2::new(c.at.x,c.at.y),&spec).is_none()
        || ![middle.x,middle.y,r].iter().all(|n|n.is_finite()) {
        // An invalid changed source must reach every opting-in space, never
        // disappear because its numeric bounds cannot be indexed.
        return Some([f64::NEG_INFINITY,f64::NEG_INFINITY,f64::INFINITY,f64::INFINITY]);
    }
    Some([middle.x-r,middle.y-r,middle.x+r,middle.y+r])
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

pub(crate) fn takes_level(e: &Entity, level: Option<&str>) -> bool {
    level.is_none_or(|id| matches!(e, Entity::Wall(w) if w.wall.arch.level.as_deref() == Some(id)))
}

/// The explicit room scope never treats a missing level as an implicit filter.
pub fn source_level(scope: SpaceScope, target: Option<&str>, source: Option<&str>) -> bool {
    match scope {
        SpaceScope::Project => target.is_none_or(|id| source == Some(id)),
        SpaceScope::Unassigned => target.is_none() && source.is_none(),
    }
}

fn eligible_wall(e: &Entity, s: &SpaceFields) -> bool {
    match e {
        Entity::Wall(w)=>source_level(s.scope,s.arch.level.as_deref(),w.wall.arch.level.as_deref()),
        _=>separator_path(e).is_some_and(|p|source_level(s.scope,s.arch.level.as_deref(),p.space_separator.as_ref().and_then(|a|a.level.as_deref()))),
    }
}

/// A source view's key: the level, the scope, the sources and the design-option view (docs/adr/0788 §4).
type SourceKey = (Option<String>, SpaceScope, Option<Vec<String>>, String);
type SelectedSources = Result<Option<Arc<BTreeSet<Slot>>>, SpaceBreak>;

/// A partial copy cannot keep references into the original drawing. Preserve
/// its last boundary as an independent drawn room, before preview or commit.
pub fn prepare_source_copies(entities: &mut [Entity], origins: &[String]) -> usize {
    if !entities.iter().any(|e| matches!(e, Entity::Space(s) if s.space.sources.is_some())) {
        return 0;
    }
    let copied: BTreeSet<_> = origins.iter().map(String::as_str).collect();
    let mut detached = 0;
    for entity in entities {
        if let Entity::Space(s) = entity
            && s.space.sources.as_ref().is_some_and(|ids| ids.iter().any(|id| !copied.contains(id.as_str())))
        {
            s.space.sources = None;
            s.space.seed = None;
            s.space.scope = Default::default();
            s.space.columns = Default::default();
            s.space.broken = None;
            detached += 1;
        }
    }
    detached
}

type ColumnAreas = Result<Vec<(Area, Box4)>, SpaceBreak>;
type BoundarySources = Arc<Vec<(Slot, Option<Box4>)>>;

/// An immutable source view for one plan or follower pass. Borrowing the
/// document prevents wall edits from invalidating a cached boundary network.
pub struct SpaceSearch<'a> {
    doc: &'a Document,
    indexes: BTreeMap<(Vec<Slot>, bool), kentos_geometry_core::geom::space_index::BoundaryIndex>,
    columns: BTreeMap<SourceKey, ColumnAreas>,
    selected: BTreeMap<SourceKey, SelectedSources>,
    boundaries: BTreeMap<SourceKey, BoundarySources>,
    /// The walls the design-option view in force leaves out (docs/adr/0788 §4), and that view's key.
    out: Arc<BTreeSet<Slot>>,
    view_key: String,
    views: BTreeMap<String, Arc<BTreeSet<Slot>>>,
}

impl<'a> SpaceSearch<'a> {
    pub fn new(doc: &'a Document) -> Self {
        Self {
            doc,
            indexes: BTreeMap::new(),
            columns: BTreeMap::new(),
            selected: BTreeMap::new(),
            boundaries: BTreeMap::new(),
            out: Arc::new(doc.option_out()),
            view_key: String::new(),
            views: BTreeMap::new(),
        }
    }

    /// [`found`](Self::found) for the space `uid`: among the main model's walls and its own design option's
    /// (docs/adr/0788 §4), each other set's primary.
    pub fn found_as(&mut self, uid: crate::Uuid, s: &SpaceFields) -> SpaceFields {
        if self.doc.arch_options().is_empty() {
            return self.found(s);
        }
        let view = self.doc.option_view_of(uid);
        let key: String = view.iter().map(|c| format!("{}/{};", c.set, c.option)).collect();
        let doc = self.doc;
        let out = self
            .views
            .entry(key.clone())
            .or_insert_with(|| Arc::new(doc.option_hidden_slots(&view).into_iter().collect()))
            .clone();
        let (was_out, was_key) = (std::mem::replace(&mut self.out, out), std::mem::replace(&mut self.view_key, key));
        let found = self.found(s);
        self.out = was_out;
        self.view_key = was_key;
        found
    }

    /// Validate every explicit identity, including remote sources whose box
    /// does not touch this room. Missing/changed sources never mean "all".
    pub fn source_slots(&mut self, s: &SpaceFields) -> SelectedSources {
        if s.sources.is_none() && !self.doc.store.has_space_separators() {return Ok(None);}
        let key = (s.arch.level.clone(),s.scope,s.sources.clone(),self.view_key.clone());
        self.selected.entry(key).or_insert_with(|| {
            if s.scope_problem().is_some() { return Err(SpaceBreak::SourceUnavailable); }
            let Some(sources)=&s.sources else {
                for slot in self.doc.store.space_separator_slots() {
                    if let Some(e)=self.doc.get(slot) && eligible_wall(e,s) && core_separator(e).is_none() {
                        return Err(SpaceBreak::SourceUnavailable);
                    }
                }
                return Ok(None);
            };
            let mut slots = BTreeSet::new();
            for id in sources {
                let uid = crate::Uuid::parse_str(id).map_err(|_| SpaceBreak::SourceUnavailable)?;
                let slot = self.doc.slot_of(uid).ok_or(SpaceBreak::SourceUnavailable)?;
                let level = match self.doc.get(slot) {
                    Some(Entity::Wall(w)) => {
                        if w.wall.problem().is_some() {
                            return Err(SpaceBreak::InvalidBoundarySource);
                        }
                        w.wall.arch.level.as_deref()
                    }
                    Some(Entity::Column(c)) => {
                        if c.column.problem().is_some() { return Err(SpaceBreak::InvalidBoundarySource); }
                        c.column.arch.level.as_deref()
                    }
                    Some(Entity::Polyline(p)) if p.space_separator.is_some()=> {
                        if p.extras_problem(false).is_some(){return Err(SpaceBreak::SourceUnavailable);}
                        p.space_separator.as_ref().and_then(|a|a.level.as_deref())
                    }
                    _ => return Err(SpaceBreak::SourceUnavailable),
                };
                if !source_level(s.scope,s.arch.level.as_deref(),level) { return Err(SpaceBreak::SourceUnavailable); }
                slots.insert(slot);
            }
            Ok(Some(Arc::new(slots)))
        }).clone()
    }

    fn among(&mut self, slots: Vec<Slot>, seed: Vec2, how: SpaceMethod, free_room: bool) -> Option<SpacePart> {
        let index = self.indexes.entry((slots.clone(),free_room)).or_insert_with(|| {
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
            let separators:Vec<_>=slots.iter().filter_map(|&s|self.doc.get(s)).filter_map(core_separator).collect();
            if free_room { kentos_geometry_core::geom::space_index::BoundaryIndex::new_with_separators(&ins,&separators) }
            else { kentos_geometry_core::geom::space_index::BoundaryIndex::for_queries_with_separators(&ins,&separators) }
        });
        let at=CoreVec2::new(seed.x,seed.y);
        let found=if free_room { index.room_at(at,method(how)).ok().flatten() }
            else { index.found(at,method(how)) };
        found.map(|a|part(&a))
    }

    /// Keep source order identical to Document::entities, even after draw-order
    /// edits. Validate the full explicit set before pruning by a room's box.
    fn boundary_sources(&mut self, s: &SpaceFields) -> Result<BoundarySources, SpaceBreak> {
        let selected = self.source_slots(s)?;
        let key = (s.arch.level.clone(), s.scope, s.sources.clone(), self.view_key.clone());
        let out = Arc::clone(&self.out);
        Ok(Arc::clone(self.boundaries.entry(key).or_insert_with(|| Arc::new(
            self.doc.entities()
                .filter(|e| eligible_wall(e,s) && !out.contains(&Slot(e.base().id)) && selected.as_ref().is_none_or(|ids| ids.contains(&Slot(e.base().id))))
                .map(|e| (Slot(e.base().id), boundary_box(e)))
                .collect()
        ))))
    }

    /// Reports reuse this immutable search, while legacy rooms keep the exact
    /// old Document search route. An ordinary path must not change that route.
    pub fn found_preserving_legacy(&mut self, s: &SpaceFields) -> SpaceFields {
        if uses_source_search(self.doc,s) { self.found(s) } else { self.doc.space_found(s) }
    }

    /// A scope change must not reclassify its old seed as a physical wall.
    pub fn seed_is_room(&mut self, s: &SpaceFields) -> bool {
        let Some(seed)=s.seed else{return false};
        let Ok(selected) = self.source_slots(s) else {return false};
        let out = Arc::clone(&self.out);
        let slots=self.doc.entities().filter(|e|eligible_wall(e,s) && !out.contains(&Slot(e.base().id)) && selected.as_ref().is_none_or(|ids|ids.contains(&Slot(e.base().id)))).map(|e|Slot(e.base().id)).collect();
        self.among(slots,seed,s.method,true).is_some()
    }

    /// The selected physical footprints in this immutable source view. Geometry
    /// stays analytic; render fill, layer visibility and grid-break flags do not
    /// change the occupied footprint. A broken grid keeps its last real place.
    pub fn deduct_columns(&mut self, s: &SpaceFields) -> Result<Vec<SpacePart>, SpaceBreak> {
        if !s.columns.applies(s.method) { return Ok(s.parts.clone()); }
        let selected = self.source_slots(s)?;
        let level = s.arch.level.clone();
        let columns = self.columns.entry((level.clone(),s.scope,s.sources.clone(),self.view_key.clone())).or_insert_with(|| {
            self.doc.entities().filter_map(|e| match e {
                Entity::Column(c) if source_level(s.scope,level.as_deref(),c.column.arch.level.as_deref())
                    && selected.as_ref().is_none_or(|ids|ids.contains(&Slot(c.base.id))) => Some(&c.column),
                _ => None,
            }).map(|c| {
                if c.problem().is_some() { return Err(SpaceBreak::InvalidBoundarySource); }
                let spec = column_spec(c);
                let at = CoreVec2::new(c.at.x,c.at.y);
                let area = arch::column_area(at,&spec).ok_or(SpaceBreak::InvalidBoundarySource)?;
                let r = if spec.round { c.width / 2. } else { c.width.hypot(c.depth()) / 2. };
                let middle = arch::column_center(at,&spec);
                Ok((area,[middle.x-r,middle.y-r,middle.x+r,middle.y+r]))
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
        let sources = match self.boundary_sources(s) {
            Ok(sources) => sources, Err(why) => {out.broken=Some(why);return out;}
        };
        let near = if s.parts.is_empty() {
            None
        } else {
            let area = grow(grow(space_box(s), 2.0), 1.0);
            let slots = sources.iter()
                .filter(|(_,bounds)| bounds.is_some_and(|b| meets(&b, &area)))
                .map(|(slot,_)| *slot)
                .collect();
            self.among(slots, seed, s.method, !s.scope.is_project() || s.sources.is_some())
        };
        let found = near.or_else(|| {
            let slots = sources.iter().map(|(slot,_)| *slot).collect();
            self.among(slots, seed, s.method, !s.scope.is_project() || s.sources.is_some())
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

fn uses_source_search(doc: &Document, s: &SpaceFields) -> bool {
    !s.columns.is_ignore() || !s.scope.is_project() || s.sources.is_some() || doc.store.has_space_separators()
}

impl Document {
    /// The drawing's walls whose reach meets `area` (grown by a metre), as the core reads them.
    pub fn walls_near(&self, area: Box4) -> Vec<CoreWall> {
        self.space_walls_near(area, None)
    }

    pub(crate) fn space_walls_near(&self, area: Box4, level: Option<&str>) -> Vec<CoreWall> {
        let area = grow(area, 1.0);
        let out = self.option_out();
        self.entities()
            .filter(|e| takes_level(e, level) && !out.contains(&Slot(e.base().id)))
            .filter(|e| wall_box(e).is_some_and(|b| meets(&b, &area)))
            .filter_map(core_wall)
            .collect()
    }

    /// Find the seeded boundary among its explicit level's walls (all levels for legacy unassigned spaces).
    /// Search near the previous boundary first, then the whole eligible level; drawn spaces stay unchanged.
    pub fn space_found(&self, s: &SpaceFields) -> SpaceFields {
        if uses_source_search(self,s) { return SpaceSearch::new(self).found(s); }
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
            let out = self.option_out();
            let all: Vec<CoreWall> = self
                .entities()
                .filter(|e| takes_level(e, level) && !out.contains(&Slot(e.base().id)))
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
        let mut touched = BTreeSet::new();
        for op in ops {
            match op {
                Op::Add(s) | Op::Remove(s) => {
                    if is_source(&s.entity) { touched.insert(s.uid.to_string()); }
                    boxes.extend(boundary_box(&s.entity));
                    column_boxes.extend(column_box(&s.entity));
                    if matches!(*s.entity, Entity::Space(_)) && matches!(op, Op::Add(_)) {
                        own.push(s.slot());
                    }
                }
                Op::Update { before, after } => {
                    if is_source(&before.entity) || is_source(&after.entity) { touched.insert(before.uid.to_string()); }
                    boxes.extend(boundary_box(&before.entity));
                    boxes.extend(boundary_box(&after.entity));
                    column_boxes.extend(column_box(&before.entity));
                    column_boxes.extend(column_box(&after.entity));
                    if let (Entity::Space(b), Entity::Space(a)) = (&*before.entity, &*after.entity)
                        && (a.space.seed != b.space.seed
                            || a.space.method != b.space.method
                            || a.space.arch.level != b.space.arch.level
                            || a.space.parts != b.space.parts
                            || a.space.sources != b.space.sources
                            || a.space.scope != b.space.scope
                            || a.space.columns != b.space.columns)
                    {
                        own.push(after.slot());
                    }
                }
                // Objects into or out of a design option, or another primary (docs/adr/0788 §4): the spaces round
                // their walls find their boundaries anew, and the spaces moved find theirs among their new view.
                Op::ArchOptions { before, after } => {
                    for uid in crate::arch_options::touched_members(before, after) {
                        if let Some(st) = self.store.slot_of(uid).and_then(|s| self.store.get(s)) {
                            boxes.extend(boundary_box(&st.entity));
                            if matches!(*st.entity, Entity::Space(_)) {
                                own.push(st.slot());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if boxes.is_empty() && column_boxes.is_empty() && own.is_empty() && touched.is_empty() {
            return Vec::new();
        }
        let mut slots: Vec<Slot> = self
            .store
            .iter()
            .filter(|s| match &*s.entity {
                Entity::Space(x) => {
                    x.space.seed.is_some()
                        && (x.space.sources.as_ref().is_some_and(|ids| ids.iter().any(|id|touched.contains(id)))
                            || boxes.iter().any(|b| meets(b, &grow(space_box(&x.space), 0.5)))
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
            let found = search.found_as(current.uid, &s.space);
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
