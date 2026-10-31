//! Room modules after their first slice (docs/adr/0779): members added to a definition from an instance and given to
//! the other linked instances (`addMembers`), members taken out with their objects deleted in the other instances or
//! released (`removeMembers`), an instance's objects changed by hand away from the source (drift), an instance catching
//! up whole on a source update, and a level's linked instances repeated on other levels (`repeat`). A child of
//! `modules_edit`, sharing its work and its helpers.

use std::collections::{HashMap, HashSet};

use kentos_commands::{
    ArchModule, Entity, ModuleConflict, ModuleConflictWhy, ModuleCopied, ModuleInstance,
    ModuleLeft, ModuleLeftWhy, ModuleLevelTarget, ModuleMember, ModuleNumbering, ModuleObject,
    ModulePlaced, ModuleSkipWhy, ModuleSkipped, module_kind_ok,
};
use kentos_domain::{Document, Slot, Uuid};
use serde_json::Value;

use super::{
    Work, arch_level_holder, arch_mut, find, in_operation, instance_at, instantiate, inverse,
    keep_placement, kind_words, level_ok, live, next_instance_id, number_mut, object_of, place_all,
    references, remap, storey_order, storeys_fit,
};
use crate::checks::{self, Stop};
use crate::codes;
use crate::openings::fail;

/// The most a coordinate of an instance's object may stand off its source's and still be the source's (drawing units):
/// a matrix's rounding, not a change by hand (docs/adr/0779 §3).
const DRIFT_TOLERANCE: f64 = 1e-6;

/// `unit_not_found` for a group the building program lacks.
pub(super) fn unit_known(doc: &Document, unit: Option<&str>, path: &str) -> Result<(), Stop> {
    match unit {
        Some(u) if doc.program().unit(u).is_none() => Err(fail(
            codes::UNIT_NOT_FOUND,
            format!("“{u}” bölümü bina programında yok; önce Bina programı penceresinden ekleyin."),
            path,
        )),
        _ => Ok(()),
    }
}

/// The fields the document writes itself after a write (an opening's jambs, a space's found rings, a tag's text and
/// anchor, a broken link's reason): not the source's, never a drift.
fn written_by_the_document(e: &Entity) -> &'static [&'static str] {
    match e {
        Entity::Opening(_) => &["place", "broken"],
        Entity::Space(s) if s.space.sources.is_some() => &["parts", "broken"],
        Entity::Space(_) => &["broken"],
        Entity::Tag(_) => &["text", "anchor", "broken"],
        Entity::Dimension(_) => &["broken"],
        Entity::Text(t) if t.label_of.is_some() => &["text"],
        _ => &[],
    }
}

/// Whether two values are the same, numbers within [`DRIFT_TOLERANCE`].
fn near(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) => (x - y).abs() <= DRIFT_TOLERANCE,
            _ => x == y,
        },
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| near(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| near(v, w)))
        }
        _ => a == b,
    }
}

/// Whether `current` is `expected` but for what the document writes itself and the coordinates' rounding.
pub(super) fn same_object(expected: &Entity, current: &Entity) -> bool {
    let skip = written_by_the_document(current);
    let clean = |e: &Entity| {
        let mut v = serde_json::to_value(e).unwrap_or(Value::Null);
        if let Some(o) = v.as_object_mut() {
            o.remove("id");
            for k in skip {
                o.remove(*k);
            }
        }
        v
    };
    near(&clean(expected), &clean(current))
}

/// What the instance's object of `mem` is when it follows the source: the definition placed by the instance's matrix,
/// on its walls as they stand, keeping its own place on the view.
fn expected_of(
    doc: &Document,
    inst: &ModuleInstance,
    mem: &ModuleMember,
    current: &Entity,
) -> Option<Entity> {
    let map: HashMap<String, String> = inst
        .members
        .iter()
        .map(|o| (o.key.clone(), o.uid.clone()))
        .collect();
    let wall_of = |u: &str| object_of(doc, u).map(|(_, e)| e.clone());
    let e = instantiate(doc, mem, inst, &map, &wall_of).ok()?;
    Some(keep_placement(e, current))
}

/// Whether the instance's `current` object of `mem` was changed away from the source.
fn drifted(doc: &Document, inst: &ModuleInstance, mem: &ModuleMember, current: &Entity) -> bool {
    expected_of(doc, inst, mem, current).is_some_and(|e| !same_object(&e, current))
}

/// The keys of an instance whose objects were changed by hand away from the source (docs/adr/0779 §3): not local
/// exceptions, not gone, at the module's version; none for a removed module's instance.
pub fn drifted_keys(doc: &Document, m: &ArchModule, inst: &ModuleInstance) -> Vec<String> {
    if m.removed || inst.version < m.version {
        return Vec::new();
    }
    inst.members
        .iter()
        .filter(|o| !inst.exceptions.contains(&o.key))
        .filter_map(|o| {
            let mem = m.member(&o.key)?;
            let (_, current) = object_of(doc, &o.uid)?;
            drifted(doc, inst, mem, current).then(|| o.key.clone())
        })
        .collect()
}

/// [`drifted_keys`] as the objects' ids.
pub fn drifted_uids(doc: &Document, m: &ArchModule, inst: &ModuleInstance) -> Vec<String> {
    let keys = drifted_keys(doc, m, inst);
    inst.members
        .iter()
        .filter(|o| keys.contains(&o.key))
        .map(|o| o.uid.clone())
        .collect()
}

/// A number written by an instance back to the definition's: its prefix and suffix taken off.
fn unwrapped(n: &str, numbering: &ModuleNumbering) -> String {
    let p = numbering.prefix.as_deref().unwrap_or("");
    let s = numbering.suffix.as_deref().unwrap_or("");
    n.strip_prefix(p)
        .and_then(|r| r.strip_suffix(s))
        .map_or_else(|| n.to_owned(), str::to_owned)
}

/// One instance taking a source update: the changed keys (all, when it is behind the module), each written by its
/// matrix unless gone or a local exception (an object changed by hand is written and said), and the keys it lacks
/// placed as new objects. The new objects' entries, for the instance.
pub(super) fn sync_instance(
    doc: &Document,
    old: &ArchModule,
    m: &ArchModule,
    inst: &ModuleInstance,
    changed: &[String],
    work: &mut Work,
) -> Result<Vec<ModuleObject>, Stop> {
    let whole = inst.version < old.version;
    let mut map: HashMap<String, String> = inst
        .members
        .iter()
        .map(|o| (o.key.clone(), o.uid.clone()))
        .collect();
    let mut entries = Vec::new();
    let mut new_ids: HashMap<String, Uuid> = HashMap::new();
    for mem in &m.members {
        if !map.contains_key(&mem.key) {
            let id = Uuid::now_v7();
            map.insert(mem.key.clone(), id.to_string());
            new_ids.insert(mem.key.clone(), id);
            entries.push(ModuleObject {
                key: mem.key.clone(),
                uid: id.to_string(),
            });
        }
    }
    // The walls first: an opening reads itself back onto its wall as it is renewed.
    let mut order: Vec<&ModuleMember> = m
        .members
        .iter()
        .filter(|mem| new_ids.contains_key(&mem.key) || whole || changed.contains(&mem.key))
        .collect();
    order.sort_by_key(|mem| !matches!(mem.entity, Entity::Wall(_)));
    let mut renewed: HashMap<String, Entity> = HashMap::new();
    let mut fresh: Vec<(Slot, Entity)> = Vec::new();
    for mem in order {
        let wall_of = |u: &str| {
            renewed
                .get(u)
                .cloned()
                .or_else(|| object_of(doc, u).map(|(_, e)| e.clone()))
        };
        if let Some(id) = new_ids.get(&mem.key) {
            let e = instantiate(doc, mem, inst, &map, &wall_of)?;
            renewed.insert(id.to_string(), e.clone());
            work.adds.push((*id, e));
            if !work.report.added_keys.contains(&mem.key) {
                work.report.added_keys.push(mem.key.clone());
            }
            continue;
        }
        let uid = map.get(&mem.key).cloned();
        let Some((slot, current)) = uid.as_deref().and_then(|u| object_of(doc, u)) else {
            work.report.conflicts.push(ModuleConflict {
                instance: inst.id.clone(),
                key: mem.key.clone(),
                uid,
                why: ModuleConflictWhy::Gone,
            });
            continue;
        };
        if inst.exceptions.contains(&mem.key) {
            work.report.conflicts.push(ModuleConflict {
                instance: inst.id.clone(),
                key: mem.key.clone(),
                uid,
                why: ModuleConflictWhy::Exception,
            });
            continue;
        }
        if !whole
            && let Some(was) = old.member(&mem.key)
            && drifted(doc, inst, was, current)
        {
            work.report.conflicts.push(ModuleConflict {
                instance: inst.id.clone(),
                key: mem.key.clone(),
                uid: uid.clone(),
                why: ModuleConflictWhy::Drifted,
            });
        }
        let mut e = instantiate(doc, mem, inst, &map, &wall_of)?;
        e = keep_placement(e, current);
        e.base_mut().id = slot.0;
        if let Some(u) = &uid {
            renewed.insert(u.clone(), e.clone());
        }
        if whole && same_object(&e, current) {
            continue;
        }
        fresh.push((slot, e));
    }
    work.updates.extend(fresh);
    Ok(entries)
}

/// The objects an instance takes for the new members `added` (in the definition), placed by its matrix; the walls
/// first, for the openings on them.
fn place_new(
    doc: &Document,
    inst: &ModuleInstance,
    added: &[ModuleMember],
    work: &mut Work,
) -> Result<Vec<ModuleObject>, Stop> {
    let mut map: HashMap<String, String> = inst
        .members
        .iter()
        .map(|o| (o.key.clone(), o.uid.clone()))
        .collect();
    let ids: Vec<Uuid> = added.iter().map(|_| Uuid::now_v7()).collect();
    for (mem, id) in added.iter().zip(&ids) {
        map.insert(mem.key.clone(), id.to_string());
    }
    let mut order: Vec<(&ModuleMember, &Uuid)> = added.iter().zip(&ids).collect();
    order.sort_by_key(|(mem, _)| !matches!(mem.entity, Entity::Wall(_)));
    let mut made: HashMap<String, Entity> = HashMap::new();
    let mut adds = Vec::new();
    for (mem, id) in order {
        let wall_of = |u: &str| {
            made.get(u)
                .cloned()
                .or_else(|| object_of(doc, u).map(|(_, e)| e.clone()))
        };
        let e = instantiate(doc, mem, inst, &map, &wall_of)?;
        made.insert(id.to_string(), e.clone());
        adds.push((*id, e));
    }
    // In the definition's order.
    adds.sort_by_key(|(id, _)| ids.iter().position(|x| x == id));
    work.adds.extend(adds);
    Ok(added
        .iter()
        .zip(&ids)
        .map(|(mem, id)| ModuleObject {
            key: mem.key.clone(),
            uid: id.to_string(),
        })
        .collect())
}

fn locked_instance(doc: &Document, inst: &ModuleInstance) -> bool {
    inst.members.iter().any(|o| {
        object_of(doc, &o.uid).is_some_and(|(_, e)| doc.layers().is_locked(&e.base().layer_id))
    })
}

/// The storey of an object on `level` in an instance based on `base` (docs/adr/0796 §1): 0 on the base (both none
/// too), the levels above it by their order; none below the base, on another kind of level or past the most storeys.
fn storey_of(order: &[String], base: Option<&str>, level: Option<&str>) -> Option<u32> {
    if base == level {
        return Some(0);
    }
    let b = order.iter().position(|x| Some(x.as_str()) == base)?;
    let l = order.iter().position(|x| Some(x.as_str()) == level)?;
    let d = u32::try_from(l.checked_sub(b)?).ok()?;
    (d < kentos_commands::MAX_MODULE_STOREYS).then_some(d)
}

/// Modüle ekle (docs/adr/0779 §1).
pub(super) fn add_members(
    doc: &Document,
    modules: &mut [ArchModule],
    module: &str,
    instance: &str,
    uids: &[String],
    work: &mut Work,
) -> Result<(), Stop> {
    let k = find(modules, module)?;
    live(&modules[k])?;
    let i = instance_at(&modules[k], instance, "operation.instance")?;
    checks::uids(uids, "Modüle eklenecek nesne verilmedi.").map_err(in_operation)?;
    let found = checks::named(doc, uids).map_err(in_operation)?;
    let inst = modules[k].instances[i].clone();
    let order = storey_order(doc);
    let taken: HashSet<&str> = modules
        .iter()
        .flat_map(|m| {
            m.instances
                .iter()
                .flat_map(|i| i.members.iter().map(|o| o.uid.as_str()))
        })
        .collect();
    let allowed: HashSet<&str> = inst
        .members
        .iter()
        .map(|o| o.uid.as_str())
        .chain(found.iter().map(|(_, _, _, u)| u.as_str()))
        .collect();
    for (j, _, e, uid) in &found {
        let path = format!("operation.uids[{j}]");
        if doc.layers().is_locked(&e.base().layer_id) {
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{}” katmanı kilitli; modüle eklenemez.",
                    doc.layers().path(&e.base().layer_id)
                ),
                &path,
            ));
        }
        let curved = matches!(e, Entity::Wall(w) if w.wall.bulges.as_ref().is_some_and(|b| b.iter().any(|x| *x != 0.0)));
        if !module_kind_ok(e) || curved {
            return Err(fail(
                codes::MODULE_KIND,
                if curved {
                    "Eğrisel duvar modül kapsamında değil; düz duvar ekleyin.".into()
                } else {
                    "Oda modülünde düz duvar, açıklık, mahal, donatı, ölçü, etiket ve yazı olur."
                        .into()
                },
                &path,
            ));
        }
        if taken.contains(uid.as_str()) {
            return Err(fail(
                codes::MODULE_MEMBER_TAKEN,
                "Bu nesne zaten bir modül örneğinin; önce o örneği bağımsızlaştırın.".into(),
                &path,
            ));
        }
        for r in references(doc, e) {
            if !allowed.contains(r.as_str()) {
                let other = object_of(doc, &r).map_or("nesne", |(_, o)| kind_words(o));
                return Err(fail(
                    codes::MODULE_OUTSIDE,
                    format!(
                        "Eklenen {} örneğin dışındaki bir nesneye bağlı ({other}, {r}); onu da ekleyin ya da bağı kaldırın (kapsam dışı).",
                        kind_words(e)
                    ),
                    &path,
                ));
            }
        }
        // On the instance's base level or one above it: an upper storey (docs/adr/0796 §1).
        if arch_level_holder(e) && storey_of(&order, inst.level.as_deref(), e.own_level()).is_none() {
            return Err(fail(
                codes::MODULE_LEVELS,
                "Eklenen nesne örneğin katlarında değil; örneğin taban katındaki ya da üstündeki katlardaki nesneleri ekleyin.".into(),
                &path,
            ));
        }
    }
    let back = inverse(inst.at, inst.rotation, inst.mirror);
    let mut to_key: HashMap<String, String> = inst
        .members
        .iter()
        .map(|o| (o.uid.clone(), o.key.clone()))
        .collect();
    for (_, _, _, uid) in &found {
        to_key.insert((*uid).clone(), (*uid).clone());
    }
    let mut added = Vec::new();
    for (_, _, e, uid) in &found {
        let storey = if arch_level_holder(e) {
            storey_of(&order, inst.level.as_deref(), e.own_level()).unwrap_or(0)
        } else {
            0
        };
        let mut t = crate::transform::by_matrix(e, &back).ok_or_else(|| {
            fail(
                codes::NOT_FINITE,
                "Nesne modüle alınamadı.".into(),
                "operation.uids",
            )
        })?;
        t = remap(&t, &to_key);
        if let Some(a) = arch_mut(&mut t) {
            a.level = None;
        }
        if !inst.numbering.is_empty()
            && let Some(Some(n)) = number_mut(&mut t)
        {
            *n = unwrapped(n, &inst.numbering);
        }
        if let Entity::Space(sp) = &mut t
            && inst.unit.is_some()
            && sp.space.unit == inst.unit
        {
            sp.space.unit = None;
        }
        t.base_mut().id = 0;
        added.push(ModuleMember {
            key: (*uid).clone(),
            entity: t,
            storey,
        });
    }
    let old_version = modules[k].version;
    let version = old_version + 1;
    modules[k].members.extend(added.iter().cloned());
    modules[k].version = version;
    let from = &mut modules[k].instances[i];
    from.members.extend(added.iter().map(|mem| ModuleObject {
        key: mem.key.clone(),
        uid: mem.key.clone(),
    }));
    if from.version == old_version {
        from.version = version;
    }
    work.report.updated.push(inst.id.clone());
    for j in 0..modules[k].instances.len() {
        if j == i {
            continue;
        }
        let other = modules[k].instances[j].clone();
        if locked_instance(doc, &other) {
            work.report.skipped.push(ModuleSkipped {
                instance: other.id.clone(),
                why: ModuleSkipWhy::Locked,
            });
            continue;
        }
        storeys_fit(doc, &modules[k], other.level.as_deref(), "operation.uids")?;
        let entries = place_new(doc, &other, &added, work)?;
        let target = &mut modules[k].instances[j];
        target.members.extend(entries);
        if target.version == old_version {
            target.version = version;
        }
        work.report.updated.push(other.id);
    }
    work.report.added_keys = added.into_iter().map(|m| m.key).collect();
    work.module = Some(modules[k].id.clone());
    work.instance = Some(inst.id);
    Ok(())
}

/// The definition's keys a member's object names: what it leans on.
fn leans_on(mem: &ModuleMember) -> Vec<String> {
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) if s.len() == 36 => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Ok(v) = serde_json::to_value(&mem.entity) {
        walk(&v, &mut out);
    }
    out
}

/// Modülden çıkar (docs/adr/0779 §2).
pub(super) fn remove_members(
    doc: &Document,
    modules: &mut [ArchModule],
    module: &str,
    uids: &[String],
    work: &mut Work,
) -> Result<(), Stop> {
    let k = find(modules, module)?;
    live(&modules[k])?;
    checks::uids(uids, "Modülden çıkarılacak nesne verilmedi.").map_err(in_operation)?;
    let mut keys: Vec<(usize, String)> = Vec::new();
    let mut named: HashSet<String> = HashSet::new();
    for (j, uid) in uids.iter().enumerate() {
        let path = format!("operation.uids[{j}]");
        checks::named(doc, std::slice::from_ref(uid)).map_err(|s| {
            let mut e = s.into_error();
            e.path = Some(path.clone());
            Stop::failed(e)
        })?;
        let key = modules[k].instances.iter().find_map(|inst| {
            inst.members
                .iter()
                .find(|o| &o.uid == uid)
                .map(|o| o.key.clone())
        });
        let Some(key) = key else {
            return Err(fail(
                codes::NOT_IN_MODULE,
                format!("Nesne “{}” modülünün bir örneğinde değil.", modules[k].name),
                &path,
            ));
        };
        named.insert(uid.clone());
        if !keys.iter().any(|(_, x)| *x == key) {
            keys.push((j, key));
        }
    }
    if keys.len() >= modules[k].members.len() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Modülün bütün nesneleri çıkarılamaz; modülü silmek için Kaynağı sil'i kullanın."
                .into(),
            "operation.uids",
        ));
    }
    let gone: HashSet<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    for mem in &modules[k].members {
        if gone.contains(mem.key.as_str()) {
            continue;
        }
        if let Some(needed) = leans_on(mem)
            .into_iter()
            .find(|r| gone.contains(r.as_str()))
        {
            let j = keys
                .iter()
                .find(|(_, k)| *k == needed)
                .map_or(0, |(j, _)| *j);
            let what = modules[k]
                .member(&needed)
                .map_or("nesne", |n| kind_words(&n.entity));
            return Err(fail(
                codes::MODULE_MEMBER_NEEDED,
                format!(
                    "Modülün bir nesnesi ({}) çıkarılan nesneye ({}) bağlı; onu da birlikte çıkarın ya da bırakın.",
                    kind_words(&mem.entity),
                    what
                ),
                &format!("operation.uids[{j}]"),
            ));
        }
    }
    let old_version = modules[k].version;
    let version = old_version + 1;
    for inst in &mut modules[k].instances {
        let mut touched = false;
        for o in inst
            .members
            .iter()
            .filter(|o| gone.contains(o.key.as_str()))
        {
            touched = true;
            if named.contains(&o.uid) {
                work.report.released.push(o.uid.clone());
                continue;
            }
            let Some((slot, e)) = object_of(doc, &o.uid) else {
                continue;
            };
            let why = if inst.exceptions.contains(&o.key) {
                Some(ModuleConflictWhy::Exception)
            } else if doc.layers().is_locked(&e.base().layer_id) {
                Some(ModuleConflictWhy::Locked)
            } else {
                None
            };
            match why {
                Some(why) => {
                    work.report.released.push(o.uid.clone());
                    work.report.conflicts.push(ModuleConflict {
                        instance: inst.id.clone(),
                        key: o.key.clone(),
                        uid: Some(o.uid.clone()),
                        why,
                    });
                }
                None => {
                    work.removes.push(slot);
                    work.report.deleted.push(o.uid.clone());
                }
            }
        }
        inst.members.retain(|o| !gone.contains(o.key.as_str()));
        inst.exceptions.retain(|x| !gone.contains(x.as_str()));
        if inst.version == old_version {
            inst.version = version;
        }
        if touched {
            work.report.updated.push(inst.id.clone());
        }
    }
    modules[k]
        .members
        .retain(|m| !gone.contains(m.key.as_str()));
    modules[k].version = version;
    work.report.removed_keys = keys.into_iter().map(|(_, k)| k).collect();
    work.module = Some(modules[k].id.clone());
    Ok(())
}

/// A refusal of `find` with its path where the input names the module.
fn found_at(modules: &[ArchModule], key: &str, path: &str) -> Result<usize, Stop> {
    find(modules, key).map_err(|s| {
        let mut e = s.into_error();
        e.path = Some(path.into());
        Stop::failed(e)
    })
}

/// The objects of the level `from` in no module a repeat with `loose` copies (docs/adr/0796 §2), in the drawing's order
/// with their ids, and those it leaves, with why: an object of an instance not repeated, on a locked layer, of a kind
/// not copied, or leaning on an object neither copied nor of a repeated instance (again and again: one left leaves
/// those leaning on it).
fn loose_plan(
    doc: &Document,
    modules: &[ArchModule],
    from: &str,
    repeated: &HashSet<String>,
) -> (Vec<(String, Entity)>, Vec<ModuleLeft>) {
    let in_modules: HashSet<&str> = modules
        .iter()
        .flat_map(|m| m.instances.iter().flat_map(|i| i.members.iter().map(|o| o.uid.as_str())))
        .collect();
    let by_uid = |u: &str| {
        Uuid::parse_str(u)
            .ok()
            .and_then(|x| doc.slot_of(x))
            .and_then(|s| doc.get(s))
    };
    // A design option's objects (docs/adr/0788) stay: a plain copy would join the main model.
    let in_options: HashSet<&str> = doc
        .arch_options()
        .iter()
        .flat_map(|s| s.options.iter().flat_map(|o| o.members.iter().map(String::as_str)))
        .collect();
    let mut copy: Vec<(String, Entity)> = Vec::new();
    let mut left: Vec<(u32, ModuleLeft)> = Vec::new();
    for e in doc.entities() {
        if e.level(by_uid) != Some(from) {
            continue;
        }
        let slot = e.base().id;
        let Some(uid) = doc.uid(Slot(slot)).map(|u| u.to_string()) else {
            continue;
        };
        if repeated.contains(&uid) {
            continue;
        }
        let why = if in_modules.contains(uid.as_str()) {
            Some(ModuleLeftWhy::Module)
        } else if in_options.contains(uid.as_str()) {
            Some(ModuleLeftWhy::Option)
        } else if doc.layers().is_locked(&e.base().layer_id) {
            Some(ModuleLeftWhy::Locked)
        } else if !matches!(
            e,
            Entity::Wall(_)
                | Entity::Opening(_)
                | Entity::Space(_)
                | Entity::Insert(_)
                | Entity::Dimension(_)
                | Entity::Tag(_)
        ) {
            Some(ModuleLeftWhy::Kind)
        } else {
            None
        };
        match why {
            Some(why) => left.push((slot, ModuleLeft { uid, why })),
            None => copy.push((uid, e.clone())),
        }
    }
    // What leans outside the copies and the repeated instances stays, and so in turn what leans on it.
    loop {
        let held: HashSet<&str> = copy
            .iter()
            .map(|(u, _)| u.as_str())
            .chain(repeated.iter().map(String::as_str))
            .collect();
        let out: Vec<usize> = copy
            .iter()
            .enumerate()
            .filter(|(_, (_, e))| references(doc, e).iter().any(|r| !held.contains(r.as_str())))
            .map(|(i, _)| i)
            .collect();
        if out.is_empty() {
            break;
        }
        for &i in out.iter().rev() {
            let (uid, e) = copy.remove(i);
            left.push((
                e.base().id,
                ModuleLeft {
                    uid,
                    why: ModuleLeftWhy::Outside,
                },
            ));
        }
    }
    left.sort_by_key(|(slot, _)| *slot);
    (copy, left.into_iter().map(|(_, l)| l).collect())
}

/// A loose space's or opening's number on a repeat's target (docs/adr/0796 §2): the source level's project prefix (a
/// space's, docs/adr/0690) taken off, then the target's numbering put on; with none, the target level's prefix in place
/// of the source's; else as it was.
fn renumbered(
    n: &str,
    from_prefix: Option<&str>,
    to_prefix: Option<&str>,
    numbering: Option<&ModuleNumbering>,
) -> String {
    let stripped = from_prefix
        .filter(|p| !p.is_empty())
        .and_then(|p| n.strip_prefix(p));
    match numbering {
        Some(nb) if !nb.is_empty() => nb.apply(stripped.unwrap_or(n)),
        _ => match (stripped, to_prefix.filter(|p| !p.is_empty())) {
            (Some(rest), Some(tp)) => format!("{tp}{rest}"),
            _ => n.to_owned(),
        },
    }
}

/// Katları tekrarla (docs/adr/0779 §5; the level's objects in no module too, docs/adr/0796 §2).
pub(super) fn repeat(
    doc: &Document,
    modules: &mut [ArchModule],
    from: &str,
    of: &[String],
    to: &[ModuleLevelTarget],
    loose: bool,
    work: &mut Work,
) -> Result<(), Stop> {
    level_ok(doc, Some(from)).map_err(|s| {
        let mut e = s.into_error();
        e.path = Some("operation.from".into());
        Stop::failed(e)
    })?;
    if to.is_empty() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Hedef kat verilmedi; en az bir kat seçin.".into(),
            "operation.to",
        ));
    }
    for (j, t) in to.iter().enumerate() {
        let path = format!("operation.to[{j}].level");
        level_ok(doc, Some(&t.level)).map_err(|s| {
            let mut e = s.into_error();
            e.path = Some(path.clone());
            Stop::failed(e)
        })?;
        if t.level == from {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Kaynak kat “{from}” hedeflerde olamaz."),
                &path,
            ));
        }
        if to[..j].iter().any(|x| x.level == t.level) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{}” katı hedeflerde iki kez geçiyor.", t.level),
                &path,
            ));
        }
        unit_known(doc, t.unit.as_deref(), &format!("operation.to[{j}].unit"))?;
        for (l, one) in t.instances.iter().enumerate() {
            unit_known(
                doc,
                one.unit.as_deref(),
                &format!("operation.to[{j}].instances[{l}].unit"),
            )?;
        }
    }
    let chosen: Vec<usize> = if of.is_empty() {
        (0..modules.len())
            .filter(|&k| !modules[k].removed)
            .collect()
    } else {
        let mut out = Vec::new();
        for (i, key) in of.iter().enumerate() {
            let k = found_at(modules, key, &format!("operation.modules[{i}]"))?;
            live(&modules[k]).map_err(|s| {
                let mut e = s.into_error();
                e.path = Some(format!("operation.modules[{i}]"));
                Stop::failed(e)
            })?;
            if !out.contains(&k) {
                out.push(k);
            }
        }
        out
    };
    let sources: Vec<(usize, ModuleInstance)> = chosen
        .iter()
        .flat_map(|&k| {
            modules[k]
                .instances
                .iter()
                .filter(|i| i.level.as_deref() == Some(from) && i.broken.is_none())
                .map(move |i| (k, i.clone()))
        })
        .collect();
    let repeated: HashSet<String> = sources
        .iter()
        .flat_map(|(_, s)| s.members.iter().map(|o| o.uid.clone()))
        .collect();
    let (copies, left) = if loose {
        loose_plan(doc, modules, from, &repeated)
    } else {
        (Vec::new(), Vec::new())
    };
    if sources.is_empty() && copies.is_empty() {
        return Err(fail(
            codes::MODULE_REPEAT_EMPTY,
            if loose {
                format!("“{from}” katında tekrarlanacak bağlı modül örneği ya da modülsüz nesne yok.")
            } else {
                format!("“{from}” katında tekrarlanacak bağlı modül örneği yok.")
            },
            "operation.from",
        ));
    }
    let prefixes = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.space_number_prefixes.clone())
        .unwrap_or_default();
    for (j, t) in to.iter().enumerate() {
        for (l, one) in t.instances.iter().enumerate() {
            let at = format!("operation.to[{j}].instances[{l}]");
            let k = found_at(modules, &one.module, &format!("{at}.module"))?;
            if !sources
                .iter()
                .any(|(sk, s)| *sk == k && s.id == one.instance)
            {
                return Err(fail(
                    codes::MODULE_INSTANCE_NOT_FOUND,
                    format!(
                        "“{}” örneği “{from}” katında tekrarlanacak örnekler arasında yok.",
                        one.instance
                    ),
                    &format!("{at}.instance"),
                ));
            }
        }
    }
    for &(k, _) in &sources {
        for mem in &modules[k].members {
            let l = &mem.entity.base().layer_id;
            if doc.layers().is_locked(l) {
                return Err(fail(
                    codes::LAYER_LOCKED,
                    format!(
                        "“{}” katmanı kilitli; modül tekrarlanamaz.",
                        doc.layers().path(l)
                    ),
                    "operation.from",
                ));
            }
        }
    }
    let mut involved: Vec<usize> = Vec::new();
    for (j, t) in to.iter().enumerate() {
        // The repeated instances' objects on this target, for the loose copies' references.
        let mut map: HashMap<String, String> = HashMap::new();
        for (k, src) in &sources {
            let k = *k;
            if let Some(there) = modules[k].instances.iter().find(|i| {
                i.level.as_deref() == Some(t.level.as_str())
                    && i.at == src.at
                    && i.rotation == src.rotation
                    && i.mirror == src.mirror
            }) {
                return Err(fail(
                    codes::MODULE_REPEAT_TAKEN,
                    format!(
                        "“{}” modülünün “{}” katında aynı yerde bir örneği var ({}).",
                        modules[k].name, t.level, there.id
                    ),
                    &format!("operation.to[{j}].level"),
                ));
            }
            storeys_fit(doc, &modules[k], Some(&t.level), &format!("operation.to[{j}].level"))?;
            let line = t
                .instances
                .iter()
                .find(|one| one.instance == src.id && find(modules, &one.module).ok() == Some(k));
            let numbering = line
                .and_then(|o| o.numbering.clone())
                .or_else(|| t.numbering.clone())
                .unwrap_or_else(|| src.numbering.clone());
            let unit = line
                .and_then(|o| o.unit.clone())
                .or_else(|| t.unit.clone())
                .or_else(|| src.unit.clone());
            let m = &modules[k];
            let ids: Vec<Uuid> = m.members.iter().map(|_| Uuid::now_v7()).collect();
            let inst = ModuleInstance {
                id: next_instance_id(m),
                at: src.at,
                rotation: src.rotation,
                mirror: src.mirror,
                level: Some(t.level.clone()),
                phase: src.phase,
                numbering,
                unit,
                version: m.version,
                members: m
                    .members
                    .iter()
                    .zip(&ids)
                    .map(|(mem, id)| ModuleObject {
                        key: mem.key.clone(),
                        uid: id.to_string(),
                    })
                    .collect(),
                exceptions: Vec::new(),
                broken: None,
            };
            let made = place_all(doc, m, &inst, &ids)?;
            work.adds.extend(ids.iter().copied().zip(made));
            for o in &src.members {
                if let Some(n) = inst.members.iter().find(|x| x.key == o.key) {
                    map.insert(o.uid.clone(), n.uid.clone());
                }
            }
            work.placed.push(ModulePlaced {
                module: m.id.clone(),
                instance: inst.id.clone(),
                level: inst.level.clone(),
            });
            if !involved.contains(&k) {
                involved.push(k);
            }
            modules[k].instances.push(inst);
        }
        // The level's objects in no module, as plain copies on the target (docs/adr/0796 §2).
        let ids: Vec<Uuid> = copies.iter().map(|_| Uuid::now_v7()).collect();
        for ((uid, _), id) in copies.iter().zip(&ids) {
            map.insert(uid.clone(), id.to_string());
        }
        for ((uid, e), id) in copies.iter().zip(&ids) {
            let mut c = remap(e, &map);
            c.base_mut().id = 0;
            if let Some(a) = arch_mut(&mut c) {
                a.level = Some(t.level.clone());
            }
            let space = matches!(c, Entity::Space(_));
            if let Some(Some(n)) = number_mut(&mut c) {
                *n = if space {
                    renumbered(
                        n,
                        prefixes.get(from).map(String::as_str),
                        prefixes.get(&t.level).map(String::as_str),
                        t.numbering.as_ref(),
                    )
                } else {
                    renumbered(n, None, None, t.numbering.as_ref())
                };
            }
            work.adds.push((*id, c));
            work.copied.push(ModuleCopied {
                source: uid.clone(),
                uid: id.to_string(),
                level: t.level.clone(),
            });
        }
    }
    work.report.left = left;
    if let [k] = involved[..] {
        work.module = Some(modules[k].id.clone());
    }
    Ok(())
}
