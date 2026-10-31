//! `cad.options.edit` v1 (docs/adr/0788): design options. A set of alternatives for one part of the plan is defined
//! from its objects (they become the first option, the primary), an option is copied as an independent alternative in
//! the same place (its objects' references to each other made the copies'), objects move into and out of an option,
//! another option becomes primary, an option is accepted (the set's other options' objects deleted, the set gone) or
//! deleted, names change. A main-model object never leans on an option's object, nor an option's object on another
//! option's: such a write is refused (`option_leaning`). Every write is one undo step “Tasarım seçeneği”. The shared
//! cases are `fixtures/commands/v1/cad.options.edit.json` (`scripts/fixtures/option_cases.py`).

use std::collections::{HashMap, HashSet};

use kentos_commands::{
    CommandResult, DesignOption, Entity, OptionSet, OptionsEdit, OptionsEdited, OptionsOperation,
    opening_type_id_of, option_set,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::openings::fail;

pub const STEP: &str = "Tasarım seçeneği";

/// The drawing as it is built (docs/adr/0788 §5): a copy without the options its sets do not show; none when it
/// shows every object (no options). Mimari denetim reads it, so two options of a set are never two overlapping walls.
pub fn primary_view(doc: &Document) -> Option<Document> {
    if doc.arch_options().is_empty() {
        return None;
    }
    let out: Vec<Slot> = doc.option_out().into_iter().collect();
    if out.is_empty() {
        return None;
    }
    let mut copy = doc.reading_copy();
    copy.remove(&out);
    Some(copy)
}

/// What a write does: the sets after it, the objects to add (with their new ids) and to delete, what to answer.
#[derive(Default)]
struct Work {
    sets: Vec<OptionSet>,
    adds: Vec<(Uuid, Entity)>,
    removes: Vec<Slot>,
    set: Option<String>,
    option: Option<String>,
    released: Vec<String>,
    deleted: Vec<String>,
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
                "“{name}” seçenek adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 harf."
            ),
            path,
        ));
    }
    Ok(())
}

fn set_at(sets: &[OptionSet], key: &str) -> Result<usize, Stop> {
    option_set(sets, key)
        .and_then(|s| sets.iter().position(|x| x.id == s.id))
        .ok_or_else(|| {
            fail(
                codes::OPTION_SET_NOT_FOUND,
                format!("Projede “{key}” adlı ya da kimlikli tasarım seçeneği kümesi yok."),
                "operation.set",
            )
        })
}

fn option_at(set: &OptionSet, key: &str, path: &str) -> Result<usize, Stop> {
    set.option(key)
        .and_then(|o| set.options.iter().position(|x| x.id == o.id))
        .ok_or_else(|| {
            fail(
                codes::OPTION_NOT_FOUND,
                format!("“{}” kümesinde “{key}” adlı ya da kimlikli seçenek yok.", set.name),
                path,
            )
        })
}

/// The first letter a–z no option of `set` has as its id: a new option's id, its name the letter in capitals.
fn next_letter(set: &OptionSet) -> String {
    ('a'..='z')
        .map(String::from)
        .find(|l| set.options.iter().all(|o| &o.id != l))
        .unwrap_or_else(|| format!("s{}", set.options.len() + 1))
}

fn present(doc: &Document, uid: &str) -> Option<Slot> {
    Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u))
}

/// The sets with their options' objects gone from the drawing left out.
fn pruned(doc: &Document, sets: &[OptionSet]) -> Vec<OptionSet> {
    let mut out = sets.to_vec();
    for s in &mut out {
        for o in &mut s.options {
            o.members.retain(|m| present(doc, m).is_some());
        }
    }
    out
}

/// Every text in `e` that names another object of the drawing.
fn references(doc: &Document, e: &Entity) -> Vec<String> {
    fn walk(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::String(s) if s.len() == 36 => out.push(s.clone()),
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            serde_json::Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Ok(v) = serde_json::to_value(e) {
        walk(&v, &mut out);
    }
    out.retain(|s| present(doc, s).is_some());
    out.sort();
    out.dedup();
    out
}

/// Whether an object of this kind can name another (an opening its wall, a tag its object, a dimension its points …):
/// the kinds that never do are not read.
fn may_lean(e: &Entity) -> bool {
    !matches!(
        e,
        Entity::Line(_)
            | Entity::Point(_)
            | Entity::Circle(_)
            | Entity::Arc(_)
            | Entity::Polygon(_)
            | Entity::Wall(_)
            | Entity::Ellipse(_)
            | Entity::Spline(_)
            | Entity::Nurbs(_)
    )
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

/// Refused when, with `sets`, a main-model object leans on an option's object, or an option's object on another
/// option's (docs/adr/0788 §1): accepting one option would leave the other's references hanging.
fn leaning(doc: &Document, sets: &[OptionSet]) -> Result<(), Stop> {
    let mut group: HashMap<&str, (usize, usize)> = HashMap::new();
    for (i, s) in sets.iter().enumerate() {
        for (j, o) in s.options.iter().enumerate() {
            for m in &o.members {
                group.insert(m.as_str(), (i, j));
            }
        }
    }
    if group.is_empty() {
        return Ok(());
    }
    for (uid, e) in doc.entities_with_uids() {
        if !may_lean(e) {
            continue;
        }
        let me = uid.to_string();
        let mine = group.get(me.as_str()).copied();
        for r in references(doc, e) {
            let theirs = group.get(r.as_str()).copied();
            if theirs.is_some() && theirs != mine {
                let other = present(doc, &r)
                    .and_then(|s| doc.get(s))
                    .map_or("nesne", kind_words);
                let (s, o) = theirs.unwrap_or_default();
                let where_ = match mine {
                    None => "Ana modeldeki".to_owned(),
                    Some((a, b)) => format!(
                        "“{}” kümesinin “{}” seçeneğindeki",
                        sets[a].name, sets[a].options[b].name
                    ),
                };
                return Err(fail(
                    codes::OPTION_LEANING,
                    format!(
                        "{where_} {} ({me}) “{}” kümesinin “{}” seçeneğindeki bir {other} nesnesine bağlı ({r}); bağlı nesneyi de aynı seçeneğe alın ya da bağı kaldırın.",
                        kind_words(e),
                        sets[s].name,
                        sets[s].options[o].name
                    ),
                    "operation.uids",
                ));
            }
        }
    }
    Ok(())
}

/// `uids` named and in no option: their slots and ids; refused for one in an option already.
fn free_objects<'a>(
    doc: &'a Document,
    sets: &[OptionSet],
    uids: &'a [String],
) -> Result<Vec<String>, Stop> {
    checks::uids(uids, "Seçeneğe alınacak nesne verilmedi.").map_err(in_operation)?;
    let found = checks::named(doc, uids).map_err(in_operation)?;
    let mut out = Vec::new();
    for (i, _, _, uid) in &found {
        if kentos_commands::option_of(sets, uid).is_some() {
            return Err(fail(
                codes::OPTION_MEMBER_TAKEN,
                "Bu nesne zaten bir tasarım seçeneğinde; önce o seçenekten çıkarın.".into(),
                &format!("operation.uids[{i}]"),
            ));
        }
        out.push((*uid).clone());
    }
    Ok(out)
}

fn in_operation(s: Stop) -> Stop {
    let mut e = s.into_error();
    e.path = e.path.map(|p| format!("operation.{p}"));
    Stop::failed(e)
}

/// Refused when one of `slots` is on a locked layer: an option's objects are made or deleted whole.
fn unlocked(doc: &Document, slots: &[Slot]) -> Result<(), Stop> {
    for &s in slots {
        if let Some(e) = doc.get(s)
            && doc.layers().is_locked(&e.base().layer_id)
        {
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{}” katmanı kilitli; seçeneğin nesneleri yazılamaz. Katmanın kilidini açın.",
                    doc.layers().path(&e.base().layer_id)
                ),
                "operation",
            ));
        }
    }
    Ok(())
}

/// `e` with every text naming one of `map`'s keys made its value.
fn remap(e: &Entity, map: &HashMap<String, String>) -> Entity {
    fn walk(v: &mut serde_json::Value, map: &HashMap<String, String>) {
        match v {
            serde_json::Value::String(s) => {
                if let Some(to) = map.get(s.as_str()) {
                    *s = to.clone();
                }
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(|x| walk(x, map)),
            serde_json::Value::Object(o) => o.values_mut().for_each(|x| walk(x, map)),
            _ => {}
        }
    }
    let Ok(mut v) = serde_json::to_value(e) else {
        return e.clone();
    };
    walk(&mut v, map);
    serde_json::from_value(v).unwrap_or_else(|_| e.clone())
}

fn check(doc: &Document, input: &OptionsEdit) -> Result<Work, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut sets = pruned(doc, doc.arch_options());
    let mut work = Work::default();
    match &input.operation {
        OptionsOperation::Define {
            name,
            id,
            option,
            uids,
        } => {
            name_ok(name, "operation.name")?;
            let folded = name.to_lowercase();
            if sets.iter().any(|s| s.name.to_lowercase() == folded) {
                return Err(fail(
                    codes::OPTION_SET_EXISTS,
                    format!("Projede “{name}” adlı bir seçenek kümesi var."),
                    "operation.name",
                ));
            }
            let sid = match id {
                Some(id) => {
                    if !kentos_commands::option_id_ok(id) {
                        return Err(fail(
                            codes::INVALID_INPUT,
                            format!("“{id}” küme kimliği olamaz: küçük harf, rakam ve tire."),
                            "operation.id",
                        ));
                    }
                    if sets.iter().any(|s| &s.id == id) {
                        return Err(fail(
                            codes::OPTION_SET_EXISTS,
                            format!("Projede “{id}” kimlikli bir seçenek kümesi var."),
                            "operation.id",
                        ));
                    }
                    id.clone()
                }
                None => {
                    let stem = opening_type_id_of(name);
                    let stem = if stem == "tip" { "secenek".to_owned() } else { stem };
                    let mut sid = stem.clone();
                    let mut n = 2;
                    while sets.iter().any(|s| s.id == sid) {
                        sid = format!("{stem}-{n}");
                        n += 1;
                    }
                    sid
                }
            };
            let first = option.clone().unwrap_or_else(|| "A".to_owned());
            name_ok(&first, "operation.option")?;
            if sets.len() >= kentos_commands::MAX_OPTION_SETS {
                return Err(fail(
                    codes::INVALID_INPUT,
                    format!("Bir projede en çok {} seçenek kümesi olur.", kentos_commands::MAX_OPTION_SETS),
                    "operation",
                ));
            }
            let members = free_objects(doc, &sets, uids)?;
            sets.push(OptionSet {
                id: sid.clone(),
                name: name.clone(),
                primary: "a".into(),
                options: vec![DesignOption {
                    id: "a".into(),
                    name: first,
                    members,
                }],
            });
            leaning(doc, &sets)?;
            work.set = Some(sid);
            work.option = Some("a".into());
        }
        OptionsOperation::Copy {
            set,
            option,
            name,
            primary,
        } => {
            let si = set_at(&sets, set)?;
            let oi = option_at(&sets[si], option, "operation.option")?;
            if sets[si].options.len() >= kentos_commands::MAX_OPTIONS {
                return Err(fail(
                    codes::OPTION_SET_FULL,
                    format!(
                        "“{}” kümesinde en çok {} seçenek olur; birini silin ya da kabul edin.",
                        sets[si].name,
                        kentos_commands::MAX_OPTIONS
                    ),
                    "operation.set",
                ));
            }
            let id = next_letter(&sets[si]);
            let new_name = name.clone().unwrap_or_else(|| id.to_uppercase());
            name_ok(&new_name, "operation.name")?;
            let folded = new_name.to_lowercase();
            if sets[si].options.iter().any(|o| o.name.to_lowercase() == folded) {
                return Err(fail(
                    codes::OPTION_EXISTS,
                    format!("“{}” kümesinde “{new_name}” adlı bir seçenek var.", sets[si].name),
                    "operation.name",
                ));
            }
            // The copies in the document's order, each with a new id; their references to each other the copies'.
            let source: HashSet<&str> = sets[si].options[oi]
                .members
                .iter()
                .map(String::as_str)
                .collect();
            let originals: Vec<(Slot, String)> = doc
                .entities_with_uids()
                .filter_map(|(u, e)| {
                    let t = u.to_string();
                    source.contains(t.as_str()).then(|| (Slot(e.base().id), t))
                })
                .collect();
            unlocked(doc, &originals.iter().map(|(s, _)| *s).collect::<Vec<_>>())?;
            let map: HashMap<String, String> = originals
                .iter()
                .map(|(_, u)| (u.clone(), Uuid::now_v7().to_string()))
                .collect();
            for (slot, u) in &originals {
                let Some(e) = doc.get(*slot) else { continue };
                let copy = remap(e, &map);
                let Some(new) = map.get(u).and_then(|n| Uuid::parse_str(n).ok()) else {
                    continue;
                };
                work.adds.push((new, copy));
            }
            let members: Vec<String> = work.adds.iter().map(|(u, _)| u.to_string()).collect();
            sets[si].options.push(DesignOption {
                id: id.clone(),
                name: new_name,
                members,
            });
            if *primary {
                sets[si].primary = id.clone();
            }
            work.set = Some(sets[si].id.clone());
            work.option = Some(id);
        }
        OptionsOperation::AddMembers { set, option, uids } => {
            let si = set_at(&sets, set)?;
            let oi = option_at(&sets[si], option, "operation.option")?;
            let members = free_objects(doc, &sets, uids)?;
            sets[si].options[oi].members.extend(members);
            leaning(doc, &sets)?;
            work.set = Some(sets[si].id.clone());
            work.option = Some(sets[si].options[oi].id.clone());
        }
        OptionsOperation::RemoveMembers { set, uids } => {
            let si = set_at(&sets, set)?;
            checks::uids(uids, "Seçenekten çıkarılacak nesne verilmedi.").map_err(in_operation)?;
            let found = checks::named(doc, uids).map_err(in_operation)?;
            for (i, _, _, uid) in &found {
                let at = sets[si]
                    .options
                    .iter()
                    .position(|o| o.members.iter().any(|m| m == *uid));
                let Some(oi) = at else {
                    return Err(fail(
                        codes::NOT_IN_OPTION,
                        format!("Nesne “{}” kümesinin bir seçeneğinde değil.", sets[si].name),
                        &format!("operation.uids[{i}]"),
                    ));
                };
                sets[si].options[oi].members.retain(|m| m != *uid);
                work.released.push((*uid).clone());
            }
            leaning(doc, &sets)?;
            work.set = Some(sets[si].id.clone());
        }
        OptionsOperation::Primary { set, option } => {
            let si = set_at(&sets, set)?;
            let oi = option_at(&sets[si], option, "operation.option")?;
            let id = sets[si].options[oi].id.clone();
            if sets[si].primary == id {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!("“{}” zaten birincil seçenek.", sets[si].options[oi].name),
                    "operation.option",
                ));
            }
            sets[si].primary = id.clone();
            work.set = Some(sets[si].id.clone());
            work.option = Some(id);
        }
        OptionsOperation::Accept { set, option } => {
            let si = set_at(&sets, set)?;
            let oi = match option {
                Some(o) => option_at(&sets[si], o, "operation.option")?,
                None => sets[si]
                    .options
                    .iter()
                    .position(|o| o.id == sets[si].primary)
                    .unwrap_or(0),
            };
            let gone: Vec<(Slot, String)> = sets[si]
                .options
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != oi)
                .flat_map(|(_, o)| o.members.iter())
                .filter_map(|m| present(doc, m).map(|s| (s, m.clone())))
                .collect();
            unlocked(doc, &gone.iter().map(|(s, _)| *s).collect::<Vec<_>>())?;
            work.option = Some(sets[si].options[oi].id.clone());
            work.released = sets[si].options[oi].members.clone();
            work.removes = gone.iter().map(|(s, _)| *s).collect();
            work.deleted = gone.into_iter().map(|(_, u)| u).collect();
            sets.remove(si);
        }
        OptionsOperation::Delete { set, option } => {
            let si = set_at(&sets, set)?;
            let oi = option_at(&sets[si], option, "operation.option")?;
            if sets[si].options[oi].id == sets[si].primary {
                return Err(fail(
                    codes::OPTION_PRIMARY,
                    format!(
                        "“{}” birincil seçenek; silinemez. Önce başka bir seçeneği birincil yapın ya da seçeneği kabul edin.",
                        sets[si].options[oi].name
                    ),
                    "operation.option",
                ));
            }
            let gone: Vec<(Slot, String)> = sets[si].options[oi]
                .members
                .iter()
                .filter_map(|m| present(doc, m).map(|s| (s, m.clone())))
                .collect();
            unlocked(doc, &gone.iter().map(|(s, _)| *s).collect::<Vec<_>>())?;
            work.removes = gone.iter().map(|(s, _)| *s).collect();
            work.deleted = gone.into_iter().map(|(_, u)| u).collect();
            work.option = Some(sets[si].options[oi].id.clone());
            sets[si].options.remove(oi);
            work.set = Some(sets[si].id.clone());
        }
        OptionsOperation::Rename { set, option, name } => {
            name_ok(name, "operation.name")?;
            let si = set_at(&sets, set)?;
            let folded = name.to_lowercase();
            match option {
                None => {
                    if sets[si].name == *name {
                        return Err(fail(codes::NOTHING_TO_SET, "Küme zaten bu adda.".into(), "operation.name"));
                    }
                    if sets
                        .iter()
                        .enumerate()
                        .any(|(j, s)| j != si && s.name.to_lowercase() == folded)
                    {
                        return Err(fail(
                            codes::OPTION_SET_EXISTS,
                            format!("Projede “{name}” adlı bir seçenek kümesi var."),
                            "operation.name",
                        ));
                    }
                    sets[si].name = name.clone();
                }
                Some(o) => {
                    let oi = option_at(&sets[si], o, "operation.option")?;
                    if sets[si].options[oi].name == *name {
                        return Err(fail(codes::NOTHING_TO_SET, "Seçenek zaten bu adda.".into(), "operation.name"));
                    }
                    if sets[si]
                        .options
                        .iter()
                        .enumerate()
                        .any(|(j, x)| j != oi && x.name.to_lowercase() == folded)
                    {
                        return Err(fail(
                            codes::OPTION_EXISTS,
                            format!("“{}” kümesinde “{name}” adlı bir seçenek var.", sets[si].name),
                            "operation.name",
                        ));
                    }
                    sets[si].options[oi].name = name.clone();
                    work.option = Some(sets[si].options[oi].id.clone());
                }
            }
            work.set = Some(sets[si].id.clone());
        }
    }
    work.sets = sets;
    Ok(work)
}

fn output(doc: &Document, work: &Work, created: Vec<String>) -> OptionsEdited {
    OptionsEdited {
        set: work
            .set
            .as_ref()
            .and_then(|id| work.sets.iter().find(|s| &s.id == id).cloned()),
        option: work.option.clone(),
        created,
        deleted: work.deleted.clone(),
        released: work.released.clone(),
        revision: doc.revision().to_string(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &OptionsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &OptionsEdit) -> CommandResult<OptionsEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: the window's preview.
pub fn plan_in(doc: &Document, input: &OptionsEdit) -> CommandResult<OptionsEdited> {
    match check(doc, input) {
        Ok(work) => {
            let created = work.adds.iter().map(|(u, _)| u.to_string()).collect();
            CommandResult::Completed {
                output: output(doc, &work, created),
                warnings: Vec::new(),
            }
        }
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: OptionsEdit) -> CommandResult<OptionsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_OPTIONS_EDIT,
        kentos_commands::CAD_OPTIONS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: OptionsEdit) -> CommandResult<OptionsEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let created: Vec<String> = work.adds.iter().map(|(u, _)| u.to_string()).collect();
    // The sets first: the step's followers (openings, spaces, schedules) read the options as they are after it.
    let result = doc.transact(STEP, |d| {
        d.set_arch_options(work.sets.clone(), STEP)?;
        if !work.adds.is_empty() {
            d.add_many_with_uids(work.adds.clone(), STEP)?;
        }
        if !work.removes.is_empty() {
            d.remove(&work.removes);
        }
        Ok::<(), kentos_domain::Refusal>(())
    });
    if let Err(e) = result {
        return CommandResult::Failed {
            error: error(codes::INVALID_INPUT, e.0, Some("operation".into())),
        };
    }
    CommandResult::Completed {
        output: output(doc, &work, created),
        warnings: Vec::new(),
    }
}
