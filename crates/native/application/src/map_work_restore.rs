//! `map.work.restore` v1 (plan/map-cad.md MAP-01, docs/adr/4080): Onaylı hâline getir. An approved item's objects put
//! back as its approval kept them (their values, docs/adr/3680), in one undo step: an approved object no longer in the
//! drawing made again with its persistent id (as ArcGIS's restoreRows brings rows back from a common ancestor by their
//! object ids), one changed or moved off the item's layers set back to its value. Objects added since the approval
//! stay. All or none: every object put back lands on a layer that takes it (there, a layer, unlocked, of its kind), and
//! a changed one is not on a locked layer now. The shared cases are `fixtures/commands/v1/map.work.restore.json`.

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityId, LayerNodeType, MapWorkRestore, MapWorkRestored, WorkItem, WorkStatus,
    uuid_text,
};
use kentos_domain::{Document, Slot, Uuid};
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::map_work::{content_objects, map_project};

/// The undo step's name.
const STEP: &str = "Onaylı hâline getir";

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// How an approved object stands now.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Now {
    /// Not in the drawing: made again.
    Removed,
    /// In the drawing, not as approved (changed, or moved off the item's layers): set back, at its slot.
    Changed(Slot),
    /// As approved.
    Same,
}

/// An approved object to put back: its id, its value as approved and (changed) its slot now.
struct Back {
    uid: Uuid,
    value: Entity,
    slot: Option<Slot>,
}

struct Checked {
    item: String,
    back: Vec<Back>,
    added: u32,
    approved: Vec<(EntityId, String)>,
    warnings: Vec<CommandWarning>,
}

/// The approved object's value as an object (its slot id any: the write gives it one).
fn entity_of(text: &str) -> Option<Entity> {
    approved_entity(&serde_json::from_str(text).ok()?)
}

/// An approved object's value (its snapshot form without the slot id, docs/adr/3680) as an object, its slot id 0; none
/// when it does not read as one. Harita işi draws a changed geometry's approved shape with it (docs/adr/4480).
pub fn approved_entity(value: &Value) -> Option<Entity> {
    let mut v = value.clone();
    v.as_object_mut()?.insert("id".into(), Value::from(0));
    serde_json::from_value(v).ok()
}

/// Where the item's approval stops a restore: not approved, or no values kept (docs/adr/3680 says why there are none).
fn approval_of(it: &WorkItem) -> Result<(), Stop> {
    if it.status != WorkStatus::Approved {
        return Err(refuse(
            codes::WORK_NOT_APPROVED,
            format!("“{}” onaylı değil; onaylı hâline getirilecek bir onayı yok.", it.name),
            "item",
        ));
    }
    let name = &it.name;
    let why = if it.file.is_some() {
        Some(format!(
            "“{name}” dosyalı bir kaynak: onayı nesnelerini değil dosyasının revizyonunu tutar; onaylı hâline getirilemez."
        ))
    } else if it.approved_objects.is_empty() {
        Some(format!(
            "“{name}” öğesinin onayı nesne başına özet tutmuyor (şema 210'dan önce ya da 100 000'den çok nesne); onaylı hâline getirilemez, öğeyi yeniden onaylayın."
        ))
    } else if it.approved_values.len() != it.approved_objects.len() {
        Some(format!(
            "“{name}” öğesinin onayı nesnelerin değerlerini tutmuyor (şema 213'ten önce ya da birlikte 4 MiB'tan büyük); onaylı hâline getirilemez, öğeyi yeniden onaylayın."
        ))
    } else {
        None
    };
    match why {
        Some(message) => Err(refuse(codes::NO_APPROVED_VALUES, message, "item")),
        None => Ok(()),
    }
}

fn check(doc: &Document, input: &MapWorkRestore) -> Result<Checked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    map_project(doc)?;
    let empty = kentos_commands::MapWork::default();
    let work = doc.map_work().unwrap_or(&empty);
    let Some(it) = work.items.iter().find(|i| i.id == input.item.trim()) else {
        return Err(refuse(codes::UNKNOWN_WORK_ITEM, format!("Harita işinde “{}” kimlikli öğe yok.", input.item.trim()), "item"));
    };
    approval_of(it)?;
    // Each approved object now: not in the drawing, changed (its digest now another), or as approved.
    let now: BTreeMap<EntityId, String> = content_objects(doc, work, &it.id).into_iter().collect();
    let state = |uid: &EntityId, digest: &str| -> Now {
        match doc.slot_of(Uuid::from_bytes(uid.0)) {
            None => Now::Removed,
            Some(slot) => {
                let same = doc.get(slot).is_some_and(|e| crate::sheets_sources::object_digest(e) == digest);
                if same { Now::Same } else { Now::Changed(slot) }
            }
        }
    };
    let approved: BTreeMap<EntityId, (usize, &String)> =
        it.approved_objects.iter().enumerate().map(|(k, (u, d))| (*u, (k, d))).collect();
    let added = now.keys().filter(|u| !approved.contains_key(*u)).count();
    // The objects asked: those named, or every one removed or changed.
    let mut asked: Vec<(EntityId, usize, Now, String)> = Vec::new();
    if input.uids.is_empty() {
        for (u, (k, d)) in &approved {
            let s = state(u, d);
            if s != Now::Same {
                asked.push((*u, *k, s, "item".into()));
            }
        }
        if asked.is_empty() {
            return Err(refuse(
                codes::NOTHING_TO_RESTORE,
                format!("“{}” onaylandığından beri silinen ya da değişen nesnesi yok; onaylı hâline getirilecek bir şey yok.", it.name),
                "item",
            ));
        }
    } else {
        let mut seen = HashSet::new();
        for (i, text) in input.uids.iter().enumerate() {
            let path = format!("uids[{i}]");
            let Some(uid) = Uuid::parse_str(text.trim()).ok().filter(|_| checks::is_uid_text(text.trim())) else {
                return Err(refuse(
                    codes::INVALID_UID,
                    format!(
                        "“{text}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
                    ),
                    &path,
                ));
            };
            let u = EntityId(*uid.as_bytes());
            let Some((k, d)) = approved.get(&u) else {
                return Err(refuse(
                    codes::NOT_APPROVED_OBJECT,
                    format!("“{}” “{}” öğesinin onaylı nesnelerinden biri değil.", text.trim(), it.name),
                    &path,
                ));
            };
            let s = state(&u, d);
            if s == Now::Same {
                return Err(refuse(
                    codes::NOTHING_TO_RESTORE,
                    format!("“{}” onaylandığı gibi; onaylı hâline getirilecek bir şey yok.", text.trim()),
                    &path,
                ));
            }
            if seen.insert(u) {
                asked.push((u, *k, s, path));
            }
        }
        asked.sort_by_key(|a| a.0);
    }
    // Each one's value, the layer it goes back to and (changed) the layer it is on now.
    let layers = doc.layers();
    let mut back = Vec::new();
    let mut warnings = Vec::new();
    for (u, k, s, path) in asked {
        let uid = Uuid::from_bytes(u.0);
        let Some(value) = entity_of(&it.approved_values[k]) else {
            return Err(refuse(
                codes::NO_APPROVED_VALUES,
                format!("“{}” nesnesinin onaylanan değeri okunamadı; öğeyi yeniden onaylayın.", uuid_text(&u.0)),
                &path,
            ));
        };
        let layer = value.base().layer_id.clone();
        if let Some(node) = layers.get(&layer)
            && node.kind == LayerNodeType::Layer
            && let Some(holds) = node.geometry
            && !holds.admits(value.kind())
        {
            return Err(refuse(
                codes::LAYER_GEOMETRY_MISMATCH,
                format!(
                    "“{}” bir {}; {} bu katmana yazılmaz. Katmanın türünü Karışık yapın.",
                    node.name,
                    holds.layer_words(),
                    kentos_commands::kind_words(value.kind())
                ),
                &path,
            ));
        }
        for w in checks::layer_at(doc, &layer, &path)? {
            if !warnings.iter().any(|x: &CommandWarning| x.message == w.message) {
                warnings.push(w);
            }
        }
        let slot = match s {
            Now::Changed(slot) => {
                if let Some(e) = doc.get(slot)
                    && layers.is_locked(&e.base().layer_id)
                {
                    let name = layers.get(&e.base().layer_id).map_or_else(|| e.base().layer_id.clone(), |n| n.name.clone());
                    return Err(refuse(
                        codes::LAYER_LOCKED,
                        format!("“{name}” katmanı kilitli; üstündeki nesne onaylı hâline getirilemez. Kilidi Katmanlar panelinden açın."),
                        &path,
                    ));
                }
                Some(slot)
            }
            _ => None,
        };
        back.push(Back { uid, value, slot });
    }
    Ok(Checked {
        item: it.id.clone(),
        back,
        added: u32::try_from(added).unwrap_or(u32::MAX),
        approved: it.approved_objects.clone(),
        warnings,
    })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<MapWorkRestored> {
    cx.doc.name_step(STEP);
    let (mut restored, mut reverted) = (Vec::new(), Vec::new());
    let (mut made, mut set) = (Vec::new(), Vec::new());
    for b in c.back {
        match b.slot {
            None => {
                restored.push(uuid_text(b.uid.as_bytes()));
                made.push((b.uid, b.value));
            }
            Some(slot) => {
                reverted.push(uuid_text(b.uid.as_bytes()));
                set.push((slot, b.value));
            }
        }
    }
    if !made.is_empty()
        && let Err(r) = cx.doc.add_many_with_uids(made, STEP)
    {
        return Stop::failed(error(codes::INVALID_INPUT, r.0, Some("item".into()))).into();
    }
    if !set.is_empty() {
        cx.doc.update_many(set, STEP);
    }
    let empty = kentos_commands::MapWork::default();
    let as_approved = content_objects(cx.doc, cx.doc.map_work().unwrap_or(&empty), &c.item) == c.approved;
    CommandResult::Completed {
        output: MapWorkRestored {
            item: c.item,
            restored,
            reverted,
            added: c.added,
            as_approved,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &MapWorkRestore) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would put back, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &MapWorkRestore) -> CommandResult<MapWorkRestored> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Puts the objects back: one undo step “Onaylı hâline getir”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: MapWorkRestore) -> CommandResult<MapWorkRestored> {
    crate::call::recorded(cx, kentos_commands::MAP_WORK_RESTORE, kentos_commands::MAP_WORK_RESTORE_VERSION, input, |cx, input| {
        match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        }
    })
}
