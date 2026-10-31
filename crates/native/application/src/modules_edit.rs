//! `cad.modules.edit` v1 (docs/adr/0775): room modules. A module's definition is its objects in its own frame (base at
//! the origin, no level), each under a key; an instance places it (base point, mirror across its own y axis, turn,
//! level, phase, numbering) as ordinary objects whose references (an opening's wall, a space's sources, a dimension's
//! points, a tag's object, a text's object) name the instance's own objects. A source update takes one instance's
//! objects back into the definition and gives the changed members to the other linked instances; a local exception
//! keeps an object's own values; detaching leaves the objects following nothing; removing the definition leaves the
//! instances as they last were, their link broken. Every write is one undo step “Oda modülü”. The shared cases are
//! `fixtures/commands/v1/cad.modules.edit.json` (`scripts/fixtures/module_cases.py`).
//!
//! The second slice (docs/adr/0779) adds members to and takes them out of a definition, marks an instance's objects
//! changed by hand (drifted), gives a flat's instance its own building-program group and repeats a level's instances on
//! other levels (`modules_more.rs`); `fixtures/commands/v1/cad.modules.edit.flats.json`
//! (`scripts/fixtures/module_flats_cases.py`).

use std::collections::{HashMap, HashSet};

use kentos_commands::{
    ArchModule, CommandResult, CommandWarning, Entity, ModuleBreak, ModuleConflictWhy,
    ModuleInstance, ModuleMember, ModuleNumbering, ModuleObject, ModulePlaced, ModuleReport,
    ModuleSkipWhy, ModuleSkipped, ModulesEdit, ModulesEdited, ModulesOperation, Vec2,
    module_kind_ok, opening_type_id_of,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::geom::affine::{Affine, compose, mirror, rotation, translation};
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::openings::fail;

pub const STEP: &str = "Oda modülü";

#[path = "modules_more.rs"]
mod more;
pub use more::{drifted_keys, drifted_uids};

/// What a write does: the modules after it, the objects to add (with their new ids) and to change, what to answer.
#[derive(Default)]
struct Work {
    modules: Vec<ArchModule>,
    adds: Vec<(Uuid, Entity)>,
    updates: Vec<(Slot, Entity)>,
    /// The objects deleted with their keys (Modülden çıkar).
    removes: Vec<Slot>,
    placed: Vec<ModulePlaced>,
    module: Option<String>,
    instance: Option<String>,
    report: ModuleReport,
}

/// The matrix of an instance: its definition mirrored across its own y axis when asked, turned, then moved to its base.
pub(crate) fn matrix(at: Vec2, rotation_rad: f64, mirrored: bool) -> Affine {
    let o = kentos_geometry_core::Vec2::new(0.0, 0.0);
    let flip = if mirrored {
        mirror(o, kentos_geometry_core::Vec2::new(0.0, 1.0))
    } else {
        translation(0.0, 0.0)
    };
    compose(
        &translation(at.x, at.y),
        &compose(&rotation(rotation_rad, o), &flip),
    )
}

/// The inverse of [`matrix`]: from the drawing back into the definition's frame.
pub(crate) fn inverse(at: Vec2, rotation_rad: f64, mirrored: bool) -> Affine {
    let o = kentos_geometry_core::Vec2::new(0.0, 0.0);
    let flip = if mirrored {
        mirror(o, kentos_geometry_core::Vec2::new(0.0, 1.0))
    } else {
        translation(0.0, 0.0)
    };
    compose(
        &flip,
        &compose(&rotation(-rotation_rad, o), &translation(-at.x, -at.y)),
    )
}

/// `e` with every text naming one of `map`'s keys made its value: the object's references to the others.
pub(crate) fn remap(e: &Entity, map: &HashMap<String, String>) -> Entity {
    fn walk(v: &mut Value, map: &HashMap<String, String>) {
        match v {
            Value::String(s) => {
                if let Some(to) = map.get(s.as_str()) {
                    *s = to.clone();
                }
            }
            Value::Array(a) => a.iter_mut().for_each(|x| walk(x, map)),
            Value::Object(o) => o.values_mut().for_each(|x| walk(x, map)),
            _ => {}
        }
    }
    let Ok(mut v) = serde_json::to_value(e) else {
        return e.clone();
    };
    walk(&mut v, map);
    serde_json::from_value(v).unwrap_or_else(|_| e.clone())
}

/// Every text in `e` that names an object of the drawing.
fn references(doc: &Document, e: &Entity) -> Vec<String> {
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) if s.len() == 36 => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Ok(v) = serde_json::to_value(e) {
        walk(&v, &mut out);
    }
    out.retain(|s| {
        Uuid::parse_str(s)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .is_some()
    });
    out.sort();
    out.dedup();
    out
}

fn arch_mut(e: &mut Entity) -> Option<&mut kentos_commands::ArchInfo> {
    match e {
        Entity::Wall(x) => Some(&mut x.wall.arch),
        Entity::Space(x) => Some(&mut x.space.arch),
        Entity::Insert(x) => Some(&mut x.arch),
        _ => None,
    }
}

fn number_mut(e: &mut Entity) -> Option<&mut Option<String>> {
    match e {
        Entity::Space(x) => Some(&mut x.space.number),
        Entity::Opening(x) => Some(&mut x.opening.number),
        _ => None,
    }
}

/// A definition's object as an instance writes it: placed by its matrix, its references on the instance's objects, on
/// its level and in its phase, its number wrapped; an opening read back onto its (new) wall.
fn instantiate(
    member: &ModuleMember,
    inst: &ModuleInstance,
    map: &HashMap<String, String>,
    wall_of: &dyn Fn(&str) -> Option<Entity>,
) -> Result<Entity, Stop> {
    let m = matrix(inst.at, inst.rotation, inst.mirror);
    let mut e = crate::transform::by_matrix(&member.entity, &m).ok_or_else(|| {
        fail(
            codes::NOT_FINITE,
            "Modülün bir nesnesi yerleştirilemedi.".into(),
            "operation",
        )
    })?;
    e = remap(&e, map);
    if let Some(arch) = arch_mut(&mut e) {
        arch.level.clone_from(&inst.level);
        if inst.phase.is_some() {
            arch.phase = inst.phase;
        }
    }
    if !inst.numbering.is_empty()
        && let Some(Some(n)) = number_mut(&mut e)
    {
        *n = inst.numbering.apply(n);
    }
    // A flat's instance is its own group of the building program (docs/adr/0779 §4).
    if let (Some(u), Entity::Space(sp)) = (&inst.unit, &mut e) {
        sp.space.unit = Some(u.clone());
    }
    if let Entity::Opening(o) = &mut e {
        let line =
            wall_of(&o.opening.host).and_then(|w| kentos_domain::opening_follow::wall_line(&w));
        o.opening = kentos_domain::opening_follow::placed_on(&o.opening, line.as_ref(), true);
    }
    Ok(e)
}

/// `new` keeping `old`'s place on the view: a tag's point, a text's point and turn, a dimension's moved text
/// (docs/adr/0775 §4: the model relation travels, the placement stays).
fn keep_placement(new: Entity, old: &Entity) -> Entity {
    let keys: &[&str] = match old {
        Entity::Tag(_) => &["at"],
        Entity::Text(_) => &["p", "rotation"],
        Entity::Dimension(_) => &["textShift"],
        _ => return new,
    };
    let (Ok(mut n), Ok(o)) = (serde_json::to_value(&new), serde_json::to_value(old)) else {
        return new;
    };
    for k in keys {
        match o.get(*k) {
            Some(v) => n[*k] = v.clone(),
            None => {
                if let Some(obj) = n.as_object_mut() {
                    obj.remove(*k);
                }
            }
        }
    }
    serde_json::from_value(n).unwrap_or(new)
}

fn name_ok(name: &str, path: &str) -> Result<(), Stop> {
    if name.trim().is_empty()
        || name.trim() != name
        || name.chars().count() > 120
        || name.chars().any(char::is_control)
    {
        return Err(fail(
            codes::INVALID_INPUT,
            format!(
                "“{name}” modül adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 harf."
            ),
            path,
        ));
    }
    Ok(())
}

fn find(modules: &[ArchModule], key: &str) -> Result<usize, Stop> {
    let folded = key.trim().to_lowercase();
    modules
        .iter()
        .position(|m| m.id == key)
        .or_else(|| modules.iter().position(|m| m.name.to_lowercase() == folded))
        .ok_or_else(|| {
            fail(
                codes::MODULE_NOT_FOUND,
                format!("Projede “{key}” adlı ya da kimlikli oda modülü yok."),
                "operation.module",
            )
        })
}

fn live(m: &ArchModule) -> Result<(), Stop> {
    if m.removed {
        return Err(fail(
            codes::MODULE_REMOVED,
            format!(
                "“{}” modülünün kaynağı silinmiş; örnekleri son hâliyle duruyor.",
                m.name
            ),
            "operation.module",
        ));
    }
    Ok(())
}

fn instance_at(m: &ArchModule, id: &str, path: &str) -> Result<usize, Stop> {
    m.instances.iter().position(|i| i.id == id).ok_or_else(|| {
        fail(
            codes::MODULE_INSTANCE_NOT_FOUND,
            format!("“{}” modülünde “{id}” örneği yok.", m.name),
            path,
        )
    })
}

fn level_ok(doc: &Document, level: Option<&str>) -> Result<(), Stop> {
    if let Some(l) = level
        && !doc
            .settings()
            .arch
            .as_ref()
            .is_some_and(|a| a.levels.iter().any(|x| x.id == l))
    {
        return Err(fail(
            codes::LEVEL_NOT_FOUND,
            format!("Projede “{l}” katı yok."),
            "operation.level",
        ));
    }
    Ok(())
}

fn finite(v: Vec2) -> bool {
    v.x.is_finite() && v.y.is_finite()
}

/// The object an instance key stands for: its slot and the object, none when it is gone.
fn object_of<'a>(doc: &'a Document, uid: &str) -> Option<(Slot, &'a Entity)> {
    let slot = doc.slot_of(Uuid::parse_str(uid).ok()?)?;
    Some((slot, doc.get(slot)?))
}

/// An instance's next id: the module's id and the first number past those taken.
fn next_instance_id(m: &ArchModule) -> String {
    let mut n = m.instances.len() + 1;
    while m.instances.iter().any(|i| i.id == format!("{}-{n}", m.id)) {
        n += 1;
    }
    format!("{}-{n}", m.id)
}

fn kind_words(e: &Entity) -> &'static str {
    match e {
        Entity::Wall(_) => "duvar",
        Entity::Opening(_) => "açıklık",
        Entity::Space(_) => "mahal",
        Entity::Insert(_) => "donatı",
        Entity::Dimension(_) => "ölçü",
        Entity::Tag(_) => "etiket",
        Entity::Text(_) => "yazı",
        _ => "nesne",
    }
}

/// A definition's object without its slot, for comparing.
fn bare(e: &Entity) -> Value {
    let mut e = e.clone();
    e.base_mut().id = 0;
    serde_json::to_value(e).unwrap_or(Value::Null)
}

/// Places every member of `m` as the instance `inst`, the new objects' ids given.
fn place_all(
    doc: &Document,
    m: &ArchModule,
    inst: &ModuleInstance,
    ids: &[Uuid],
) -> Result<Vec<Entity>, Stop> {
    let map: HashMap<String, String> = m
        .members
        .iter()
        .zip(ids)
        .map(|(mem, id)| (mem.key.clone(), id.to_string()))
        .collect();
    // The walls first: an opening reads itself back onto its new wall.
    let mut walls: HashMap<String, Entity> = HashMap::new();
    let none = |_: &str| None;
    for (mem, id) in m.members.iter().zip(ids) {
        if matches!(mem.entity, Entity::Wall(_)) {
            walls.insert(id.to_string(), instantiate(mem, inst, &map, &none)?);
        }
    }
    let wall_of = |uid: &str| {
        walls
            .get(uid)
            .cloned()
            .or_else(|| object_of(doc, uid).map(|(_, e)| e.clone()))
    };
    m.members
        .iter()
        .map(|mem| instantiate(mem, inst, &map, &wall_of))
        .collect()
}

fn check(doc: &Document, input: &ModulesEdit) -> Result<Work, Stop> {
    let mut modules = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.modules.clone())
        .unwrap_or_default();
    let mut work = Work::default();
    match &input.operation {
        ModulesOperation::Define {
            name,
            id,
            uids,
            base,
        } => {
            name_ok(name, "operation.name")?;
            if !finite(*base) {
                return Err(fail(
                    codes::NOT_FINITE,
                    "Taban noktası sonlu sayılar olmalı.".into(),
                    "operation.base",
                ));
            }
            let folded = name.to_lowercase();
            if modules.iter().any(|m| m.name.to_lowercase() == folded) {
                return Err(fail(
                    codes::MODULE_EXISTS,
                    format!("Projede “{name}” adlı bir modül var."),
                    "operation.name",
                ));
            }
            let mid = match id {
                Some(id) => {
                    if !kentos_commands::module_id_ok(id) {
                        return Err(fail(
                            codes::INVALID_INPUT,
                            format!("“{id}” modül kimliği olamaz: küçük harf, rakam ve tire."),
                            "operation.id",
                        ));
                    }
                    if modules.iter().any(|m| &m.id == id) {
                        return Err(fail(
                            codes::MODULE_EXISTS,
                            format!("Projede “{id}” kimlikli bir modül var."),
                            "operation.id",
                        ));
                    }
                    id.clone()
                }
                None => {
                    // The types' rule, “tip” its empty name's word.
                    let stem = opening_type_id_of(name);
                    let stem = if stem == "tip" {
                        "modul".to_owned()
                    } else {
                        stem
                    };
                    let mut mid = stem.clone();
                    let mut n = 2;
                    while modules.iter().any(|m| m.id == mid) {
                        mid = format!("{stem}-{n}");
                        n += 1;
                    }
                    mid
                }
            };
            checks::uids(uids, "Modüle alınacak nesne verilmedi.").map_err(in_operation)?;
            let found = checks::named(doc, uids).map_err(in_operation)?;
            let set: HashSet<&str> = found.iter().map(|(_, _, _, u)| u.as_str()).collect();
            let taken: HashSet<&str> = modules
                .iter()
                .flat_map(|m| {
                    m.instances
                        .iter()
                        .flat_map(|i| i.members.iter().map(|o| o.uid.as_str()))
                })
                .collect();
            let mut level: Option<Option<String>> = None;
            for (i, _, e, uid) in &found {
                let path = format!("operation.uids[{i}]");
                if doc.layers().is_locked(&e.base().layer_id) {
                    return Err(fail(
                        codes::LAYER_LOCKED,
                        format!(
                            "“{}” katmanı kilitli; modüle alınamaz.",
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
                            "Eğrisel duvar ilk modül kapsamında değil; düz duvarlı bir oda seçin."
                                .into()
                        } else {
                            "Oda modülünde düz duvar, açıklık, mahal, donatı, ölçü, etiket ve yazı olur.".into()
                        },
                        &path,
                    ));
                }
                if taken.contains(uid.as_str()) {
                    return Err(fail(
                        codes::MODULE_MEMBER_TAKEN,
                        "Bu nesne zaten bir modül örneğinin; önce o örneği bağımsızlaştırın."
                            .into(),
                        &path,
                    ));
                }
                for r in references(doc, e) {
                    if !set.contains(r.as_str()) {
                        let other = object_of(doc, &r).map_or("nesne", |(_, o)| kind_words(o));
                        return Err(fail(
                            codes::MODULE_OUTSIDE,
                            format!(
                                "Seçimdeki {} seçimin dışındaki bir nesneye bağlı ({other}, {r}); onu da seçin ya da bağı kaldırın (kapsam dışı).",
                                kind_words(e)
                            ),
                            &path,
                        ));
                    }
                }
                if arch_level_holder(e) {
                    let own = e.own_level().map(str::to_owned);
                    match &level {
                        None => level = Some(own),
                        Some(l) if *l != own => {
                            return Err(fail(
                                codes::MODULE_LEVELS,
                                "Modülün nesneleri tek katta olur; başka kattakileri seçimden çıkarın.".into(),
                                &path,
                            ));
                        }
                        Some(_) => {}
                    }
                }
            }
            let back = translation(-base.x, -base.y);
            // A flat (docs/adr/0779 §4): its spaces all in one group make the group the first instance's, the
            // definition's spaces in none.
            let units: Vec<Option<&str>> = found
                .iter()
                .filter_map(|(_, _, e, _)| match e {
                    Entity::Space(s) => Some(s.space.unit.as_deref()),
                    _ => None,
                })
                .collect();
            let unit = match units.first() {
                Some(Some(u)) if units.iter().all(|x| *x == Some(*u)) => Some((*u).to_owned()),
                _ => None,
            };
            let mut members = Vec::new();
            for (_, _, e, uid) in &found {
                let mut t = crate::transform::by_matrix(e, &back).ok_or_else(|| {
                    fail(
                        codes::NOT_FINITE,
                        "Nesne modüle alınamadı.".into(),
                        "operation.uids",
                    )
                })?;
                if let Some(a) = arch_mut(&mut t) {
                    a.level = None;
                }
                if unit.is_some()
                    && let Entity::Space(sp) = &mut t
                {
                    sp.space.unit = None;
                }
                t.base_mut().id = 0;
                members.push(ModuleMember {
                    key: (*uid).clone(),
                    entity: t,
                });
            }
            let inst = ModuleInstance {
                id: format!("{mid}-1"),
                at: *base,
                rotation: 0.0,
                mirror: false,
                level: level.flatten(),
                phase: None,
                numbering: ModuleNumbering::default(),
                unit,
                version: 1,
                members: found
                    .iter()
                    .map(|(_, _, _, u)| ModuleObject {
                        key: (*u).clone(),
                        uid: (*u).clone(),
                    })
                    .collect(),
                exceptions: Vec::new(),
                broken: None,
            };
            work.instance = Some(inst.id.clone());
            work.module = Some(mid.clone());
            modules.push(ArchModule {
                id: mid,
                name: name.clone(),
                version: 1,
                members,
                instances: vec![inst],
                removed: false,
            });
        }
        ModulesOperation::Place {
            module,
            at,
            rotation: turn,
            mirror: mirrored,
            level,
            phase,
            numbering,
            unit,
        } => {
            let k = find(&modules, module)?;
            live(&modules[k])?;
            let turn = turn.unwrap_or(0.0);
            if !finite(*at) || !turn.is_finite() {
                return Err(fail(
                    codes::NOT_FINITE,
                    "Yer ve dönüş sonlu sayılar olmalı.".into(),
                    "operation.at",
                ));
            }
            level_ok(doc, level.as_deref())?;
            more::unit_known(doc, unit.as_deref(), "operation.unit")?;
            for mem in &modules[k].members {
                let l = &mem.entity.base().layer_id;
                if doc.layers().is_locked(l) {
                    return Err(fail(
                        codes::LAYER_LOCKED,
                        format!(
                            "“{}” katmanı kilitli; modül yerleştirilemez.",
                            doc.layers().path(l)
                        ),
                        "operation.module",
                    ));
                }
            }
            let m = &modules[k];
            let ids: Vec<Uuid> = m.members.iter().map(|_| Uuid::now_v7()).collect();
            let inst = ModuleInstance {
                id: next_instance_id(m),
                at: *at,
                rotation: turn,
                mirror: *mirrored,
                level: level.clone(),
                phase: *phase,
                numbering: numbering.clone().unwrap_or_default(),
                unit: unit.clone(),
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
            work.adds = ids.iter().copied().zip(made).collect();
            work.instance = Some(inst.id.clone());
            work.module = Some(m.id.clone());
            modules[k].instances.push(inst);
        }
        ModulesOperation::Update {
            module,
            from,
            instances,
        } => {
            let k = find(&modules, module)?;
            live(&modules[k])?;
            let f = instance_at(&modules[k], from, "operation.from")?;
            let targets: Vec<usize> = if instances.is_empty() {
                (0..modules[k].instances.len())
                    .filter(|&i| i != f)
                    .collect()
            } else {
                instances
                    .iter()
                    .enumerate()
                    .map(|(j, id)| {
                        instance_at(&modules[k], id, &format!("operation.instances[{j}]"))
                    })
                    .collect::<Result<_, _>>()?
            };
            let src = modules[k].instances[f].clone();
            let back = inverse(src.at, src.rotation, src.mirror);
            let to_key: HashMap<String, String> = src
                .members
                .iter()
                .map(|o| (o.uid.clone(), o.key.clone()))
                .collect();
            let mut changed = Vec::new();
            let mut members = modules[k].members.clone();
            for mem in &mut members {
                if src.exceptions.contains(&mem.key) {
                    continue;
                }
                let Some(o) = src.members.iter().find(|o| o.key == mem.key) else {
                    continue;
                };
                let Some((_, e)) = object_of(doc, &o.uid) else {
                    return Err(fail(
                        codes::MODULE_MEMBER_MISSING,
                        format!(
                            "“{from}” örneğinin bir nesnesi ({}) silinmiş; başka bir örnekten güncelleyin.",
                            o.uid
                        ),
                        "operation.from",
                    ));
                };
                let mut t = crate::transform::by_matrix(e, &back).ok_or_else(|| {
                    fail(
                        codes::NOT_FINITE,
                        "Nesne kaynağa alınamadı.".into(),
                        "operation.from",
                    )
                })?;
                t = remap(&t, &to_key);
                if let Some(a) = arch_mut(&mut t) {
                    a.level = None;
                    if src.phase.is_some()
                        && let Some(old) = arch_of(&mem.entity)
                    {
                        a.phase = old.phase;
                    }
                }
                // The definition's numbers stay its own; an instance wraps them.
                if let (Some(n), Some(old)) = (number_mut(&mut t), number_of(&mem.entity)) {
                    *n = old;
                }
                // The definition's groups stay its own; a flat's instance names its group (docs/adr/0779 §4).
                if src.unit.is_some()
                    && let (Entity::Space(sp), Entity::Space(was)) = (&mut t, &mem.entity)
                {
                    sp.space.unit.clone_from(&was.space.unit);
                }
                t.base_mut().id = 0;
                // Within a matrix's rounding the member stays as it was (docs/adr/0779 §3).
                if !more::near_entities(&t, &mem.entity) {
                    changed.push(mem.key.clone());
                    mem.entity = t;
                }
            }
            // Instances behind the module's version, or lacking a key, catch up whole (docs/adr/0779 §1).
            let old_version = modules[k].version;
            let behind = |i: &ModuleInstance| {
                i.version < old_version
                    || modules[k]
                        .members
                        .iter()
                        .any(|mem| i.members.iter().all(|o| o.key != mem.key))
            };
            let catching = targets
                .iter()
                .any(|&t| t != f && behind(&modules[k].instances[t]));
            if changed.is_empty() && !catching {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!("“{from}” örneği kaynakla aynı; güncellenecek bir şey yok."),
                    "operation.from",
                ));
            }
            let old = modules[k].clone();
            let version = if changed.is_empty() {
                old_version
            } else {
                old_version + 1
            };
            modules[k].members = members;
            modules[k].version = version;
            modules[k].instances[f].version = version;
            work.report.updated.push(src.id.clone());
            let m = modules[k].clone();
            for &t in &targets {
                if t == f {
                    continue;
                }
                let inst = m.instances[t].clone();
                let locked = inst.members.iter().any(|o| {
                    object_of(doc, &o.uid)
                        .is_some_and(|(_, e)| doc.layers().is_locked(&e.base().layer_id))
                });
                if locked {
                    work.report.skipped.push(ModuleSkipped {
                        instance: inst.id.clone(),
                        why: ModuleSkipWhy::Locked,
                    });
                    continue;
                }
                let synced = more::sync_instance(doc, &old, &m, &inst, &changed, &mut work)?;
                modules[k].instances[t].members.extend(synced);
                modules[k].instances[t].version = version;
                work.report.updated.push(inst.id.clone());
            }
            work.report.changed_keys = changed;
            work.module = Some(modules[k].id.clone());
            work.instance = Some(src.id);
        }
        ModulesOperation::AddMembers {
            module,
            instance,
            uids,
        } => more::add_members(doc, &mut modules, module, instance, uids, &mut work)?,
        ModulesOperation::RemoveMembers { module, uids } => {
            more::remove_members(doc, &mut modules, module, uids, &mut work)?;
        }
        ModulesOperation::Repeat {
            from,
            modules: of,
            to,
        } => {
            more::repeat(doc, &mut modules, from, of, to, &mut work)?;
        }
        ModulesOperation::Except { module, uids } | ModulesOperation::Restore { module, uids } => {
            let restore = matches!(input.operation, ModulesOperation::Restore { .. });
            let k = find(&modules, module)?;
            if restore {
                live(&modules[k])?;
            }
            checks::uids(uids, "Nesne verilmedi.").map_err(in_operation)?;
            let mut touched = false;
            for (j, uid) in uids.iter().enumerate() {
                let path = format!("operation.uids[{j}]");
                checks::named(doc, std::slice::from_ref(uid)).map_err(|s| {
                    let mut e = s.into_error();
                    e.path = Some(path.clone());
                    Stop::failed(e)
                })?;
                let found = modules[k]
                    .instances
                    .iter()
                    .enumerate()
                    .find_map(|(i, inst)| {
                        inst.members
                            .iter()
                            .find(|o| &o.uid == uid)
                            .map(|o| (i, o.key.clone()))
                    });
                let Some((i, key)) = found else {
                    return Err(fail(
                        codes::NOT_IN_MODULE,
                        format!("Nesne “{}” modülünün bir örneğinde değil.", modules[k].name),
                        &path,
                    ));
                };
                let has = modules[k].instances[i].exceptions.contains(&key);
                if restore {
                    let inst = modules[k].instances[i].clone();
                    let map: HashMap<String, String> = inst
                        .members
                        .iter()
                        .map(|o| (o.key.clone(), o.uid.clone()))
                        .collect();
                    let Some(mem) = modules[k].member(&key).cloned() else {
                        continue;
                    };
                    let Some((slot, old)) = object_of(doc, uid) else {
                        continue;
                    };
                    let wall_of = |u: &str| object_of(doc, u).map(|(_, e)| e.clone());
                    let mut e = instantiate(&mem, &inst, &map, &wall_of)?;
                    e = keep_placement(e, old);
                    e.base_mut().id = slot.0;
                    if bare(&e) != bare(old) || has {
                        touched = true;
                    }
                    if doc.layers().is_locked(&old.base().layer_id) {
                        return Err(fail(
                            codes::LAYER_LOCKED,
                            "Nesnenin katmanı kilitli; kaynağa döndürülemez.".into(),
                            &path,
                        ));
                    }
                    work.updates.push((slot, e));
                    modules[k].instances[i].exceptions.retain(|x| x != &key);
                } else if !has {
                    modules[k].instances[i].exceptions.push(key);
                    touched = true;
                }
                work.instance
                    .get_or_insert(modules[k].instances[i].id.clone());
            }
            if !touched {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    if restore {
                        "Nesneler kaynakla aynı; döndürülecek bir şey yok.".into()
                    } else {
                        "Nesneler zaten yerel istisna.".into()
                    },
                    "operation.uids",
                ));
            }
            work.module = Some(modules[k].id.clone());
        }
        ModulesOperation::Detach { module, instance } => {
            let k = find(&modules, module)?;
            let i = instance_at(&modules[k], instance, "operation.instance")?;
            modules[k].instances.remove(i);
            work.instance = Some(instance.clone());
            work.module = Some(modules[k].id.clone());
            if modules[k].removed && modules[k].instances.is_empty() {
                modules.remove(k);
            }
        }
        ModulesOperation::Rename { module, name } => {
            let k = find(&modules, module)?;
            name_ok(name, "operation.name")?;
            if modules[k].name == *name {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Modülün adı zaten bu.".into(),
                    "operation.name",
                ));
            }
            let folded = name.to_lowercase();
            if modules
                .iter()
                .enumerate()
                .any(|(i, m)| i != k && m.name.to_lowercase() == folded)
            {
                return Err(fail(
                    codes::MODULE_EXISTS,
                    format!("Projede “{name}” adlı bir modül var."),
                    "operation.name",
                ));
            }
            modules[k].name = name.clone();
            work.module = Some(modules[k].id.clone());
        }
        ModulesOperation::Remove { module } => {
            let k = find(&modules, module)?;
            live(&modules[k])?;
            work.module = Some(modules[k].id.clone());
            if modules[k].instances.is_empty() {
                modules.remove(k);
            } else {
                let m = &mut modules[k];
                m.removed = true;
                m.members.clear();
                for inst in &mut m.instances {
                    inst.broken = Some(ModuleBreak::SourceGone);
                    inst.exceptions.clear();
                }
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    if let Some((path, why)) = kentos_commands::arch_modules_problem(&modules) {
        return Err(fail(codes::INVALID_INPUT, why, &path));
    }
    work.modules = modules;
    Ok(work)
}

/// A check's refusal with its path under `operation`.
fn in_operation(s: Stop) -> Stop {
    let mut e = s.into_error();
    e.path = e.path.map(|p| format!("operation.{p}"));
    Stop::failed(e)
}

fn arch_level_holder(e: &Entity) -> bool {
    matches!(e, Entity::Wall(_) | Entity::Space(_) | Entity::Insert(_))
}

fn arch_of(e: &Entity) -> Option<&kentos_commands::ArchInfo> {
    match e {
        Entity::Wall(x) => Some(&x.wall.arch),
        Entity::Space(x) => Some(&x.space.arch),
        Entity::Insert(x) => Some(&x.arch),
        _ => None,
    }
}

fn number_of(e: &Entity) -> Option<Option<String>> {
    match e {
        Entity::Space(x) => Some(x.space.number.clone()),
        Entity::Opening(x) => Some(x.opening.number.clone()),
        _ => None,
    }
}

fn warnings(r: &ModuleReport) -> Vec<CommandWarning> {
    let mut out = Vec::new();
    // Taking members out: an exception's object is released, not passed by.
    let removing = !r.removed_keys.is_empty();
    if !r.skipped.is_empty() {
        out.push(CommandWarning {
            code: codes::MODULE_INSTANCE_SKIPPED.into(),
            message: format!(
                "{} örnek kilitli katmandaki nesneleri yüzünden güncellenmedi: {}.",
                r.skipped.len(),
                r.skipped
                    .iter()
                    .map(|s| s.instance.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            path: Some("operation".into()),
        });
    }
    let kept = r
        .conflicts
        .iter()
        .filter(|c| c.why == ModuleConflictWhy::Exception)
        .count();
    if kept > 0 && !removing {
        out.push(CommandWarning {
            code: codes::MODULE_EXCEPTION_KEPT.into(),
            message: format!(
                "{kept} nesne yerel istisna olduğu için kaynağın değişikliğini almadı."
            ),
            path: Some("operation".into()),
        });
    }
    let gone = r
        .conflicts
        .iter()
        .filter(|c| c.why == ModuleConflictWhy::Gone)
        .count();
    if gone > 0 {
        out.push(CommandWarning {
            code: codes::MODULE_MEMBER_GONE.into(),
            message: format!("{gone} örnek nesnesi silinmiş; kaynağın değişikliği ona gitmedi."),
            path: Some("operation".into()),
        });
    }
    let drifted = r
        .conflicts
        .iter()
        .filter(|c| c.why == ModuleConflictWhy::Drifted)
        .count();
    if drifted > 0 {
        out.push(CommandWarning {
            code: codes::MODULE_DRIFT_OVERWRITTEN.into(),
            message: format!(
                "Elle değiştirilmiş {drifted} örnek nesnesi kaynağın değeriyle yazıldı; korumak için önce yerel istisna yapın."
            ),
            path: Some("operation".into()),
        });
    }
    let released = r
        .conflicts
        .iter()
        .filter(|c| {
            matches!(
                c.why,
                ModuleConflictWhy::Locked | ModuleConflictWhy::Exception
            )
        })
        .count();
    if removing && released > 0 {
        out.push(CommandWarning {
            code: codes::MODULE_MEMBER_RELEASED.into(),
            message: format!(
                "Tanımdan çıkan nesnenin {released} karşılığı silinmedi (yerel istisna ya da kilitli katman); sıradan nesne olarak kaldı."
            ),
            path: Some("operation".into()),
        });
    }
    out
}

fn output(doc: &Document, work: &Work, created: Vec<String>) -> ModulesEdited {
    ModulesEdited {
        module: work
            .module
            .as_ref()
            .and_then(|id| work.modules.iter().find(|m| &m.id == id).cloned()),
        instance: work.instance.clone(),
        created,
        changed: work
            .updates
            .iter()
            .filter_map(|(s, _)| doc.uid(*s).map(|u| u.to_string()))
            .collect(),
        placed: work.placed.clone(),
        report: work.report.clone(),
        revision: doc.revision().to_string(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ModulesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(w) => CommandResult::Completed {
            output: (),
            warnings: warnings(&w.report),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ModulesEdit) -> CommandResult<ModulesEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: the tool's and the window's preview.
pub fn plan_in(doc: &Document, input: &ModulesEdit) -> CommandResult<ModulesEdited> {
    match check(doc, input) {
        Ok(work) => {
            let created = work.adds.iter().map(|(u, _)| u.to_string()).collect();
            CommandResult::Completed {
                warnings: warnings(&work.report),
                output: output(doc, &work, created),
            }
        }
        Err(stop) => stop.into(),
    }
}

/// The objects a placement would add, for a preview.
pub fn placed_objects(doc: &Document, input: &ModulesEdit) -> Result<Vec<Entity>, String> {
    check(doc, input)
        .map(|w| w.adds.into_iter().map(|(_, e)| e).collect())
        .map_err(|s| s.into_error().message)
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ModulesEdit) -> CommandResult<ModulesEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_MODULES_EDIT,
        kentos_commands::CAD_MODULES_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ModulesEdit,
) -> CommandResult<ModulesEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let created: Vec<String> = work.adds.iter().map(|(u, _)| u.to_string()).collect();
    let result = doc.transact(STEP, |d| {
        if !work.adds.is_empty() {
            d.add_many_with_uids(work.adds.clone(), STEP)?;
        }
        if !work.updates.is_empty() {
            d.update_many(work.updates.clone(), STEP);
        }
        if !work.removes.is_empty() {
            d.remove(&work.removes);
        }
        d.set_arch_modules(work.modules.clone(), STEP)?;
        Ok::<(), kentos_domain::Refusal>(())
    });
    if let Err(e) = result {
        return CommandResult::Failed {
            error: error(codes::INVALID_INPUT, e.0, Some("operation".into())),
        };
    }
    CommandResult::Completed {
        warnings: warnings(&work.report),
        output: output(doc, &work, created),
    }
}
