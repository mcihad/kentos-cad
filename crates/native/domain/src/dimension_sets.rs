//! Persistent source-driven architectural dimension rows. The recipe owns source
//! membership and row placement; its children remain ordinary selectable dimensions.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kentos_contracts::{
    DimensionEntity, DimensionRefs, DimensionSet, DimensionSetBreak, DimensionSetFace,
    DimensionSetId, DimensionSetMember, DimensionSetOpenings, DimensionSetRow,
    DimensionSetRowPlace, DimensionSetSegment, DimensionStyle, Entity, EntityBase, EntityId,
    SubRef, Vec2, WallFields, dimension_sets_problem,
};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::ops::dimension_set as core;
use kentos_geometry_core::ops::subref::{Place, point};
use kentos_geometry_core::vec2::Vec2 as CoreVec2;

use crate::document::{Document, ObjectPolicy};
use crate::edit::{Refusal, SlotsExhausted};
use crate::history::Op;
use crate::identity::{Slot, Uuid, new_uid};
use crate::store::Stored;

pub mod labels {
    pub const ADD: &str = "Otomatik duvar ölçüleri";
    pub const CHANGE: &str = "Otomatik ölçüleri değiştir";
    pub const REMOVE: &str = "Ölçü grubunu bağımsızlaştır";
}

const OWNED_EDIT: &str = "Bu ölçü otomatik gruba aittir. Uçları ve sıra yerini grup ayarından değiştirin; üye silme ile grup düzenlemeyi ayrı adımlarda uygulayın veya önce grubu bağımsızlaştırın.";

/// A source interval as it would be drawn, before any identity is allocated.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedDimensionSetMember {
    pub segment: DimensionSetSegment,
    pub uid: Option<EntityId>,
    pub dimension: DimensionEntity,
}

/// The face edge a recipe measures: its two ends, the face line's number and the ends' vertex numbers, and how
/// far from the face an opening of this edge may lie (the wall's width).
#[derive(Clone, Copy, Debug)]
struct FaceEdge {
    line: [CoreVec2; 2],
    part: usize,
    ends: [usize; 2],
    reach: f64,
}

/// A wall's edges as the dimension tool and command need them (docs/adr/0700).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WallEdges {
    /// The straight edges' numbers, in order.
    pub straight: Vec<u32>,
    /// All its edges.
    pub count: usize,
    pub closed: bool,
    /// The path turns counter-clockwise: a closed wall's left face is its inside.
    pub counter_clockwise: bool,
}

fn p(v: Vec2) -> CoreVec2 {
    CoreVec2::new(v.x, v.y)
}
fn v(p: CoreVec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}
/// The edge's number on the wall's new path (docs/adr/0700): the same number while the path keeps its vertex count
/// and closedness; else the edge whose two ends are, bit for bit, the old edge's. None: the edge is gone. An open
/// two-point wall's only edge is kept as no number, as an old recipe has it.
fn follow_edge(before: &WallFields, after: &WallFields, edge: Option<u32>) -> Option<Option<u32>> {
    let (p0, p1) = (&before.pts, &after.pts);
    if p0.len() == p1.len() && before.closed == after.closed {
        return Some(edge);
    }
    let (n0, n1) = (p0.len(), p1.len());
    let k = edge.unwrap_or(0) as usize;
    if n0 < 2 || n1 < 2 || (!before.closed && k + 1 >= n0) || k >= n0 {
        return None;
    }
    let (a, b) = (p0[k], p0[(k + 1) % n0]);
    let same = |u: Vec2, w: Vec2| u.x.to_bits() == w.x.to_bits() && u.y.to_bits() == w.y.to_bits();
    let edges = if after.closed { n1 } else { n1 - 1 };
    let j = (0..edges).find(|&j| same(p1[j], a) && same(p1[(j + 1) % n1], b))?;
    Some((after.closed || n1 > 2).then_some(j as u32))
}

use crate::dimension_spans::{contract_row, core_row as row};

/// An interval's identity apart from the places naming it (docs/adr/0729 §4): its row, and its two objects with
/// where they are, to a nanometre, in either order.
type Interval = (&'static str, [(EntityId, i64, i64); 2]);

fn interval(row: DimensionSetRow, a: (EntityId, Vec2), b: (EntityId, Vec2)) -> Interval {
    let at = |(id, p): (EntityId, Vec2)| (id, (p.x * 1e9).round() as i64, (p.y * 1e9).round() as i64);
    let (a, b) = (at(a), at(b));
    (row.name(), if a <= b { [a, b] } else { [b, a] })
}
fn fault(f: core::Fault) -> DimensionSetBreak {
    match f {
        core::Fault::UnsupportedWall => DimensionSetBreak::UnsupportedWall,
        core::Fault::OpeningUnplaced => DimensionSetBreak::OpeningUnplaced,
    }
}

/// Fields owned by a recipe. Text, text placement, breaks and appearance stay editable.
pub(crate) fn same_owned_geometry(a: &DimensionEntity, b: &DimensionEntity) -> bool {
    a.a == b.a
        && a.b == b.b
        && a.offset == b.offset
        && a.style == b.style
        && a.angle == b.angle
        && a.c == b.c
        && a.za == b.za
        && a.zb == b.zb
        && a.refs == b.refs
}

fn base(layer: &str) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.into(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
        look: Default::default(),
    }
}

/// Whether `q` lies on a wall's straight path within a micrometre.
fn on_path(w: &WallFields, q: Vec2) -> bool {
    let n = w.pts.len();
    let edges = if w.closed { n } else { n.saturating_sub(1) };
    (0..edges).any(|k| {
        let bulge = w.bulges.as_ref().and_then(|b| b.get(k)).copied().unwrap_or(0.0);
        let (a, b) = (w.pts[k], w.pts[(k + 1) % n]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l2 = dx * dx + dy * dy;
        if bulge != 0.0 || l2 <= 0.0 {
            return false;
        }
        let t = ((q.x - a.x) * dx + (q.y - a.y) * dy) / l2;
        let (px, py) = (a.x + dx * t, a.y + dy * t);
        (-1e-9..=1.0 + 1e-9).contains(&t) && (q.x - px).hypot(q.y - py) <= 1e-6
    })
}

/// A wall's path turned the other way round, bit for bit (`cad.walls.edit`'s Çevir).
fn is_turned(before: &WallFields, after: &WallFields) -> bool {
    let same = |u: Vec2, w: Vec2| u.x.to_bits() == w.x.to_bits() && u.y.to_bits() == w.y.to_bits();
    before.closed == after.closed
        && before.pts.len() == after.pts.len()
        && before.pts.len() >= 2
        && before.pts.iter().rev().zip(&after.pts).all(|(u, w)| same(*u, *w))
}

/// A turned wall's edge: the same two ends' (an open path's `n − 2 − k`, a closed one's `(2n − 2 − k) mod n`).
fn turned_edge(before: &WallFields, _after: &WallFields, edge: Option<u32>) -> Option<u32> {
    let n = before.pts.len();
    edge.map(|k| {
        let k = k as usize;
        (if before.closed { (2 * n - 2 - k) % n } else { n.saturating_sub(2 + k) }) as u32
    })
}

/// A façade or wall recipe's sources after the step (docs/adr/0729 §4): a split's piece joins them, a join's wall takes
/// its absorbed one's place, a deleted one leaves; one wall's recipe split or joined becomes a façade's along its
/// measured edge's former direction. A turned single wall's face is the other: returns whether it turned.
fn follow_sources(
    set: &mut DimensionSet,
    walls: &HashMap<EntityId, (WallFields, WallFields)>,
    made: &[(EntityId, WallFields)],
    gone: &[(EntityId, WallFields)],
) -> bool {
    if set.span.as_ref().is_some_and(|s| s.space) {
        return false;
    }
    let mut turned = false;
    if set.span.is_none()
        && let Some((b, a)) = walls.get(&set.wall)
        && is_turned(b, a)
    {
        set.face = match set.face {
            DimensionSetFace::Left => DimensionSetFace::Right,
            DimensionSetFace::Right => DimensionSetFace::Left,
        };
        turned = true;
    }
    let before = set.sources();
    let mut sources = before.clone();
    // A piece the step made on a source's former path.
    for (id, piece) in made {
        let split = before.iter().any(|s| {
            walls
                .get(s)
                .is_some_and(|(old, _)| piece.pts.iter().all(|q| on_path(old, *q)))
        });
        if split && !sources.contains(id) {
            sources.push(*id);
        }
    }
    // A source the step removed: a wall not yet a source now running over its former path takes its place, else it
    // leaves.
    for (id, old) in gone {
        let Some(at) = sources.iter().position(|s| s == id) else {
            continue;
        };
        let mut takers: Vec<&EntityId> = walls
            .iter()
            .filter(|(w, (_, now))| !sources.contains(w) && old.pts.iter().all(|q| on_path(now, *q)))
            .map(|(w, _)| w)
            .collect();
        takers.sort();
        match takers.first() {
            Some(w) => sources[at] = **w,
            None => {
                sources.remove(at);
            }
        }
    }
    let mut seen = HashSet::new();
    sources.retain(|s| seen.insert(*s));
    if sources == before || sources.is_empty() {
        return turned;
    }
    if set.span.is_none() {
        // The measured wall's fields before the step: an edited one's, or a removed one's.
        let former = walls
            .get(&set.wall)
            .map(|(b, _)| b)
            .or_else(|| gone.iter().find(|(g, _)| *g == set.wall).map(|(_, w)| w));
        let Some(former) = former else {
            return turned;
        };
        let n = former.pts.len();
        let k = set.edge.unwrap_or(0) as usize;
        if n < 2 || k >= n {
            return turned;
        }
        let (a, b) = (former.pts[k], former.pts[(k + 1) % n]);
        let l = (b.x - a.x).hypot(b.y - a.y);
        if l.is_nan() || l <= 0.0 {
            return turned;
        }
        set.span = Some(kentos_contracts::DimensionSetSpan {
            walls: Vec::new(),
            space: false,
            direction: Vec2 {
                x: (b.x - a.x) / l,
                y: (b.y - a.y) / l,
            },
            reference: kentos_contracts::DimensionSetReference::Faces,
            level: former.arch.level.clone(),
        });
        set.edge = None;
        set.face = DimensionSetFace::Left;
    }
    set.wall = sources[0];
    if let Some(span) = &mut set.span {
        span.walls = sources[1..].to_vec();
    }
    turned
}

/// Arc identities distinguish our exact child writes from a caller's visually identical operation.
fn owned_writes(ops: &[Op]) -> Vec<(Uuid, Arc<Entity>, Arc<Entity>)> {
    ops.iter()
        .filter_map(|op| match op {
            Op::Update { before, after } => {
                Some((after.uid, before.entity.clone(), after.entity.clone()))
            }
            _ => None,
        })
        .collect()
}

impl Document {
    pub fn dimension_sets(&self) -> &[Arc<DimensionSet>] {
        &self.dimension_sets
    }

    pub fn dimension_set(&self, id: DimensionSetId) -> Option<&DimensionSet> {
        self.dimension_sets
            .iter()
            .find(|s| s.id == id)
            .map(|s| &**s)
    }

    pub fn dimension_set_of_item(&self, uid: Uuid) -> Option<(&DimensionSet, usize)> {
        let id = EntityId(uid.into_bytes());
        self.dimension_sets.iter().find_map(|s| {
            s.members
                .iter()
                .position(|m| m.uid == id)
                .map(|i| (&**s, i))
        })
    }

    /// Ordinary member deletion and an explicit recipe edit cannot share a pending step.
    /// Private recipe reconciliation removes children directly, so this guard does not reject it.
    pub(crate) fn dimension_set_pending_members(&self) -> HashSet<Uuid> {
        let editing: HashSet<_> = self
            .history
            .pending
            .iter()
            .flat_map(|s| &s.ops)
            .filter_map(|op| match op {
                Op::DimensionSetAdd { set, .. } => Some(set.id),
                Op::DimensionSetUpdate { after, .. } => Some(after.id),
                _ => None,
            })
            .collect();
        if editing.is_empty() {
            return HashSet::new();
        }
        self.dimension_sets
            .iter()
            .filter(|s| editing.contains(&s.id))
            .flat_map(|s| s.members.iter().map(|m| Uuid::from_bytes(m.uid.0)))
            .collect()
    }

    /// Stored children must exist, but a deleted/unsupported host is a retained broken recipe.
    pub(crate) fn dimension_sets_refused(&self, sets: &[&DimensionSet]) -> Option<String> {
        let list: Vec<DimensionSet> = sets.iter().map(|s| (*s).clone()).collect();
        if let Some((_, why)) = dimension_sets_problem(&list) {
            return Some(format!("Otomatik ölçü: {why}"));
        }
        for set in sets {
            if self.layers.get(&set.layer_id).is_none() {
                return Some("Otomatik ölçünün hedef katmanı çizimde yok.".into());
            }
            for member in &set.members {
                let e = self
                    .slot_of(Uuid::from_bytes(member.uid.0))
                    .and_then(|slot| self.get(slot));
                let Some(Entity::Dimension(d)) = e else {
                    return Some(
                        "Otomatik ölçünün kayıtlı üyesi çizimde bir ölçü değil veya eksik.".into(),
                    );
                };
                let clean = |r: &Option<SubRef>| {
                    r.as_ref().map(|r| {
                        let mut r = r.clone();
                        r.broken = None;
                        r
                    })
                };
                if !d.refs.as_ref().is_some_and(|refs| {
                    clean(&refs.a) == Some(member.a.clone())
                        && clean(&refs.b) == Some(member.b.clone())
                        && refs.c.is_none()
                        && refs.rule.is_none()
                }) {
                    return Some(
                        "Otomatik ölçü üyesinin kaynak başvuruları kayıtlı segmentle uyuşmuyor."
                            .into(),
                    );
                }
            }
        }
        None
    }

    /// Loading/sync never drops a recipe with a missing or unsupported source. Freeze its cached
    /// geometry and mark its references, so the drawing cannot show a healthy-looking stale chain.
    pub(crate) fn normalize_dimension_set_sources(&mut self) {
        for before in self.dimension_sets.clone() {
            let Err(why) = self.dimension_set_plan(&before) else {
                continue;
            };
            let mut after = (*before).clone();
            after.broken = Some(why);
            let op = Op::DimensionSetUpdate {
                before,
                after: Arc::new(after.clone()),
                writes: Vec::new(),
            };
            self.apply_dimension_set_op(&op);
            for member in &after.members {
                let Some(stored) = self
                    .slot_of(Uuid::from_bytes(member.uid.0))
                    .and_then(|slot| self.store.get(slot))
                    .cloned()
                else {
                    continue;
                };
                let Entity::Dimension(d) = &*stored.entity else {
                    continue;
                };
                let mut d = d.clone();
                if let Some(refs) = &mut d.refs {
                    for r in [&mut refs.a, &mut refs.b, &mut refs.c]
                        .into_iter()
                        .flatten()
                    {
                        r.broken = Some(
                            if self.slot_of(Uuid::from_bytes(r.uid.0)).is_none() {
                                "object_gone"
                            } else {
                                "no_such_place"
                            }
                            .into(),
                        );
                    }
                }
                let next = Stored {
                    uid: stored.uid,
                    entity: Arc::new(Entity::Dimension(d)),
                };
                if next.entity != stored.entity {
                    self.apply_op(&Op::Update {
                        before: stored,
                        after: next,
                    });
                }
            }
        }
        self.refresh_dimension_set_rows();
    }

    /// Row lines are derived from the current plot scale, just like a view mark's cached text.
    /// Settings stay outside undo; refresh after history/rollback as well so undoing an object
    /// never restores a row gap computed at an obsolete scale. Manual rowPlaces are world points.
    pub(crate) fn refresh_dimension_set_rows(&mut self) -> usize {
        self.refresh_dimension_set_rows_inner(true)
    }

    pub(crate) fn refresh_dimension_set_rows_quietly(&mut self) -> usize {
        self.refresh_dimension_set_rows_inner(false)
    }

    fn refresh_dimension_set_rows_inner(&mut self, report: bool) -> usize {
        let mut count = 0;
        for set in self.dimension_sets.clone() {
            let Ok(plan) = self.dimension_set_plan(&set) else {
                continue;
            };
            for planned in plan {
                let Some(uid) = planned.uid else {
                    continue;
                };
                let Some(before) = self
                    .slot_of(Uuid::from_bytes(uid.0))
                    .and_then(|slot| self.store.get(slot))
                    .cloned()
                else {
                    continue;
                };
                let Entity::Dimension(d) = &*before.entity else {
                    continue;
                };
                if d.offset == planned.dimension.offset {
                    continue;
                }
                let mut d = d.clone();
                d.offset = planned.dimension.offset;
                let after = Stored {
                    uid: before.uid,
                    entity: Arc::new(Entity::Dimension(d)),
                };
                if report {
                    self.note_dimension_row_cache(before.clone(), after.clone());
                }
                let op = Op::Update { before, after };
                self.apply_op(&op);
                count += 1;
            }
        }
        count
    }

    fn dimension_set_policy(&self) -> Result<(), Refusal> {
        if self.object_policy != ObjectPolicy::Free {
            return Err(Refusal(
                "Veri girişi rolü otomatik ölçü grubunu veya yerini değiştiremez.".into(),
            ));
        }
        Ok(())
    }

    /// The real chosen face, from stable wall subplaces, never render tessellation.
    fn dimension_set_face(&self, set: &DimensionSet) -> Result<FaceEdge, DimensionSetBreak> {
        self.dimension_set_face_of(set.wall, set.face, set.edge)
    }

    /// The face edge a recipe on `wall` measures (docs/adr/0700): `edge` of a wall with more than one edge, the only
    /// edge of a two-point wall when none.
    pub fn dimension_set_wall_face(
        &self,
        wall: Uuid,
        face: DimensionSetFace,
        edge: Option<u32>,
    ) -> Result<[Vec2; 2], DimensionSetBreak> {
        self.dimension_set_face_of(EntityId(wall.into_bytes()), face, edge)
            .map(|f| f.line.map(v))
    }

    /// A wall's straight edges and whether its path is closed and counter-clockwise (its left face inside); none
    /// for another object or a wall the core cannot draw.
    pub fn dimension_set_wall_edges(&self, wall: Uuid) -> Option<WallEdges> {
        let e = self.slot_of(wall).and_then(|slot| self.get(slot))?;
        let Entity::Wall(w) = e else {
            return None;
        };
        let n = w.wall.pts.len();
        let bulges = w.wall.bulges.clone().unwrap_or_else(|| vec![0.0; n]);
        let area: f64 = (0..n)
            .map(|i| {
                let (a, b) = (w.wall.pts[i], w.wall.pts[(i + 1) % n]);
                a.x * b.y - b.x * a.y
            })
            .sum();
        Some(WallEdges {
            straight: core::straight_edges(n, &bulges, w.wall.closed)
                .into_iter()
                .map(|k| k as u32)
                .collect(),
            count: if w.wall.closed { n } else { n.saturating_sub(1) },
            closed: w.wall.closed,
            counter_clockwise: area > 0.0,
        })
    }

    fn dimension_set_face_of(
        &self,
        wall_id: EntityId,
        face: DimensionSetFace,
        edge: Option<u32>,
    ) -> Result<FaceEdge, DimensionSetBreak> {
        let e = self
            .slot_of(Uuid::from_bytes(wall_id.0))
            .and_then(|slot| self.get(slot))
            .ok_or(DimensionSetBreak::HostGone)?;
        let Entity::Wall(w) = e else {
            return Err(DimensionSetBreak::UnsupportedWall);
        };
        let n = w.wall.pts.len();
        let edge = match edge {
            Some(k) => k as usize,
            None if !w.wall.closed && n == 2 => 0,
            None => return Err(DimensionSetBreak::UnsupportedWall),
        };
        let text = serde_json::to_string(e).map_err(|_| DimensionSetBreak::UnsupportedWall)?;
        let shape = CoreEntity::from_json(
            &Json::parse(&text).map_err(|_| DimensionSetBreak::UnsupportedWall)?,
        )
        .map_err(|_| DimensionSetBreak::UnsupportedWall)?;
        let kentos_geometry_core::entity::Shape::Wall { pts, bulges, wall } = &shape.shape else {
            return Err(DimensionSetBreak::UnsupportedWall);
        };
        let lines = kentos_geometry_core::geom::wall::lines(pts, bulges.as_deref(), wall)
            .map_err(|_| DimensionSetBreak::UnsupportedWall)?;
        let part = match face {
            DimensionSetFace::Left => 0,
            DimensionSetFace::Right => lines
                .len()
                .checked_sub(1)
                .ok_or(DimensionSetBreak::UnsupportedWall)?,
        };
        let line = lines.get(part).ok_or(DimensionSetBreak::UnsupportedWall)?;
        // A bevelled or cleaned corner leaves no face vertex per path vertex: its places are not the path's.
        if line.pts.len() != n {
            return Err(DimensionSetBreak::UnsupportedWall);
        }
        let path_bulges = bulges.clone().unwrap_or_else(|| vec![0.0; n]);
        let (_, ends) = core::edge_face(&line.pts, &path_bulges, wall.closed, edge)
            .ok_or(DimensionSetBreak::UnsupportedWall)?;
        // The places as every other reader of a wall's places finds them (ADR0694's subrefs).
        let at = |index| {
            point(
                &shape,
                Place::Vertex {
                    part,
                    ring: 0,
                    index,
                },
            )
            .ok_or(DimensionSetBreak::UnsupportedWall)
        };
        Ok(FaceEdge {
            line: [at(ends[0])?, at(ends[1])?],
            part,
            ends,
            reach: wall.width,
        })
    }

    /// Row offset is relative to the selected physical wall face, not a centre-reference member; a façade's or a
    /// space's from its bodies' outermost face (docs/adr/0729 §6).
    pub fn dimension_set_row_offset(
        &self,
        set: &DimensionSet,
        which: DimensionSetRow,
    ) -> Result<f64, DimensionSetBreak> {
        if set.span.is_some() {
            return self.dimension_span_row_offset(set, which);
        }
        let line = self.dimension_set_face(set)?.line;
        let frame = core::Frame::new(line).map_err(fault)?;
        let at = self.dimension_set_row_at(set, which, frame)?;
        Ok(frame.offset(at))
    }

    fn dimension_set_row_at(
        &self,
        set: &DimensionSet,
        which: DimensionSetRow,
        frame: core::Frame,
    ) -> Result<CoreVec2, DimensionSetBreak> {
        let index = set
            .rows
            .iter()
            .position(|r| *r == which)
            .ok_or(DimensionSetBreak::UnsupportedWall)?;
        if let Some(place) = set.row_places.iter().find(|r| r.row == which) {
            return Ok(p(place.at));
        }
        let at = p(set.at);
        let side = if frame.offset(at) < 0.0 { -1.0 } else { 1.0 };
        Ok(frame.at_offset(
            at,
            frame.offset(at) + side * set.gap_mm * self.settings.plot_scale / 1000.0 * index as f64,
        ))
    }

    pub fn set_dimension_set_row_offset(
        &self,
        set: &mut DimensionSet,
        which: DimensionSetRow,
        offset: f64,
    ) -> Result<(), DimensionSetBreak> {
        if !offset.is_finite() {
            return Err(DimensionSetBreak::UnsupportedWall);
        }
        if set.span.is_some() {
            return self.set_dimension_span_row_offset(set, which, offset);
        }
        let line = self.dimension_set_face(set)?.line;
        let frame = core::Frame::new(line).map_err(fault)?;
        let at = v(frame.at_offset(self.dimension_set_row_at(set, which, frame)?, offset));
        match set.row_places.iter_mut().find(|r| r.row == which) {
            Some(place) => place.at = at,
            None => set.row_places.push(DimensionSetRowPlace { row: which, at }),
        }
        Ok(())
    }

    /// Pure with respect to the document: stable source keys and no slot/UUID allocation.
    pub fn dimension_set_plan(
        &self,
        set: &DimensionSet,
    ) -> Result<Vec<PlannedDimensionSetMember>, DimensionSetBreak> {
        if set.span.is_some() {
            let plan = self.dimension_span_spans(set).map_err(|r| r.broken)?;
            return self.dimension_set_members(set, plan.spans, &plan.sources);
        }
        let (spans, sources) = self.dimension_edge_spans(set)?;
        self.dimension_set_members(set, spans, &sources)
    }

    /// A façade or space recipe's plan with its rows' account (docs/adr/0729 §5), or why it cannot be planned: what a
    /// command plans and refuses with.
    pub fn dimension_span_plan(
        &self,
        set: &DimensionSet,
    ) -> Result<(Vec<PlannedDimensionSetMember>, crate::dimension_spans::SpanNotes), crate::dimension_spans::SpanRefusal>
    {
        let plan = self.dimension_span_spans(set)?;
        let members = self
            .dimension_set_members(set, plan.spans, &plan.sources)
            .map_err(|broken| crate::dimension_spans::SpanRefusal {
                broken,
                source: None,
                why: None,
            })?;
        Ok((members, plan.notes))
    }

    /// One wall edge's spans (docs/adr/0695, 0700) and the identities their source numbers name.
    fn dimension_edge_spans(
        &self,
        set: &DimensionSet,
    ) -> Result<(Vec<core::Span>, Vec<EntityId>), DimensionSetBreak> {
        let FaceEdge {
            line,
            part,
            ends,
            reach,
        } = self.dimension_set_face(set)?;
        let mut sources = vec![set.wall];
        let mut openings = Vec::new();
        if set.rows.contains(&DimensionSetRow::Openings) {
            for (slot, opening) in self.openings_of(&set.wall.to_string()) {
                // A flue reaches no face; a niche shows on its own face only (docs/adr/0719 §4).
                let seen = match opening.form {
                    kentos_contracts::OpeningForm::Flue => false,
                    kentos_contracts::OpeningForm::Niche => {
                        (opening.side == kentos_contracts::OpeningSide::Right)
                            == (set.face == DimensionSetFace::Right)
                    }
                    _ => true,
                };
                if !seen {
                    continue;
                }
                if opening.broken.is_some() || opening.place.len() != 4 {
                    return Err(DimensionSetBreak::OpeningUnplaced);
                }
                let jambs = [
                    p(opening.place[0]),
                    p(opening.place[1]),
                    p(opening.place[2]),
                    p(opening.place[3]),
                ];
                // Another edge's opening belongs to that edge's chain (docs/adr/0700).
                if !core::on_edge(line, &jambs, reach) {
                    continue;
                }
                let uid = self.uid(slot).ok_or(DimensionSetBreak::OpeningUnplaced)?;
                sources.push(EntityId(uid.into_bytes()));
                openings.push(core::Opening {
                    source: sources.len() - 1,
                    jambs,
                });
            }
        }
        let spans = core::plan(
            line,
            part,
            ends,
            match set.face {
                DimensionSetFace::Left => core::Face::Left,
                DimensionSetFace::Right => core::Face::Right,
            },
            &openings,
            match set.openings {
                DimensionSetOpenings::Ends => core::Openings::Ends,
                DimensionSetOpenings::Centers => core::Openings::Centers,
            },
            &set.rows.iter().copied().map(row).collect::<Vec<_>>(),
            p(set.at),
            set.gap_mm * self.settings.plot_scale / 1000.0,
            &set.row_places
                .iter()
                .map(|r| (row(r.row), p(r.at)))
                .collect::<Vec<_>>(),
        )
        .map_err(fault)?;
        Ok((spans, sources))
    }

    /// The members a recipe's spans make: an unchanged source pair keeps its dimension's identity, as does the same
    /// interval named by other places of the same objects (a turned wall's faces; docs/adr/0729 §4).
    fn dimension_set_members(
        &self,
        set: &DimensionSet,
        spans: Vec<core::Span>,
        sources: &[EntityId],
    ) -> Result<Vec<PlannedDimensionSetMember>, DimensionSetBreak> {
        let mut look = set.look.clone();
        // No named style is the project's Standard dimension style, exactly as style_of(None).
        // Current style heights apply to new intervals; existing member overrides stay below.
        let mut height = if set.look.dim_style.is_none() {
            self.settings
                .annotation_height(kentos_contracts::AnnotationKind::Dimension)
        } else {
            set.height
        };
        if let Some(name) = &set.look.dim_style
            && let Some(style) = self.settings.dimension_styles.iter().find(|s| {
                s.id == *name || s.name.trim().to_lowercase() == name.trim().to_lowercase()
            })
        {
            look = style.look();
            height = style.height_at(self.settings.plot_scale);
        }
        let reference = |node: core::Point| SubRef {
            uid: sources[node.source],
            place: crate::dimension_ties::contract_place(node.place),
            path: Vec::new(),
            broken: None,
        };
        let old_by_key: HashMap<String, _> = set
            .members
            .iter()
            .map(|m| serde_json::to_string(&m.segment()).map(|key| (key, m)))
            .collect::<Result<_, _>>()
            .map_err(|_| DimensionSetBreak::UnsupportedWall)?;
        let suppressed: HashSet<String> = set
            .suppressed
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<_, _>>()
            .map_err(|_| DimensionSetBreak::UnsupportedWall)?;
        let before = self.dimension_set(set.id);
        let appearance_changed =
            before.is_some_and(|s| s.look != set.look || s.height != set.height);
        let layer_changed = before.is_some_and(|s| s.layer_id != set.layer_id);
        // A span measuring the same two places as one drawn before it, in the rows' order, is not drawn again: an
        // edge without openings has its openings row its whole length, the overall dimension once (docs/adr/0695's
        // 27 October addendum). What the user removed is not drawn, so the other row's stays.
        let mut drawn: Vec<(SubRef, SubRef)> = Vec::new();
        let mut kept: Vec<(core::Span, DimensionSetSegment, String)> = Vec::with_capacity(spans.len());
        for span in spans {
            let segment = DimensionSetSegment {
                row: contract_row(span.row),
                a: reference(span.a),
                b: reference(span.b),
            };
            let key =
                serde_json::to_string(&segment).map_err(|_| DimensionSetBreak::UnsupportedWall)?;
            if suppressed.contains(&key)
                || drawn
                    .iter()
                    .any(|(a, b)| *a == segment.a && *b == segment.b)
            {
                continue;
            }
            drawn.push((segment.a.clone(), segment.b.clone()));
            kept.push((span, segment, key));
        }
        // The members' identities: the same source pair first, then the same interval by other places.
        let stored = |m: &DimensionSetMember| match self
            .slot_of(Uuid::from_bytes(m.uid.0))
            .and_then(|slot| self.get(slot))
        {
            Some(Entity::Dimension(d)) => Some(d),
            _ => None,
        };
        let mut claimed: HashSet<EntityId> = kept
            .iter()
            .filter_map(|(_, _, key)| old_by_key.get(key))
            .filter(|m| stored(m).is_some())
            .map(|m| m.uid)
            .collect();
        let mut by_interval: HashMap<Interval, &DimensionSetMember> = HashMap::new();
        for m in &set.members {
            if claimed.contains(&m.uid) {
                continue;
            }
            if let Some(d) = stored(m) {
                by_interval
                    .entry(interval(m.row, (m.a.uid, d.a), (m.b.uid, d.b)))
                    .or_insert(m);
            }
        }
        let mut out = Vec::with_capacity(kept.len());
        for (span, segment, key) in kept {
            let old = old_by_key
                .get(&key)
                .copied()
                .filter(|m| stored(m).is_some())
                .or_else(|| {
                    let found = by_interval.get(&interval(
                        segment.row,
                        (segment.a.uid, v(span.a.at)),
                        (segment.b.uid, v(span.b.at)),
                    ))?;
                    claimed.insert(found.uid).then_some(*found)
                });
            let like = old.and_then(stored);
            let mut d = like.cloned().unwrap_or_else(|| DimensionEntity {
                base: base(&set.layer_id),
                a: v(span.a.at),
                b: v(span.b.at),
                offset: span.offset,
                height,
                text: None,
                style: Some(DimensionStyle::Linear),
                angle: Some(span.angle),
                c: None,
                mask: false,
                za: None,
                zb: None,
                refs: None,
                text_shift: None,
                breaks: Vec::new(),
                look: look.clone(),
                annotative: None,
                level: None,
            });
            if appearance_changed && like.is_some() {
                d.look = set.look.clone();
                d.height = set.height;
            }
            if layer_changed {
                d.base.layer_id = set.layer_id.clone();
            }
            d.a = v(span.a.at);
            d.b = v(span.b.at);
            d.offset = span.offset;
            d.style = Some(DimensionStyle::Linear);
            d.angle = Some(span.angle);
            d.c = None;
            d.za = None;
            d.zb = None;
            d.refs = Some(Box::new(DimensionRefs {
                a: Some(segment.a.clone()),
                b: Some(segment.b.clone()),
                c: None,
                rule: None,
            }));
            out.push(PlannedDimensionSetMember {
                segment,
                uid: old.filter(|_| like.is_some()).map(|m| m.uid),
                dimension: d,
            });
        }
        Ok(out)
    }

    /// Child ops are prepared completely before any is applied; capacity refusal cannot leave half a row.
    fn dimension_set_ops(
        &mut self,
        set: &mut DimensionSet,
        plan: Vec<PlannedDimensionSetMember>,
    ) -> Result<Vec<Op>, Refusal> {
        if plan.len() + set.suppressed.len() > kentos_contracts::DIMENSION_SET_MEMBERS_MAX {
            return Err(Refusal(
                "Otomatik ölçü grubunun segment sınırı aşıldı.".into(),
            ));
        }
        let count = plan.iter().filter(|m| m.uid.is_none()).count();
        if self.next_slot + count as u64 > u64::from(u32::MAX) + 1 {
            return Err(Refusal(SlotsExhausted.to_string()));
        }
        let mut ops = Vec::new();
        let mut next = Vec::with_capacity(plan.len());
        let keep: HashSet<EntityId> = plan.iter().filter_map(|m| m.uid).collect();
        for old in &set.members {
            if !keep.contains(&old.uid)
                && let Some(stored) = self
                    .slot_of(Uuid::from_bytes(old.uid.0))
                    .and_then(|slot| self.store.get(slot))
            {
                ops.push(Op::Remove(stored.clone()));
            }
        }
        for mut planned in plan {
            let uid = match planned.uid {
                Some(uid) => {
                    let before = self
                        .slot_of(Uuid::from_bytes(uid.0))
                        .and_then(|slot| self.store.get(slot))
                        .cloned()
                        .ok_or_else(|| Refusal("Otomatik ölçü üyesi çizimde yok.".into()))?;
                    planned.dimension.base.id = before.slot().0;
                    let after = Stored {
                        uid: before.uid,
                        entity: Arc::new(Entity::Dimension(planned.dimension)),
                    };
                    if before.entity != after.entity {
                        ops.push(Op::Update { before, after });
                    }
                    uid
                }
                None => {
                    let slot = Slot(self.next_slot as u32);
                    self.next_slot += 1;
                    let uid = new_uid();
                    planned.dimension.base.id = slot.0;
                    ops.push(Op::Add(Stored {
                        uid,
                        entity: Arc::new(Entity::Dimension(planned.dimension)),
                    }));
                    EntityId(uid.into_bytes())
                }
            };
            next.push(DimensionSetMember {
                uid,
                row: planned.segment.row,
                a: planned.segment.a,
                b: planned.segment.b,
            });
        }
        set.members = next;
        set.broken = None;
        Ok(ops)
    }

    pub fn add_dimension_set(&mut self, mut set: DimensionSet) -> Result<(), Refusal> {
        self.dimension_set_policy()?;
        let mut list: Vec<DimensionSet> =
            self.dimension_sets.iter().map(|s| (**s).clone()).collect();
        list.push(set.clone());
        if let Some((_, why)) = dimension_sets_problem(&list) {
            return Err(Refusal(why));
        }
        if !set.members.is_empty() {
            return Err(Refusal(
                "Yeni ölçü grubunun üyeleri kaynaklardan üretilir.".into(),
            ));
        }
        let plan = self
            .dimension_set_plan(&set)
            .map_err(|b| Refusal(format!("Ölçü kaynağı hazır değil: {}.", b.name())))?;
        let mut ops = self.dimension_set_ops(&mut set, plan)?;
        ops.push(Op::DimensionSetAdd {
            index: self.dimension_sets.len(),
            set: Arc::new(set),
        });
        self.record(ops, labels::ADD);
        Ok(())
    }

    pub fn update_dimension_set(&mut self, mut set: DimensionSet) -> Result<bool, Refusal> {
        self.dimension_set_policy()?;
        let Some(index) = self.dimension_sets.iter().position(|s| s.id == set.id) else {
            return Ok(false);
        };
        let before = self.dimension_sets[index].clone();
        if before
            .members
            .iter()
            .any(|m| self.slot_of(Uuid::from_bytes(m.uid.0)).is_none())
        {
            return Err(Refusal("Bu işlemde silinen otomatik ölçü üyesi var. Üye silme ile aynı grubu düzenlemeyi ayrı adımlarda uygulayın; silinen aralık ancak açık geri getirme işlemiyle yeniden üretilir.".into()));
        }
        // Callers edit the recipe; persistent membership is computed here, never supplied by a script.
        set.members = before.members.clone();
        let mut list: Vec<DimensionSet> =
            self.dimension_sets.iter().map(|s| (**s).clone()).collect();
        let mut recipe = set.clone();
        recipe.members.clear(); // disabled rows are removed by reconciliation below
        list[index] = recipe;
        if let Some((_, why)) = dimension_sets_problem(&list) {
            return Err(Refusal(why));
        }
        let plan = self
            .dimension_set_plan(&set)
            .map_err(|b| Refusal(format!("Ölçü kaynağı hazır değil: {}.", b.name())))?;
        let mut ops = self.dimension_set_ops(&mut set, plan)?;
        if *before == set && ops.is_empty() {
            return Ok(false);
        }
        ops.push(Op::DimensionSetUpdate {
            before,
            after: Arc::new(set),
            writes: owned_writes(&ops),
        });
        self.record(ops, labels::CHANGE);
        Ok(true)
    }

    /// Detaches the complete recipe, leaving its last children as ordinary editable dimensions.
    pub fn remove_dimension_set(&mut self, id: DimensionSetId) -> Result<bool, Refusal> {
        self.dimension_set_policy()?;
        let Some(index) = self.dimension_sets.iter().position(|s| s.id == id) else {
            return Ok(false);
        };
        self.record(
            vec![Op::DimensionSetRemove {
                index,
                set: self.dimension_sets[index].clone(),
            }],
            labels::REMOVE,
        );
        Ok(true)
    }

    pub(crate) fn apply_dimension_set_op(&mut self, op: &Op) {
        match op {
            Op::DimensionSetAdd { index, set } => self
                .dimension_sets
                .insert((*index).min(self.dimension_sets.len()), set.clone()),
            Op::DimensionSetRemove { set, .. } => self.dimension_sets.retain(|s| s.id != set.id),
            Op::DimensionSetUpdate { after, .. } => {
                if let Some(now) = self.dimension_sets.iter_mut().find(|s| s.id == after.id) {
                    *now = after.clone();
                }
            }
            _ => {}
        }
    }

    /// Full source and membership reconciliation after wall/opening followers have settled.
    pub(crate) fn follow_dimension_sets(&mut self, ops: &[Op]) -> Vec<Op> {
        if self.dimension_sets.is_empty() {
            return Vec::new();
        }
        let own: HashSet<DimensionSetId> = ops
            .iter()
            .filter_map(|op| match op {
                Op::DimensionSetAdd { set, .. } | Op::DimensionSetRemove { set, .. } => {
                    Some(set.id)
                }
                Op::DimensionSetUpdate { after, .. } => Some(after.id),
                _ => None,
            })
            .collect();
        let mut changed = HashSet::new();
        let mut removed = HashSet::new();
        let mut hosts = HashSet::new();
        let mut illegal = HashSet::new();
        let mut violations = HashSet::new();
        let trusted: HashSet<_> = ops
            .iter()
            .filter_map(|op| {
                if let Op::DimensionSetUpdate { writes, .. } = op {
                    Some(writes)
                } else {
                    None
                }
            })
            .flatten()
            .map(|(uid, b, a)| (*uid, Arc::as_ptr(b), Arc::as_ptr(a)))
            .collect();
        // Each changed wall's path before the step's first write and after its last (docs/adr/0700).
        let mut walls: HashMap<EntityId, (WallFields, WallFields)> = HashMap::new();
        // The walls the step made and removed (a split's piece, a join's absorbed wall; docs/adr/0729 §4), and the
        // boxes of what it changed that a façade or space recipe reads.
        let mut made: Vec<(EntityId, WallFields)> = Vec::new();
        let mut gone: Vec<(EntityId, WallFields)> = Vec::new();
        let mut boxes: Vec<crate::dimension_spans::Box4> = Vec::new();
        for op in ops {
            match op {
                Op::Update { before, after } => {
                    if let (Entity::Wall(b), Entity::Wall(a)) = (&*before.entity, &*after.entity) {
                        walls
                            .entry(EntityId(after.uid.into_bytes()))
                            .and_modify(|(_, last)| *last = a.wall.clone())
                            .or_insert_with(|| (b.wall.clone(), a.wall.clone()));
                    }
                    boxes.extend(crate::dimension_spans::reach_box(&before.entity));
                    boxes.extend(crate::dimension_spans::reach_box(&after.entity));
                }
                Op::Add(s) => {
                    if let Entity::Wall(w) = &*s.entity {
                        made.push((EntityId(s.uid.into_bytes()), w.wall.clone()));
                    }
                    boxes.extend(crate::dimension_spans::reach_box(&s.entity));
                }
                Op::Remove(s) => {
                    if let Entity::Wall(w) = &*s.entity {
                        gone.push((EntityId(s.uid.into_bytes()), w.wall.clone()));
                    }
                    boxes.extend(crate::dimension_spans::reach_box(&s.entity));
                }
                _ => {}
            }
        }
        let owners: HashMap<Uuid, DimensionSetId> = self
            .dimension_sets
            .iter()
            .flat_map(|s| s.members.iter().map(|m| (Uuid::from_bytes(m.uid.0), s.id)))
            .collect();
        for op in ops {
            let mut source = |s: &Stored| {
                changed.insert(EntityId(s.uid.into_bytes()));
                if let Entity::Opening(o) = &*s.entity
                    && let Some(id) = EntityId::parse(&o.opening.host)
                {
                    hosts.insert(id);
                }
            };
            match op {
                Op::Add(s) => {
                    source(s);
                    removed.remove(&EntityId(s.uid.into_bytes()));
                }
                Op::Remove(s) => {
                    source(s);
                    removed.insert(EntityId(s.uid.into_bytes()));
                }
                Op::Update { before, after } => {
                    source(before);
                    source(after);
                    if let Some(set_id) = owners.get(&after.uid) {
                        let legal = matches!((&*before.entity, &*after.entity), (Entity::Dimension(a),Entity::Dimension(b)) if same_owned_geometry(a,b));
                        if !legal
                            && !trusted.contains(&(
                                after.uid,
                                Arc::as_ptr(&before.entity),
                                Arc::as_ptr(&after.entity),
                            ))
                        {
                            illegal.insert(*set_id);
                            violations.insert(after.uid);
                        }
                    }
                }
                Op::DimensionSetViolation { uid } => {
                    if let Some(set_id) = owners.get(uid) {
                        illegal.insert(*set_id);
                    }
                }
                _ => {}
            }
        }
        let sets = self.dimension_sets.clone();
        let mut out: Vec<Op> = violations
            .into_iter()
            .map(|uid| Op::DimensionSetViolation { uid })
            .collect();
        for (index, before) in sets.into_iter().enumerate() {
            if illegal.contains(&before.id) {
                continue;
            } // Explicit fault, never silently undo a user's forbidden edit.
            let source_changed = before
                .sources()
                .iter()
                .any(|s| changed.contains(s) || hosts.contains(s))
                || (before.span.is_some()
                    && self.dimension_span_reach(&before).is_some_and(|r| {
                        boxes.iter().any(|b| crate::dimension_spans::meets(b, &r))
                    }));
            let member_changed = before.members.iter().any(|m| changed.contains(&m.uid));
            if !source_changed && !member_changed && !own.contains(&before.id) {
                continue;
            }
            let mut next = (*before).clone();
            // Its sources after a split, a join, a deletion; one wall's recipe becomes a façade's (docs/adr/0729 §4).
            let turned = follow_sources(&mut next, &walls, &made, &gone);
            if !own.contains(&before.id) {
                let deleted: Vec<_> = next
                    .members
                    .iter()
                    .filter(|m| {
                        removed.contains(&m.uid)
                            && self.slot_of(Uuid::from_bytes(m.uid.0)).is_none()
                    })
                    .cloned()
                    .collect();
                if !deleted.is_empty() && deleted.len() == next.members.len() {
                    let op = Op::DimensionSetRemove {
                        index,
                        set: before.clone(),
                    };
                    self.apply_op(&op);
                    out.push(op);
                    continue;
                }
                for m in deleted {
                    next.suppressed.push(m.segment());
                    next.members.retain(|member| member.uid != m.uid);
                }
            }
            // The measured edge goes with its two ends when the wall gains or loses a vertex, and the face with a
            // turned wall's; gone, the set breaks rather than measure another edge.
            let edge = match walls.get(&next.wall) {
                _ if next.span.is_some() => Some(None),
                Some((b, a)) if turned => Some(turned_edge(b, a, next.edge)),
                Some((b, a)) => follow_edge(b, a, next.edge),
                None => Some(next.edge),
            };
            let planned = match edge {
                Some(edge) => {
                    if next.span.is_none() {
                        next.edge = edge;
                    }
                    self.dimension_set_plan(&next)
                }
                None => Err(DimensionSetBreak::UnsupportedWall),
            };
            let mut derived = match planned {
                Ok(plan) => match self.dimension_set_ops(&mut next, plan) {
                    Ok(ops) => ops,
                    Err(_) => continue, // The seal's completeness check returns an atomic capacity refusal.
                },
                Err(why) => {
                    next.broken = Some(why);
                    let mut updates = Vec::new();
                    for member in &next.members {
                        let Some(stored) = self
                            .slot_of(Uuid::from_bytes(member.uid.0))
                            .and_then(|slot| self.store.get(slot))
                            .cloned()
                        else {
                            continue;
                        };
                        let Entity::Dimension(d) = &*stored.entity else {
                            continue;
                        };
                        let mut d = d.clone();
                        if let Some(refs) = &mut d.refs {
                            for r in [&mut refs.a, &mut refs.b, &mut refs.c]
                                .into_iter()
                                .flatten()
                            {
                                r.broken = Some(
                                    if self.slot_of(Uuid::from_bytes(r.uid.0)).is_none() {
                                        "object_gone"
                                    } else {
                                        "no_such_place"
                                    }
                                    .into(),
                                );
                            }
                        }
                        let after = Stored {
                            uid: stored.uid,
                            entity: Arc::new(Entity::Dimension(d)),
                        };
                        if after.entity != stored.entity {
                            updates.push(Op::Update {
                                before: stored,
                                after,
                            });
                        }
                    }
                    updates
                }
            };
            if *before != next || !derived.is_empty() {
                // Even a geometry-only refresh records its ownership, so generic edits remain distinguishable.
                derived.push(Op::DimensionSetUpdate {
                    before,
                    after: Arc::new(next),
                    writes: owned_writes(&derived),
                });
            }
            for op in &derived {
                self.apply_op(op);
            }
            out.extend(derived);
        }
        out
    }

    /// Ownership and completeness are checked by the outer command seal, after all followers.
    pub(crate) fn dimension_set_faults(
        &self,
        report: &crate::step::StepReport,
    ) -> Vec<crate::integrity::Fault> {
        use crate::integrity::Fault;
        let authorized = |id| {
            report.dimension_sets.added.contains(&id) || report.dimension_sets.changed.contains(&id)
        };
        let mut out = Vec::new();
        let cached: HashSet<Uuid> = report.dimension_set_cached.iter().copied().collect();
        for uid in &report.dimension_set_violations {
            out.push(Fault {
                uid: *uid,
                message: OWNED_EDIT.into(),
                code: Some("dimension_set_owned"),
            });
        }
        let owners: HashMap<Uuid, &DimensionSet> = self
            .dimension_sets
            .iter()
            .flat_map(|s| s.members.iter().map(|m| (Uuid::from_bytes(m.uid.0), &**s)))
            .collect();
        for change in &report.objects {
            let Some(set) = owners.get(&change.uid) else {
                continue;
            };
            if let (Some(before), Some(after)) = (change.before.as_deref(), change.after.as_deref())
            {
                let legal = matches!((before,after), (Entity::Dimension(a),Entity::Dimension(b)) if same_owned_geometry(a,b));
                if !legal && !authorized(set.id) && !cached.contains(&change.uid) {
                    out.push(Fault {
                        uid: change.uid,
                        message: OWNED_EDIT.into(),
                        code: Some("dimension_set_owned"),
                    });
                }
            }
        }
        // A capacity shortfall leaves the old members untouched. It must refuse the source edit,
        // rather than record a source that appears to have a complete automatic chain.
        let changed: HashSet<EntityId> = report
            .objects
            .iter()
            .map(|c| EntityId(c.uid.into_bytes()))
            .collect();
        let hosts: HashSet<EntityId> = report
            .objects
            .iter()
            .flat_map(|c| {
                [c.before.as_deref(), c.after.as_deref()]
                    .into_iter()
                    .flatten()
            })
            .filter_map(|e| {
                if let Entity::Opening(o) = e {
                    EntityId::parse(&o.opening.host)
                } else {
                    None
                }
            })
            .collect();
        for set in &self.dimension_sets {
            let touched = authorized(set.id)
                || set.sources().iter().any(|s| changed.contains(s) || hosts.contains(s))
                || set.members.iter().any(|m| changed.contains(&m.uid));
            // A broken set keeps its last members on purpose (docs/adr/0700): its edge may now name another edge
            // the plan would measure, which is no capacity shortfall.
            if !touched || set.broken.is_some() {
                continue;
            }
            if let Ok(plan) = self.dimension_set_plan(set) {
                let complete =
                    plan.len() == set.members.len() && plan.iter().all(|p| p.uid.is_some_and(|uid|
                        self.slot_of(Uuid::from_bytes(uid.0)).and_then(|s|self.get(s)).is_some_and(|e|
                            matches!(e,Entity::Dimension(d) if same_owned_geometry(d,&p.dimension)))));
                if !complete {
                    out.push(Fault { uid: Uuid::from_bytes(set.wall.0), message: "Otomatik ölçü sırası eksiksiz üretilemedi; kaynak değişikliği uygulanmadı. Çizimdeki nesne kapasitesini kontrol edin.".into(), code: Some("dimension_set_incomplete") });
                }
            }
        }
        if self.object_policy == ObjectPolicy::AttributesOnly
            && (!report.dimension_sets.added.is_empty()
                || !report.dimension_sets.changed.is_empty()
                || !report.dimension_sets.removed.is_empty())
        {
            out.push(Fault {
                uid: Uuid::nil(),
                message: "Veri girişi rolü otomatik ölçü grubunu değiştiremez.".into(),
                code: None,
            });
        }
        out
    }
}
