//! Kitchen runs in room modules (docs/adr/8050 §5): a module keeps a run as its recipe about its base, never as its
//! objects; each instance lays the recipe again with its matrix among its own walls in the module's undo step, so its
//! run is an ordinary run of the drawing (it follows its walls, Mutfak dizisini düzenle edits it) that the module
//! tracks by key. A mirror turns the run's side and its single doors' hands over: the copy is the run's mirror image laid
//! by the run's own rule (the sink 5 cm from its module's start, the modules numbered from `from`), not its objects
//! mirrored. A child of `modules_edit`, sharing its work and its helpers.

use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_commands::{
    ArchModule, CaseworkDoor, CaseworkModuleSetting, CaseworkRun, CaseworkSide, ModuleInstance,
    ModuleObject, ModuleRun, Vec2,
};
use kentos_domain::{Document, Refusal, Uuid};
use kentos_geometry_core::geom::affine::{Affine, apply};

use super::{STEP, inverse, matrix};
use crate::casework_run::{fresh_id, library};
use crate::checks::Stop;
use crate::codes;
use crate::openings::fail;

/// A run an instance lays: the module's and the instance's ids, the run's key, its recipe in the drawing and the
/// instance's run laid again (none: a new one).
#[derive(Clone, Debug)]
pub(super) struct Lay {
    pub module: String,
    pub instance: String,
    pub key: String,
    pub recipe: CaseworkRun,
    pub record: Option<String>,
}

fn flip_door(d: CaseworkDoor) -> CaseworkDoor {
    match d {
        CaseworkDoor::Left => CaseworkDoor::Right,
        CaseworkDoor::Right => CaseworkDoor::Left,
    }
}

/// `r` moved by `m`; `mirrored`, its side turned over and each single door's hand (the laid modules say which are
/// single, a default left one written right).
pub(super) fn placed_recipe(
    doc: &Document,
    r: &CaseworkRun,
    m: &Affine,
    mirrored: bool,
) -> CaseworkRun {
    let path: Vec<Vec2> = r
        .path()
        .into_iter()
        .map(|v| {
            let q = apply(m, kentos_geometry_core::Vec2::new(v.x, v.y));
            Vec2 { x: q.x, y: q.y }
        })
        .collect();
    let mut out = r.along(&path);
    out.expected_revision = None;
    if !mirrored {
        return out;
    }
    out.side = Some(match r.side.unwrap_or_default() {
        CaseworkSide::Left => CaseworkSide::Right,
        CaseworkSide::Right => CaseworkSide::Left,
    });
    let mut own: BTreeMap<u32, CaseworkModuleSetting> = r
        .per_module
        .iter()
        .flatten()
        .map(|s| (s.module, s.clone()))
        .collect();
    let mut fresh = fresh_id;
    if let Ok(laid) = doc.lay_casework(r, &library, &mut fresh) {
        for module in &laid.modules {
            let hand = match module.hand {
                Some(kentos_commands::CaseworkHand::Left) => CaseworkDoor::Right,
                Some(kentos_commands::CaseworkHand::Right) => CaseworkDoor::Left,
                _ => continue,
            };
            own.entry(module.number)
                .or_insert(CaseworkModuleSetting {
                    module: module.number,
                    width: None,
                    hand: None,
                })
                .hand = Some(hand);
        }
    } else {
        // Not laid (it will not be laid here either): its own doors turned over at least.
        for s in own.values_mut() {
            s.hand = s.hand.map(flip_door);
        }
    }
    let list: Vec<CaseworkModuleSetting> = own.into_values().collect();
    out.per_module = (!list.is_empty()).then_some(list);
    out
}

/// `r` with what is its default left out (a left side, a left door, no corners, no oven, no wall cabinets) and its
/// modules' own values in order: two recipes that lay the same run compare the same.
pub(super) fn normalized(r: &CaseworkRun) -> CaseworkRun {
    let mut out = r.clone();
    out.expected_revision = None;
    if out.side == Some(CaseworkSide::Left) {
        out.side = None;
    }
    if out.through.as_ref().is_some_and(Vec::is_empty) {
        out.through = None;
    }
    if out.oven == Some(false) {
        out.oven = None;
    }
    if out.upper == Some(false) {
        out.upper = None;
    }
    let mut list: Vec<CaseworkModuleSetting> = out
        .per_module
        .take()
        .unwrap_or_default()
        .into_iter()
        .map(|mut s| {
            if s.hand == Some(CaseworkDoor::Left) {
                s.hand = None;
            }
            s
        })
        .filter(|s| s.width.is_some() || s.hand.is_some())
        .collect();
    list.sort_by_key(|s| s.module);
    out.per_module = (!list.is_empty()).then_some(list);
    out
}

/// Whether two recipes lay the same run, their points within `near`'s tolerance.
pub(super) fn same_recipe(a: &CaseworkRun, b: &CaseworkRun) -> bool {
    let (Ok(x), Ok(y)) = (
        serde_json::to_value(normalized(a)),
        serde_json::to_value(normalized(b)),
    ) else {
        return false;
    };
    super::more::near(&x, &y)
}

/// The run `id` names in the drawing.
pub(super) fn record_of<'a>(
    doc: &'a Document,
    id: &str,
) -> Option<&'a kentos_commands::CaseworkRecord> {
    let id = kentos_commands::CaseworkId::parse(id)?;
    doc.casework().iter().find(|r| r.id == id).map(|r| &**r)
}

/// The definition's recipe of the run `id` of `inst`: taken back by the instance's matrix, as the definition keeps it.
pub(super) fn definition_recipe(
    doc: &Document,
    inst: &ModuleInstance,
    id: &str,
) -> Option<CaseworkRun> {
    let r = record_of(doc, id)?;
    let back = inverse(inst.at, inst.rotation, inst.mirror);
    Some(normalized(&placed_recipe(
        doc,
        &r.recipe,
        &back,
        inst.mirror,
    )))
}

/// The runs a selection takes whole (define, add members): each run with an object in it, by its id; refused when a
/// run's objects are not all in it or a wall it stands on is neither in it nor in `also` (`module_outside`, at the
/// first such object). `found` is the selection's objects (their input index and id).
pub(super) fn selected_runs(
    doc: &Document,
    found: &[(usize, String)],
    also: &HashSet<&str>,
) -> Result<Vec<kentos_commands::CaseworkRecord>, Stop> {
    let set: HashSet<&str> = found.iter().map(|(_, u)| u.as_str()).collect();
    let mut out: Vec<kentos_commands::CaseworkRecord> = Vec::new();
    for (i, uid) in found {
        let Some((run, _)) = Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.casework_member(u))
        else {
            continue;
        };
        if out.iter().any(|r| r.id == run.id) {
            continue;
        }
        let path = format!("operation.uids[{i}]");
        if run
            .members
            .iter()
            .any(|m| !set.contains(Uuid::from_bytes(m.uid.0).to_string().as_str()))
        {
            return Err(fail(
                codes::MODULE_OUTSIDE,
                "Mutfak dizisinin bir kısmı seçili: dizinin bütün nesnelerini (dolaplar, tezgâh, dolgular) seçin ya da hiçbirini.".into(),
                &path,
            ));
        }
        if run.ties.iter().flatten().any(|t| {
            let w = Uuid::from_bytes(t.wall.0).to_string();
            !set.contains(w.as_str()) && !also.contains(w.as_str())
        }) {
            return Err(fail(
                codes::MODULE_OUTSIDE,
                "Mutfak dizisi seçimin dışındaki bir duvara dayanıyor; o duvarı da seçin ya da diziyi seçimden çıkarın.".into(),
                &path,
            ));
        }
        out.push(run.clone());
    }
    Ok(out)
}

/// The ids of the objects of `runs`.
pub(super) fn run_objects(runs: &[kentos_commands::CaseworkRecord]) -> HashSet<String> {
    runs.iter()
        .flat_map(|r| {
            r.members
                .iter()
                .map(|m| Uuid::from_bytes(m.uid.0).to_string())
        })
        .collect()
}

/// The level a run stands on: its first object's own (its modules take their wall's, docs/adr/0733 §1).
pub(super) fn run_level(doc: &Document, run: &kentos_commands::CaseworkRecord) -> Option<String> {
    run.members.iter().find_map(|m| {
        let slot = doc.slot_of(Uuid::from_bytes(m.uid.0))?;
        doc.get(slot)?.own_level().map(str::to_owned)
    })
}

/// Whether an instance holds the run (a single-storey instance its storey's only, docs/adr/0811).
pub(super) fn holds(inst: &ModuleInstance, run: &ModuleRun) -> bool {
    inst.storey.is_none_or(|s| run.storey == s)
}

/// What `inst` lays for the runs of `m` named by `keys` (none: every run it holds): its own run laid again where it has
/// one, a new one where not.
pub(super) fn lays_of(
    doc: &Document,
    m: &ArchModule,
    inst: &ModuleInstance,
    keys: Option<&[String]>,
) -> Vec<Lay> {
    let mx = matrix(inst.at, inst.rotation, inst.mirror);
    m.runs
        .iter()
        .filter(|r| holds(inst, r))
        .filter(|r| keys.is_none_or(|k| k.contains(&r.key)))
        .map(|r| Lay {
            module: m.id.clone(),
            instance: inst.id.clone(),
            key: r.key.clone(),
            recipe: placed_recipe(doc, &r.recipe, &mx, inst.mirror),
            record: inst
                .runs
                .iter()
                .find(|o| o.key == r.key)
                .map(|o| o.uid.clone()),
        })
        .collect()
}

/// The objects the lays write, as the drawing has nothing yet: for a preview.
pub(super) fn laid_objects(doc: &Document, lays: &[Lay]) -> Vec<kentos_commands::Entity> {
    let mut out = Vec::new();
    for lay in lays {
        let mut fresh = fresh_id;
        if let Ok(laid) = doc.lay_casework(&lay.recipe, &library, &mut fresh) {
            out.extend(laid.members.into_iter().map(|m| m.entity));
        }
    }
    out
}

/// Writes the lays in the step under way (after the instances' walls): each run laid among the walls as they now are
/// and written as a run of the drawing, its instance's entry pointing at it. The ids of the objects written.
pub(super) fn write_lays(
    d: &mut Document,
    lays: &[Lay],
    modules: &mut [ArchModule],
) -> Result<Vec<String>, Refusal> {
    let mut created = Vec::new();
    for lay in lays {
        let mut fresh = fresh_id;
        let laid = d
            .lay_casework(&lay.recipe, &library, &mut fresh)
            .map_err(|s| Refusal(format!("Mutfak dizisi yerleştirilemedi: {}", s.message)))?;
        let record = lay
            .record
            .as_deref()
            .and_then(|id| record_of(d, id))
            .cloned();
        let (id, written) = d.write_casework(record.as_ref(), lay.recipe.clone(), &laid, STEP)?;
        let before: HashSet<String> = record
            .iter()
            .flat_map(|r| {
                r.members
                    .iter()
                    .map(|m| Uuid::from_bytes(m.uid.0).to_string())
            })
            .collect();
        created.extend(
            written
                .iter()
                .map(|(u, _)| u.to_string())
                .filter(|u| !before.contains(u)),
        );
        let Some(inst) = modules
            .iter_mut()
            .find(|m| m.id == lay.module)
            .and_then(|m| m.instances.iter_mut().find(|i| i.id == lay.instance))
        else {
            continue;
        };
        let uid = id.to_string();
        match inst.runs.iter_mut().find(|o| o.key == lay.key) {
            Some(o) => o.uid = uid,
            None => inst.runs.push(ModuleObject {
                key: lay.key.clone(),
                uid,
            }),
        }
    }
    Ok(created)
}

/// The instance's run an object of the drawing is part of: its module's key, by the instance's runs.
pub fn run_key_of(doc: &Document, inst: &ModuleInstance, uid: &str) -> Option<String> {
    let (run, _) = doc.casework_member(Uuid::parse_str(uid).ok()?)?;
    let id = run.id.to_string();
    inst.runs
        .iter()
        .find(|o| o.uid == id)
        .map(|o| o.key.clone())
}

/// The keys of runs whose instance's recipe is not the definition's placed (changed by hand), for drift.
pub(super) fn drifted_runs(doc: &Document, m: &ArchModule, inst: &ModuleInstance) -> Vec<String> {
    inst.runs
        .iter()
        .filter(|o| !inst.exceptions.contains(&o.key))
        .filter_map(|o| {
            let def = m.run(&o.key)?;
            let now = definition_recipe(doc, inst, &o.uid)?;
            (!same_recipe(&now, &def.recipe)).then(|| o.key.clone())
        })
        .collect()
}

/// The objects of the instance's runs `keys` names.
pub(super) fn objects_of_runs(
    doc: &Document,
    inst: &ModuleInstance,
    keys: &[String],
) -> Vec<String> {
    let ids: HashMap<&str, &str> = inst
        .runs
        .iter()
        .map(|o| (o.key.as_str(), o.uid.as_str()))
        .collect();
    keys.iter()
        .filter_map(|k| ids.get(k.as_str()))
        .filter_map(|id| record_of(doc, id))
        .flat_map(|r| {
            r.members
                .iter()
                .map(|m| Uuid::from_bytes(m.uid.0).to_string())
        })
        .collect()
}
