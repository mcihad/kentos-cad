//! `cad.arrays.edit` v1 (docs/adr/0301; AutoCAD's ARRAYEDIT, Diziyi düzenle): an associative array's layout,
//! first placement or items edited by its id, as one undo step “Diziyi düzenle”. The places are the array's own
//! (`kentos_domain::arrays::placements`, the core's `item_placements`), the inserts made the way the array command
//! makes them (`item_insert`); the step changes the array itself, so the document's follow rule (docs/adr/0300 §3)
//! takes the items' changes as the array's, not as edits by hand.
//!
//! The checks, in order (the first that fails answers): the array's id; the edit's own numbers (a layout's as the
//! array command checks them, a placement's, the item lists); the expected revision; the array; the item numbers
//! against its places; the array's layer and every touched insert unlocked; a removed item named for Değiştir or
//! Sil; the replacing block; a path layout's path; the places.

use std::collections::HashSet;

use kentos_commands::{
    ArrayEdit, ArrayItem, ArrayLayout, ArraysEdit, AssocArray, CommandError, CommandResult,
    EntitiesEditPlan, EntitiesEdited, Entity, EntityId, InsertEntity,
};
use kentos_domain::arrays::{item_insert, placements};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::assoc_array::Placement;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Diziyi düzenle";

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ArraysEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: the item inserts changed (with their slots), the new ones (slot 0) and the ids
/// of those deleted; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ArraysEdit) -> CommandResult<EntitiesEditPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document read in place (a tool's preview).
pub fn plan_in(doc: &Document, input: &ArraysEdit) -> CommandResult<EntitiesEditPlan> {
    match check(doc, input) {
        Ok(planned) => CommandResult::Completed {
            output: EntitiesEditPlan {
                changed: planned.updates.iter().map(|(_, e)| e.clone()).collect(),
                created: planned.adds.iter().map(|(_, e)| e.clone()).collect(),
                removed: planned
                    .removes
                    .iter()
                    .filter_map(|s| doc.uid(*s))
                    .map(|u| u.to_string())
                    .collect(),
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the edit as one undo step “Diziyi düzenle”, into the open transaction or group
/// if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ArraysEdit) -> CommandResult<EntitiesEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ARRAYS_EDIT,
        kentos_commands::CAD_ARRAYS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ArraysEdit,
) -> CommandResult<EntitiesEdited> {
    let planned = match check(cx.doc, &input) {
        Ok(planned) => planned,
        Err(stop) => return stop.into(),
    };
    let Planned {
        mut next,
        updates,
        adds,
        removes,
    } = planned;
    let changed: Vec<String> = updates
        .iter()
        .filter_map(|(slot, _)| cx.doc.uid(*slot))
        .map(|u| u.to_string())
        .collect();
    let removed: Vec<String> = removes
        .iter()
        .filter_map(|s| cx.doc.uid(*s))
        .map(|u| u.to_string())
        .collect();
    enum Failed {
        Full(String),
        Refused(String),
    }
    let written = cx.doc.transact(LABEL, |doc| {
        for (slot, entity) in updates {
            doc.update(slot, entity);
        }
        let (places, entities): (Vec<usize>, Vec<Entity>) = adds.into_iter().unzip();
        let slots = doc
            .add_many(entities, LABEL)
            .map_err(|full| Failed::Full(full.to_string()))?;
        let mut created = Vec::with_capacity(slots.len());
        for (k, slot) in places.into_iter().zip(slots) {
            let uid = doc.uid(slot);
            if let Some(item) = next.items.get_mut(k) {
                *item = ArrayItem {
                    uid: uid.map(|u| EntityId(u.into_bytes())),
                    kept: false,
                };
            }
            created.extend(uid.map(|u| u.to_string()));
        }
        doc.remove(&removes);
        doc.update_array(next)
            .map_err(|r| Failed::Refused(r.to_string()))?;
        Ok(created)
    });
    match written {
        Ok(created) => CommandResult::Completed {
            output: EntitiesEdited {
                changed,
                created,
                removed,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(Failed::Full(message)) => CommandResult::Failed {
            error: CommandError {
                code: codes::SLOTS_EXHAUSTED.into(),
                message,
                ..Default::default()
            },
        },
        Err(Failed::Refused(message)) => CommandResult::Failed {
            error: checks::error(codes::BLOCK_REFUSED, message, Some("edit".into())),
        },
    }
}

/// What the edit writes: the array as it will be (a new place's item filled in when its insert is written), the
/// inserts changed in place, the new inserts by the item they become, and the inserts deleted.
struct Planned {
    next: AssocArray,
    updates: Vec<(Slot, Entity)>,
    adds: Vec<(usize, Entity)>,
    removes: Vec<Slot>,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

/// The array command's own checks answer at `layout.…`; here the layout is under `edit`.
fn under_edit(stop: Stop) -> Stop {
    let mut error = stop.into_error();
    error.path = error.path.map(|p| format!("edit.{p}"));
    Stop::failed(error)
}

/// An item list's own check: some numbers, each once.
fn check_list(items: &[u32]) -> Result<(), Stop> {
    let message = "Öğe numaralarını verin; her biri bir kez. Öğeler yerleşimin sırasıyla 0'dan numaralanır (ızgarada satır satır).";
    if items.is_empty() {
        return Err(refuse(codes::INVALID_ITEM, message.into(), "edit.items"));
    }
    let mut seen = HashSet::new();
    for (i, k) in items.iter().enumerate() {
        if !seen.insert(*k) {
            return Err(refuse(
                codes::INVALID_ITEM,
                message.into(),
                &format!("edit.items[{i}]"),
            ));
        }
    }
    Ok(())
}

/// The edit's own numbers, in its fields' order.
fn check_edit(edit: &ArrayEdit) -> Result<(), Stop> {
    match edit {
        ArrayEdit::Layout { layout } => crate::array::check_layout(layout).map_err(under_edit),
        ArrayEdit::Placement {
            p, scale, rotation, ..
        } => {
            if let Some(p) = p {
                checks::point(*p, "Dizinin ilk yerinin", "edit.p")?;
            }
            if let Some(s) = scale {
                checks::finite(*s, "Ölçek", "Ölçeği sonlu bir sayıyla verin.", "edit.scale")?;
                if *s <= 0.0 {
                    return Err(refuse(
                        codes::INVALID_SCALE,
                        "Ölçek sıfırdan büyük olmalı. Başka bir ölçek verin.".into(),
                        "edit.scale",
                    ));
                }
            }
            if let Some(r) = rotation {
                checks::finite(
                    *r,
                    "Dönüş açısı",
                    "Açıyı sonlu bir sayıyla verin.",
                    "edit.rotation",
                )?;
            }
            Ok(())
        }
        ArrayEdit::Replace { items, .. } | ArrayEdit::Remove { items } => check_list(items),
        ArrayEdit::Reset { items } => items.as_deref().map_or(Ok(()), check_list),
    }
}

/// An item's insert, its slot and its record: none when the item is removed or its insert is not an insert.
fn insert_of(doc: &Document, item: &ArrayItem) -> Option<(Slot, InsertEntity)> {
    let slot = doc.slot_of(Uuid::from_bytes(item.uid?.0))?;
    match doc.get(slot)? {
        Entity::Insert(insert) => Some((slot, insert.clone())),
        _ => None,
    }
}

/// The path a path layout follows, as the document has it.
fn path_of(doc: &Document, layout: &ArrayLayout) -> Option<Entity> {
    let ArrayLayout::Path { path, .. } = layout else {
        return None;
    };
    let slot = doc.slot_of(Uuid::parse_str(path).ok()?)?;
    doc.get(slot).cloned()
}

/// The places of `array` in the document: a path layout's checked as the array command checks it.
fn places_of(doc: &Document, array: &AssocArray) -> Result<Vec<Placement>, Stop> {
    crate::array::path_maps(doc, &array.layout).map_err(under_edit)?;
    placements(array, path_of(doc, &array.layout).as_ref()).ok_or_else(|| {
        refuse(
            codes::NOT_FINITE,
            "Dizinin yerleri kurulamadı: bir yer sonlu değil (sayı taşması). Daha küçük bir değer verin.".into(),
            "edit",
        )
    })
}

/// An insert at `at` made the way the array makes its items, its block the array's.
fn placed(array: &AssocArray, at: &Placement, like: Option<&InsertEntity>, slot: u32) -> Entity {
    let mut insert = item_insert(array, at, like);
    insert.block = array.block;
    insert.base.id = slot;
    Entity::Insert(insert)
}

/// The rows and columns of a grid layout.
fn grid(layout: &ArrayLayout) -> Option<(u32, u32)> {
    match *layout {
        ArrayLayout::Grid { rows, cols, .. } => Some((rows, cols)),
        _ => None,
    }
}

/// The checks in the contract's order, and what the edit writes.
fn check(doc: &Document, input: &ArraysEdit) -> Result<Planned, Stop> {
    if !checks::is_uid_text(&input.array) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{}” geçerli bir dizi kimliği değil; kimlik küçük harfli, tireli bir UUID'dir. Kimliği diziyi kuran komutun çıktısından (array) alın.",
                input.array
            ),
            "array",
        ));
    }
    check_edit(&input.edit)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let Some(array) = doc
        .arrays()
        .iter()
        .find(|a| a.id.to_text() == input.array)
        .map(|a| (**a).clone())
    else {
        return Err(refuse(
            codes::ARRAY_NOT_FOUND,
            format!(
                "“{}” kimlikli ilişkili dizi çizimde yok: kaldırılmış ya da başka bir çizimin olabilir.",
                input.array
            ),
            "array",
        ));
    };
    let count = array.items.len();
    let named: Option<&[u32]> = match &input.edit {
        ArrayEdit::Replace { items, .. } | ArrayEdit::Remove { items } => Some(items),
        ArrayEdit::Reset { items } => items.as_deref(),
        _ => None,
    };
    if let Some(items) = named {
        for (i, k) in items.iter().enumerate() {
            if *k as usize >= count {
                return Err(refuse(
                    codes::INVALID_ITEM,
                    format!(
                        "Dizinin {count} yeri var; öğe {k} yok. 0 ile {} arasında bir numara verin.",
                        count.saturating_sub(1)
                    ),
                    &format!("edit.items[{i}]"),
                ));
            }
        }
    }
    // Every insert the edit may touch, and the array's layer, unlocked: all or nothing.
    let touched: Vec<usize> = match named {
        Some(items) => items.iter().map(|k| *k as usize).collect(),
        None => (0..count).collect(),
    };
    let locked_message = "Dizinin ya da öğelerinden birinin katmanı kilitli; dizi düzenlenmedi. Katmanın kilidini Katmanlar panelinden açın.";
    if doc.layers().is_locked(&array.layer_id) {
        return Err(refuse(codes::LAYER_LOCKED, locked_message.into(), "array"));
    }
    for k in &touched {
        if let Some((_, insert)) = insert_of(doc, &array.items[*k])
            && doc.layers().is_locked(&insert.base.layer_id)
        {
            return Err(refuse(codes::LAYER_LOCKED, locked_message.into(), "array"));
        }
    }
    if let ArrayEdit::Replace { items, .. } | ArrayEdit::Remove { items } = &input.edit {
        for (i, k) in items.iter().enumerate() {
            if insert_of(doc, &array.items[*k as usize]).is_none() {
                return Err(refuse(
                    codes::ITEM_REMOVED,
                    format!("Öğe {k} elle silinmiş; önce Sıfırla ile geri getirin."),
                    &format!("edit.items[{i}]"),
                ));
            }
        }
    }
    match &input.edit {
        ArrayEdit::Layout { layout } => relayout(doc, &array, layout),
        ArrayEdit::Placement {
            p,
            scale,
            rotation,
            mirror,
        } => {
            let mut next = array.clone();
            next.p = p.unwrap_or(next.p);
            next.scale = scale.unwrap_or(next.scale);
            next.rotation = rotation.unwrap_or(next.rotation);
            next.mirror = mirror.unwrap_or(next.mirror);
            let places = places_of(doc, &next)?;
            let mut updates = Vec::new();
            for (item, at) in next.items.iter().zip(&places) {
                if item.kept {
                    continue;
                }
                if let Some((slot, was)) = insert_of(doc, item) {
                    let entity = placed(&next, at, Some(&was), slot.0);
                    if entity != Entity::Insert(was) {
                        updates.push((slot, entity));
                    }
                }
            }
            Ok(Planned {
                next,
                updates,
                adds: Vec::new(),
                removes: Vec::new(),
            })
        }
        ArrayEdit::Replace { items, block } => {
            let Some(id) = doc.block_named(block).map(|b| b.id) else {
                return Err(refuse(
                    codes::UNKNOWN_BLOCK,
                    format!(
                        "“{block}” adlı blok çizimde tanımlı değil. Var olan bir bloğun adını verin."
                    ),
                    "edit.block",
                ));
            };
            let mut next = array.clone();
            let mut updates = Vec::new();
            for k in items {
                let item = &mut next.items[*k as usize];
                if let Some((slot, mut insert)) = insert_of(doc, item) {
                    item.kept = true;
                    if insert.block != id {
                        insert.block = id;
                        insert.base.id = slot.0;
                        updates.push((slot, Entity::Insert(insert)));
                    }
                }
            }
            Ok(Planned {
                next,
                updates,
                adds: Vec::new(),
                removes: Vec::new(),
            })
        }
        ArrayEdit::Remove { items } => {
            let mut next = array.clone();
            let mut removes = Vec::new();
            for k in items {
                let item = &mut next.items[*k as usize];
                if let Some((slot, _)) = insert_of(doc, item) {
                    removes.push(slot);
                }
                *item = ArrayItem::default();
            }
            Ok(Planned {
                next,
                updates: Vec::new(),
                adds: Vec::new(),
                removes,
            })
        }
        ArrayEdit::Reset { .. } => {
            let places = places_of(doc, &array)?;
            let like = template(doc, &array);
            let mut next = array.clone();
            let mut updates = Vec::new();
            let mut adds = Vec::new();
            for k in touched {
                let at = &places[k];
                match insert_of(doc, &array.items[k]) {
                    Some((slot, was)) => {
                        next.items[k].kept = false;
                        let entity = placed(&array, at, Some(&was), slot.0);
                        if entity != Entity::Insert(was) {
                            updates.push((slot, entity));
                        }
                    }
                    None => adds.push((k, placed(&array, at, like.as_ref(), 0))),
                }
            }
            Ok(Planned {
                next,
                updates,
                adds,
                removes: Vec::new(),
            })
        }
    }
}

/// The insert a new item copies its layer, look and data from: the first item that is neither removed nor kept,
/// else the first that is there; none when every item is removed (the array's layer then).
fn template(doc: &Document, array: &AssocArray) -> Option<InsertEntity> {
    let mut first = None;
    for item in &array.items {
        if let Some((_, insert)) = insert_of(doc, item) {
            if !item.kept {
                return Some(insert);
            }
            first.get_or_insert(insert);
        }
    }
    first
}

/// A new layout: an item keeps its number (a grid's its row and column when both layouts are grids); the places
/// that are new get inserts, the items whose places are gone take their inserts away.
fn relayout(doc: &Document, array: &AssocArray, layout: &ArrayLayout) -> Result<Planned, Stop> {
    let mut next = array.clone();
    next.layout = layout.clone();
    let places = places_of(doc, &next)?;
    let n = places.len();
    let from = |j: usize| -> Option<usize> {
        match (grid(&array.layout), grid(layout)) {
            (Some((rows, cols)), Some((_, new_cols))) => {
                let (r, c) = (j / new_cols as usize, j % new_cols as usize);
                (r < rows as usize && c < cols as usize).then(|| r * cols as usize + c)
            }
            _ => (j < array.items.len()).then_some(j),
        }
    };
    let like = template(doc, array);
    let mut kept_from = vec![false; array.items.len()];
    let mut items = Vec::with_capacity(n);
    let mut updates = Vec::new();
    let mut adds = Vec::new();
    for (j, at) in places.iter().enumerate() {
        match from(j) {
            Some(k) => {
                kept_from[k] = true;
                let item = array.items[k].clone();
                if !item.kept
                    && let Some((slot, was)) = insert_of(doc, &item)
                {
                    let entity = placed(&next, at, Some(&was), slot.0);
                    if entity != Entity::Insert(was) {
                        updates.push((slot, entity));
                    }
                }
                items.push(item);
            }
            None => {
                adds.push((j, placed(&next, at, like.as_ref(), 0)));
                items.push(ArrayItem::default());
            }
        }
    }
    let removes = array
        .items
        .iter()
        .zip(&kept_from)
        .filter(|(_, kept)| !**kept)
        .filter_map(|(item, _)| insert_of(doc, item).map(|(slot, _)| slot))
        .collect();
    next.items = items;
    Ok(Planned {
        next,
        updates,
        adds,
        removes,
    })
}
