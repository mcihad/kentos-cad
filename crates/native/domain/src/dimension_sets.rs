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

fn row(r: DimensionSetRow) -> core::Row {
    match r {
        DimensionSetRow::Openings => core::Row::Openings,
        DimensionSetRow::Overall => core::Row::Overall,
    }
}
fn contract_row(r: core::Row) -> DimensionSetRow {
    match r {
        core::Row::Openings => DimensionSetRow::Openings,
        core::Row::Overall => DimensionSetRow::Overall,
    }
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

    /// Row offset is relative to the selected physical wall face, not a centre-reference member.
    pub fn dimension_set_row_offset(
        &self,
        set: &DimensionSet,
        which: DimensionSetRow,
    ) -> Result<f64, DimensionSetBreak> {
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
        let mut out = Vec::with_capacity(spans.len());
        // A span measuring the same two places as one drawn before it, in the rows' order, is not drawn again: an
        // edge without openings has its openings row its whole length, the overall dimension once (docs/adr/0695's
        // 27 October addendum). What the user removed is not drawn, so the other row's stays.
        let mut drawn: Vec<(SubRef, SubRef)> = Vec::new();
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
            let old = old_by_key.get(&key).copied();
            let like = old
                .and_then(|m| self.slot_of(Uuid::from_bytes(m.uid.0)))
                .and_then(|slot| self.get(slot))
                .and_then(|e| {
                    if let Entity::Dimension(d) = e {
                        Some(d)
                    } else {
                        None
                    }
                });
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
        for op in ops {
            if let Op::Update { before, after } = op
                && let (Entity::Wall(b), Entity::Wall(a)) = (&*before.entity, &*after.entity)
            {
                walls
                    .entry(EntityId(after.uid.into_bytes()))
                    .and_modify(|(_, last)| *last = a.wall.clone())
                    .or_insert_with(|| (b.wall.clone(), a.wall.clone()));
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
            let source_changed = changed.contains(&before.wall) || hosts.contains(&before.wall);
            let member_changed = before.members.iter().any(|m| changed.contains(&m.uid));
            if !source_changed && !member_changed && !own.contains(&before.id) {
                continue;
            }
            let mut next = (*before).clone();
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
            // The measured edge goes with its two ends when the wall gains or loses a vertex; gone, the set breaks
            // rather than measure another edge.
            let edge = match walls.get(&next.wall) {
                Some((b, a)) => follow_edge(b, a, next.edge),
                None => Some(next.edge),
            };
            let planned = match edge {
                Some(edge) => {
                    next.edge = edge;
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
                || changed.contains(&set.wall)
                || hosts.contains(&set.wall)
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
