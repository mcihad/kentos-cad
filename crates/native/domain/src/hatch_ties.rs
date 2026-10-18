//! Associative hatches (docs/adr/0186 §6): a hatch made inside a closed
//! object with İlişkili on knows its objects (`assoc`: the closed object,
//! its islands, the texts and inserts left open, and the point clicked
//! inside). Before a step is recorded (`commit`), after the linked texts,
//! its changes are read:
//!
//! - a hatch one of whose objects the step changed or removed has its region
//!   cut again by the core's rule (`ops::hatch_region`), its pattern and
//!   colour kept; a removed island or cutout leaves the list;
//! - a hatch whose closed object is gone, no longer closed, or leaves no
//!   region stays where it is and follows nothing;
//! - a hatch whose ring or holes the step itself changed (a grip, the vertex
//!   table) follows nothing, unless its objects changed in the same step
//!   (then the rule writes it).
//!
//! The changes join the step: one undo takes them back with the rest. The
//! web's is model/hatchTies.ts; both run
//! `fixtures/document-ops/v1/hatch-ties.json`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use kentos_contracts::{Entity, EntityId, HatchAssoc, HatchEntity, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::block::Blocks;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, entity_vertices};
use kentos_geometry_core::geom::hatch_boundary;
use kentos_geometry_core::ops::hatch_region::{Cut, cutout, face_region, region_cut, region_of};

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::linked::font_of;
use crate::store::Stored;

/// The geometry core's reading of an object (the contract's own JSON). The JSON reads a polyline open; a closed one
/// is closed here, so a hatch inside it follows it (docs/adr/0258 §1, 0333 §2).
fn core(e: &Entity) -> Option<CoreEntity> {
    let mut value = serde_json::to_value(e).ok()?;
    // A space's part leaves out its empty holes; the core's area reads them (docs/adr/0345 §4: a finish follows its
    // space).
    if let Some(parts) = value.get_mut("parts").and_then(|p| p.as_array_mut())
        && matches!(e, Entity::Space(_))
    {
        for part in parts {
            if let Some(o) = part.as_object_mut() {
                o.entry("holes").or_insert_with(|| serde_json::Value::Array(Vec::new()));
            }
        }
    }
    let json = value.to_string();
    let mut c = CoreEntity::from_json(&Json::parse(&json).ok()?).ok()?;
    if let (Entity::Polyline(p), Shape::Polyline { closed, .. }) = (e, &mut c.shape) {
        *closed = p.closed;
    }
    Some(c)
}

/// A hatch that follows nothing: it stays as it is.
fn untied(h: &HatchEntity) -> HatchEntity {
    HatchEntity {
        assoc: None,
        ..h.clone()
    }
}

/// Whether `e` is a text linked to the object `own` (docs/adr/0175 §4).
pub(crate) fn is_own_label(e: &Entity, own: Option<Uuid>) -> bool {
    matches!((e, own), (Entity::Text(t), Some(own)) if t.label_of.is_some_and(|l| Uuid::from_bytes(l.0) == own))
}

fn point(p: &CoreVec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

impl Document {
    /// The hatch `h` cut again from its objects now, or none when the rule
    /// gives no region (its closed object gone or open, its face's line
    /// work closing nothing around the seed, nothing left). Its exact
    /// boundary is the region's (docs/adr/0328 §1).
    /// A text that is the hatch's own linked label (`own`, the hatch's id) is not left open: its place is read from the
    /// hatch's region, which leaving it open would change again (docs/adr/0332 §2).
    fn retied(
        &self,
        h: &HatchEntity,
        shifts: &HashMap<Uuid, CoreVec2>,
        own: Option<Uuid>,
    ) -> Option<HatchEntity> {
        let a = h.assoc.as_ref()?;
        let object = |id: &EntityId| {
            self.store
                .slot_of(Uuid::from_bytes(id.0))
                .and_then(|s| self.store.get(s))
                .map(|s| Arc::clone(&s.entity))
        };
        let present = |ids: &[EntityId]| -> Vec<(EntityId, Arc<Entity>)> {
            ids.iter()
                .filter_map(|id| object(id).map(|e| (*id, e)))
                .collect()
        };
        let shapes = |list: &[(EntityId, Arc<Entity>)]| -> Vec<Shape> {
            list.iter()
                .filter_map(|(_, e)| core(e).map(|c| c.shape))
                .collect()
        };
        let islands = present(&a.islands);
        let cutouts: Vec<(EntityId, Arc<Entity>)> = present(&a.cutouts)
            .into_iter()
            .filter(|(_, e)| !is_own_label(e, own))
            .collect();
        let bounds = present(&a.bounds);
        let island_shapes = shapes(&islands);
        // An insert's box needs its block's definition.
        let blocks = if cutouts
            .iter()
            .any(|(_, e)| matches!(**e, Entity::Insert(_)))
        {
            let defs: Vec<&kentos_contracts::BlockDefinition> =
                self.blocks().iter().map(|b| &**b).collect();
            serde_json::to_string(&defs)
                .ok()
                .and_then(|t| Json::parse(&t).ok())
                .and_then(|j| Blocks::from_json(&j).ok())
                .unwrap_or_default()
        } else {
            Blocks::default()
        };
        let font = font_of(self.settings().drawing_font);
        let boxes: Vec<Vec<CoreVec2>> = cutouts
            .iter()
            .map(|(_, e)| {
                core(e)
                    .and_then(|c| cutout(&c.shape, &blocks, font))
                    .unwrap_or_default()
            })
            .collect();
        let seed = CoreVec2::new(a.seed.x, a.seed.y);
        let mut moved_seed = None;
        let (area, sources, islands, cutouts) = match &a.outer {
            // Inside a closed object (docs/adr/0186 §6).
            Some(outer) => {
                let outer = core(&*object(outer)?)?;
                let (area, _) = region_cut(&outer.shape, &island_shapes, &boxes, seed)?;
                let mut sources = vec![outer.shape];
                sources.extend(island_shapes);
                // Every island and cutout still there stays named, reaching in or not.
                let islands = islands.into_iter().map(|(id, _)| id).collect();
                let cutouts = cutouts.into_iter().map(|(id, _)| id).collect();
                (area, sources, islands, cutouts)
            }
            // In the face its line work closes (docs/adr/0328 §3).
            None => {
                let bound_shapes = shapes(&bounds);
                if bound_shapes.is_empty() {
                    return None;
                }
                // The point clicked inside, or, when the line work moved away from it, the point moved as the first
                // of its moved objects did.
                let gap = a.gap.unwrap_or(0.0);
                let found = face_region(&bound_shapes, &island_shapes, &boxes, seed, gap)
                    .map(|a| (a, seed))
                    .or_else(|| {
                        let d = bounds
                            .iter()
                            .find_map(|(id, _)| shifts.get(&Uuid::from_bytes(id.0)))?;
                        let moved = CoreVec2::new(seed.x + d.x, seed.y + d.y);
                        face_region(&bound_shapes, &island_shapes, &boxes, moved, gap)
                            .map(|a| (a, moved))
                    });
                let (area, seed) = found?;
                moved_seed = Some(Vec2 {
                    x: seed.x,
                    y: seed.y,
                });
                let mut sources = bound_shapes;
                sources.extend(island_shapes);
                let islands = islands.into_iter().map(|(id, _)| id).collect();
                let cutouts = cutouts.into_iter().map(|(id, _)| id).collect();
                (area, sources, islands, cutouts)
            }
        };
        let r = region_of(
            &area,
            &Cut {
                parts: Vec::new(),
                islands: Vec::new(),
                cutouts: Vec::new(),
            },
        );
        let loops = hatch_boundary::of_area(&area, &sources);
        let boundary = hatch_boundary::curved(&loops)
            .then(|| boundary_back(&loops))
            .flatten();
        Some(HatchEntity {
            ring: r.ring.iter().map(point).collect(),
            holes: (!r.holes.is_empty()).then(|| {
                r.holes
                    .iter()
                    .map(|h| h.iter().map(point).collect())
                    .collect()
            }),
            assoc: Some(HatchAssoc {
                gap: a.gap,
                outer: a.outer,
                bounds: bounds.into_iter().map(|(id, _)| id).collect(),
                islands,
                cutouts,
                seed: moved_seed.unwrap_or(a.seed),
            }),
            boundary,
            ..h.clone()
        })
    }

    /// The tied hatch `h` cut again from its objects as they are now (docs/adr/0329 §4: a tie changed by a command);
    /// none when its objects give no region.
    pub fn recut_hatch(&self, h: &HatchEntity) -> Option<HatchEntity> {
        self.retied(h, &HashMap::new(), None)
    }

    /// What keeps the associative hatches with their objects after `ops` (a
    /// step about to be recorded), applied: the step takes them in.
    pub(crate) fn follow_hatches(&mut self, ops: &[Op]) -> Vec<Op> {
        if !self.store.has_ties() {
            return Vec::new();
        }
        // The objects the step changed or removed, by persistent id, and the
        // hatches whose ring or holes it changed itself, by slot.
        let mut objects: Vec<Uuid> = Vec::new();
        let mut seen: HashSet<Uuid> = HashSet::new();
        let mut edited: BTreeSet<Slot> = BTreeSet::new();
        let mut retie: BTreeSet<Slot> = BTreeSet::new();
        for op in ops {
            let (stored, before) = match op {
                Op::Add(s) | Op::Remove(s) => (s, None),
                Op::Update { before, after } => (after, Some(before)),
                _ => continue,
            };
            if seen.insert(stored.uid) {
                objects.push(stored.uid);
            }
            if let (Some(before), Entity::Hatch(after)) = (before, &*stored.entity)
                && let Entity::Hatch(was) = &*before.entity
                && after.assoc.is_some()
            {
                // A tie the step changed (a command's, docs/adr/0329 §4): cut again from it.
                if was.assoc != after.assoc {
                    retie.insert(stored.slot());
                } else if was.ring != after.ring || was.holes != after.holes {
                    edited.insert(stored.slot());
                }
            }
        }
        // How far each changed object moved: its first vertex's way (a face's tie finds its point again by it).
        let mut shifts: HashMap<Uuid, CoreVec2> = HashMap::new();
        for op in ops {
            if let Op::Update { before, after } = op
                && let (Some(b), Some(a)) = (core(&before.entity), core(&after.entity))
                && let (Some(p), Some(q)) = (
                    entity_vertices(&b.shape).first().copied(),
                    entity_vertices(&a.shape).first().copied(),
                )
            {
                shifts.entry(after.uid).or_insert(CoreVec2::new(0.0, 0.0));
                if let Some(d) = shifts.get_mut(&after.uid) {
                    *d = CoreVec2::new(d.x + q.x - p.x, d.y + q.y - p.y);
                }
            }
        }
        let mut out = Vec::new();
        let mut done: HashSet<Slot> = HashSet::new();
        for uid in objects {
            let tied: Vec<Slot> = self.store.tied_to(uid).collect();
            for slot in tied {
                if !done.insert(slot) {
                    continue;
                }
                let Some(current) = self.store.get(slot).cloned() else {
                    continue;
                };
                let Entity::Hatch(h) = &*current.entity else {
                    continue;
                };
                let next = self
                    .retied(h, &shifts, Some(current.uid))
                    .unwrap_or_else(|| untied(h));
                if next != *h {
                    out.push(updated(current, next));
                }
            }
        }
        for slot in retie {
            if !done.insert(slot) {
                continue;
            }
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Hatch(h) = &*current.entity else {
                continue;
            };
            let next = self
                    .retied(h, &shifts, Some(current.uid))
                    .unwrap_or_else(|| untied(h));
            if next != *h {
                out.push(updated(current, next));
            }
        }
        // A hatch edited by itself, its objects unchanged: it follows nothing now.
        for slot in edited {
            if done.contains(&slot) {
                continue;
            }
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            if let Entity::Hatch(h) = &*current.entity
                && h.assoc.is_some()
            {
                let next = untied(h);
                out.push(updated(current, next));
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}

/// The core's boundary in the contract's terms (the same JSON).
pub(crate) fn boundary_back(
    loops: &[kentos_geometry_core::entity::HatchLoop],
) -> Option<Vec<kentos_contracts::HatchLoop>> {
    use kentos_geometry_core::api::json::ToJson;
    let mut text = String::new();
    loops.to_vec().write_json(&mut text);
    serde_json::from_str(&text).ok()
}

fn updated(before: Stored, after: HatchEntity) -> Op {
    Op::Update {
        after: Stored {
            uid: before.uid,
            entity: Arc::new(Entity::Hatch(after)),
        },
        before,
    }
}
