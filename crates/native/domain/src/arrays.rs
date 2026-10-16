//! The drawing's associative arrays (docs/adr/0300), kept as undoable state
//! like its blocks: an array is added, changed or removed, each one undo step
//! or part of the open transaction or group. Its items are ordinary inserts;
//! the array is the record above them (`AssocArray`).
//!
//! The rules (`arrays_problem`) hold after every edit: an array whose block
//! the drawing does not define, whose items are not one per place or name an
//! insert twice is refused with nothing changed.
//!
//! What follows at a step's end (`follow_arrays`, after the step's own ops):
//! - an item's insert removed: the item is removed (`uid` none) and stays so
//!   until the array is reset (docs/adr/0300 §3);
//! - an item's insert changed by an edit that did not change its array: the
//!   item is kept (`kept`), a later layout change leaves it where it is;
//! - a path array's path changed: the items that are neither removed nor kept
//!   go to their new places along it, in the same step.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kentos_contracts::{
    ArrayLayout, AssocArray, BlockId, Entity, EntityId, InsertEntity, arrays_problem,
};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::ops::assoc_array::{Layout, Placement, item_placements};
use kentos_geometry_core::vec2::Vec2 as CoreVec2;

use crate::document::Document;
use crate::edit::Refusal;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

/// The undo step names of the arrays' own edits.
pub mod labels {
    pub const ARRAY_ADD: &str = "İlişkili dizi";
    pub const ARRAY_CHANGE: &str = "Diziyi değiştir";
    pub const ARRAY_REMOVE: &str = "Diziyi kaldır";
}

/// The geometry core's reading of an object (the contract's own JSON).
fn core(e: &Entity) -> Option<CoreEntity> {
    let json = serde_json::to_string(e).ok()?;
    CoreEntity::from_json(&Json::parse(&json).ok()?).ok()
}

/// The places of an array's items, the path's shape read from `path` (a path array's), the first the
/// array's own placement; none when the layout gives none (docs/adr/0300 §2).
pub fn placements(array: &AssocArray, path: Option<&Entity>) -> Option<Vec<Placement>> {
    let source = Placement {
        p: CoreVec2::new(array.p.x, array.p.y),
        scale: array.scale,
        rotation: array.rotation,
        mirror: array.mirror,
    };
    let shape;
    let layout = match &array.layout {
        ArrayLayout::Grid { rows, cols, dx, dy } => Layout::Grid {
            rows: *rows,
            cols: *cols,
            dx: *dx,
            dy: *dy,
        },
        ArrayLayout::Line { count, dx, dy, fit } => Layout::Line {
            count: *count,
            dx: *dx,
            dy: *dy,
            fit: *fit,
        },
        ArrayLayout::Polar {
            center,
            count,
            fill,
            rotate,
        } => Layout::Polar {
            center: CoreVec2::new(center.x, center.y),
            count: *count,
            fill: *fill,
            rotate: *rotate,
        },
        ArrayLayout::Path {
            count,
            spacing,
            align,
            ..
        } => {
            shape = core(path?)?.shape;
            Layout::Path {
                path: &shape,
                count: *count,
                spacing: *spacing,
                align: *align,
            }
        }
    };
    item_placements(source, &layout)
}

/// The insert of an array's item at `at`, its other fields `like`'s (or the array's layer and block).
pub fn item_insert(
    array: &AssocArray,
    at: &Placement,
    like: Option<&InsertEntity>,
) -> InsertEntity {
    let mut insert = like.cloned().unwrap_or_else(|| InsertEntity {
        attribute_places: Default::default(),
        base: kentos_contracts::EntityBase {
            id: 0,
            layer_id: array.layer_id.clone(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
            look: Default::default(),
        },
        block: array.block,
        p: kentos_contracts::Vec2 { x: 0.0, y: 0.0 },
        scale: 1.0,
        y_scale: None,
        rotation: 0.0,
        mirror: false,
    });
    insert.p = kentos_contracts::Vec2 {
        x: at.p.x,
        y: at.p.y,
    };
    insert.scale = at.scale;
    insert.rotation = at.rotation;
    insert.mirror = at.mirror;
    insert
}

impl Document {
    /// The associative arrays in the order they were made.
    pub fn arrays(&self) -> &[Arc<AssocArray>] {
        &self.arrays
    }

    /// The array with this id.
    pub fn array(&self, id: kentos_contracts::ArrayId) -> Option<&AssocArray> {
        self.arrays.iter().find(|a| a.id == id).map(|a| &**a)
    }

    /// The array an insert is an item of, and the item's place.
    pub fn array_of_item(&self, uid: Uuid) -> Option<(&AssocArray, usize)> {
        let id = EntityId(uid.into_bytes());
        self.arrays.iter().find_map(|a| {
            a.items
                .iter()
                .position(|i| i.uid == Some(id))
                .map(|k| (&**a, k))
        })
    }

    /// Why `arrays` cannot be the drawing's, or none: the contract's rules against its blocks, and every item's
    /// insert an object of the drawing.
    pub(crate) fn arrays_refused(&self, arrays: &[&AssocArray]) -> Option<String> {
        let blocks: Vec<BlockId> = self.blocks.iter().map(|b| b.id).collect();
        let owned: Vec<AssocArray> = arrays.iter().map(|a| (*a).clone()).collect();
        if let Some((_, why)) = arrays_problem(&owned, &blocks) {
            return Some(format!("İlişkili dizi: {why}."));
        }
        for a in arrays {
            for item in &a.items {
                if let Some(uid) = item.uid
                    && self.store.slot_of(Uuid::from_bytes(uid.0)).is_none()
                {
                    return Some("İlişkili dizi: öğenin yerleştirmesi çizimde yok.".into());
                }
            }
        }
        None
    }

    /// Adds an array after the others: one undo step “İlişkili dizi”, or part of the open transaction.
    pub fn add_array(&mut self, array: AssocArray) -> Result<(), Refusal> {
        let mut next: Vec<&AssocArray> = self.arrays.iter().map(|a| &**a).collect();
        next.push(&array);
        if let Some(why) = self.arrays_refused(&next) {
            return Err(Refusal(why));
        }
        let op = Op::ArrayAdd {
            index: self.arrays.len(),
            array: Arc::new(array),
        };
        self.record(vec![op], labels::ARRAY_ADD);
        Ok(())
    }

    /// Replaces the array with the same id, in its place: one undo step “Diziyi değiştir”. `false` when no array
    /// has the id or nothing changes; refused as `add_array` is.
    pub fn update_array(&mut self, array: AssocArray) -> Result<bool, Refusal> {
        let Some(index) = self.arrays.iter().position(|a| a.id == array.id) else {
            return Ok(false);
        };
        if *self.arrays[index] == array {
            return Ok(false);
        }
        let mut next: Vec<&AssocArray> = self.arrays.iter().map(|a| &**a).collect();
        next[index] = &array;
        if let Some(why) = self.arrays_refused(&next) {
            return Err(Refusal(why));
        }
        let op = Op::ArrayUpdate {
            before: self.arrays[index].clone(),
            after: Arc::new(array),
        };
        self.record(vec![op], labels::ARRAY_CHANGE);
        Ok(true)
    }

    /// Removes an array, its items staying as independent inserts: one undo step “Diziyi kaldır”. `false` when no
    /// array has the id.
    pub fn remove_array(&mut self, id: kentos_contracts::ArrayId) -> bool {
        let Some(index) = self.arrays.iter().position(|a| a.id == id) else {
            return false;
        };
        let op = Op::ArrayRemove {
            index,
            array: self.arrays[index].clone(),
        };
        self.record(vec![op], labels::ARRAY_REMOVE);
        true
    }

    /// Applies an array op (history.rs); the arrays are found by id.
    pub(crate) fn apply_array_op(&mut self, op: &Op) {
        match op {
            Op::ArrayAdd { index, array } => {
                let at = (*index).min(self.arrays.len());
                self.arrays.insert(at, array.clone());
            }
            Op::ArrayRemove { array, .. } => self.arrays.retain(|a| a.id != array.id),
            Op::ArrayUpdate { after, .. } => {
                if let Some(slot) = self.arrays.iter_mut().find(|a| a.id == after.id) {
                    *slot = after.clone();
                }
            }
            _ => {}
        }
    }

    /// What keeps the arrays with their items and paths after `ops` (a step about to be recorded), applied: the
    /// step takes them in (module docs).
    pub(crate) fn follow_arrays(&mut self, ops: &[Op]) -> Vec<Op> {
        if self.arrays.is_empty() {
            return Vec::new();
        }
        // The arrays the step changed itself: their items' changes are theirs.
        let own: HashSet<kentos_contracts::ArrayId> = ops
            .iter()
            .filter_map(|op| match op {
                Op::ArrayAdd { array, .. } | Op::ArrayRemove { array, .. } => Some(array.id),
                Op::ArrayUpdate { after, .. } => Some(after.id),
                _ => None,
            })
            .collect();
        let mut removed: HashSet<EntityId> = HashSet::new();
        let mut changed: HashSet<EntityId> = HashSet::new();
        for op in ops {
            match op {
                Op::Remove(s) => {
                    removed.insert(EntityId(s.uid.into_bytes()));
                }
                Op::Add(s) => {
                    // Taken back in the same step (a move as remove and add): no longer gone.
                    removed.remove(&EntityId(s.uid.into_bytes()));
                }
                Op::Update { after, .. } => {
                    changed.insert(EntityId(after.uid.into_bytes()));
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        let arrays: Vec<Arc<AssocArray>> = self.arrays.clone();
        for array in arrays {
            if own.contains(&array.id) {
                continue;
            }
            let mut next = (*array).clone();
            let mut touched = false;
            for item in &mut next.items {
                let Some(uid) = item.uid else { continue };
                if removed.contains(&uid) && self.store.slot_of(Uuid::from_bytes(uid.0)).is_none() {
                    item.uid = None;
                    item.kept = false;
                    touched = true;
                } else if changed.contains(&uid) && !item.kept {
                    item.kept = true;
                    touched = true;
                }
            }
            // A path array follows its path, its items that are neither removed nor kept to their new places.
            if let ArrayLayout::Path { path, .. } = &array.layout
                && let Some(path_uid) = EntityId::parse(path)
                && changed.contains(&path_uid)
            {
                out.extend(self.relaid(&next));
            }
            if touched {
                out.push(Op::ArrayUpdate {
                    before: array.clone(),
                    after: Arc::new(next),
                });
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }

    /// The updates that put an array's items that are neither removed nor kept at their places (a path array
    /// after its path changed); none when the layout gives no places now.
    pub(crate) fn relaid(&self, array: &AssocArray) -> Vec<Op> {
        let path = match &array.layout {
            ArrayLayout::Path { path, .. } => EntityId::parse(path)
                .and_then(|id| self.store.slot_of(Uuid::from_bytes(id.0)))
                .and_then(|slot| self.store.get(slot))
                .map(|s| (*s.entity).clone()),
            _ => None,
        };
        let Some(places) = placements(array, path.as_ref()) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (item, at) in array.items.iter().zip(&places) {
            let Some(uid) = item.uid.filter(|_| !item.kept) else {
                continue;
            };
            let Some(current) = self
                .store
                .slot_of(Uuid::from_bytes(uid.0))
                .and_then(|slot| self.store.get(slot))
                .cloned()
            else {
                continue;
            };
            let Entity::Insert(was) = &*current.entity else {
                continue;
            };
            let next = item_insert(array, at, Some(was));
            if next != *was {
                out.push(Op::Update {
                    after: Stored {
                        uid: current.uid,
                        entity: Arc::new(Entity::Insert(next)),
                    },
                    before: current,
                });
            }
        }
        out
    }

    /// The arrays' items' slots, by insert: what a reading copy or a host needs to treat an item as part of its
    /// array (selection, Öznitelikler).
    pub fn array_items(&self) -> HashMap<Slot, (kentos_contracts::ArrayId, usize)> {
        let mut out = HashMap::new();
        for a in &self.arrays {
            for (k, item) in a.items.iter().enumerate() {
                if let Some(slot) = item
                    .uid
                    .and_then(|u| self.store.slot_of(Uuid::from_bytes(u.0)))
                {
                    out.insert(slot, (a.id, k));
                }
            }
        }
        out
    }
}
