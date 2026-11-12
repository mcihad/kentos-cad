//! `cad.profiles.carry` v1, Kataloğa taşı (plan/map-cad.md MAP-04, docs/adr/5340): the drawing's data carried from one
//! catalog to another as `cad.profiles.compare`'s report traces each class. A kept, changed, renamed or merged class's
//! objects go to its new class's layer; a split class's objects each to the new class holding its field code (the code
//! field's leading capital letters); a retired class's, and a split one's without a matching code, stay and are said.
//! The receiving classes' layers and missing fields are written as `cad.profiles.apply` writes them. Values travel with
//! their objects. One undo step, all or nothing. The shared cases are `fixtures/commands/v1/cad.profiles.carry.json`.

use kentos_commands::{
    CarriedClass, CatalogClassChangeKind as K, CommandResult, CommandWarning, DataProfile, ProfilesCarried,
    ProfilesCarry, ProfilesCompare,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::profiles::{Place, find};

const STEP: &str = "Kataloğa taşı";

/// What is to be done: per class with objects, its row, and each object's slot and target class (none: it stays).
struct Plan {
    old: DataProfile,
    new: DataProfile,
    classes: Vec<(CarriedClass, Vec<(Slot, Option<String>)>)>,
}

/// An object's field code: the code field's leading capital letters.
fn code_of(e: &kentos_commands::Entity, field: &str) -> Option<String> {
    let v = e.base().attrs.get(field)?.trim();
    let letters: String = v.chars().take_while(char::is_ascii_uppercase).collect();
    (!letters.is_empty()).then_some(letters)
}

fn prepare(doc: &Document, input: &ProfilesCarry) -> Result<Plan, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let compare = ProfilesCompare {
        from: input.from.clone(),
        from_profile: input.from_profile.clone(),
        to: input.to.clone(),
        to_profile: input.to_profile.clone(),
        drawing: input.drawing.clone(),
    };
    let (old, new) = crate::profiles_compare::catalogs(doc, &compare)?;
    let report = crate::profiles_compare::report(doc, &old, &new);
    let field = input.field.clone().unwrap_or_else(|| kentos_commands::DEFAULT_CODE_FIELD.to_owned());
    let layers = doc.layers();
    let mut classes = Vec::new();
    for row in &report.classes {
        let Some(from) = row.from.as_ref() else { continue };
        let Some(c) = old.classes.iter().find(|k| k.id == *from) else { continue };
        let Place::Layer(layer) = find(layers.nodes(), &c.layer) else { continue };
        let objects: Vec<(Slot, &kentos_commands::Entity)> = doc
            .entities_with_uids()
            .filter(|(_, e)| e.base().layer_id == layer)
            .filter_map(|(u, e)| doc.slot_of(u).map(|s| (s, e)))
            .collect();
        if objects.is_empty() {
            continue;
        }
        let mut each = Vec::with_capacity(objects.len());
        for (slot, e) in objects {
            let target = match row.kind {
                K::Retired | K::Added => None,
                K::Split => code_of(e, &field).and_then(|code| {
                    row.to.iter().find(|t| new.classes.iter().any(|k| k.id == **t && k.codes.contains(&code))).cloned()
                }),
                _ => row.to.first().cloned(),
            };
            // An object already on its target's layer does not move.
            let target = target.filter(|t| {
                new.classes.iter().find(|k| k.id == *t).is_none_or(|k| find(layers.nodes(), &k.layer) != Place::Layer(layer.clone()))
            });
            each.push((slot, target));
        }
        let moved = each.iter().filter(|(_, t)| t.is_some()).count();
        let left = if matches!(row.kind, K::Retired | K::Split) { each.len() - moved } else { 0 };
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        classes.push((
            CarriedClass { from: from.clone(), to: row.to.clone(), kind: row.kind, moved: count(moved), left: count(left) },
            each,
        ));
        // A layer the objects leave must be open.
        if moved > 0 && layers.is_locked(&layer) {
            return Err(Stop::failed(error(
                codes::LAYER_LOCKED,
                format!("“{}” katmanı kilitli; nesneleri yeni kataloğa taşınamaz. Kilidi Katmanlar panelinden açın.", c.title),
                None,
            )));
        }
    }
    // A layer they go to must be open too, and its path must lead to a layer (not through one, not to a group).
    for k in &new.classes {
        let receives = classes.iter().any(|(_, each)| each.iter().any(|(_, t)| t.as_deref() == Some(k.id.as_str())));
        if receives && let Place::Blocked(_) = find(layers.nodes(), &k.layer) {
            return Err(Stop::failed(error(
                codes::LAYER_REFUSED,
                format!(
                    "“{}” sınıfının yolu ({}) çizimde bir gruba ya da katmanın içine çıkıyor; nesneler oraya taşınamaz.",
                    k.title,
                    crate::profiles::path_words(&k.layer)
                ),
                None,
            )));
        }
        if receives
            && let Place::Layer(l) = find(layers.nodes(), &k.layer)
            && layers.is_locked(&l)
        {
            return Err(Stop::failed(error(
                codes::LAYER_LOCKED,
                format!("“{}” sınıfının katmanı kilitli; nesneler ona taşınamaz. Kilidi Katmanlar panelinden açın.", k.title),
                None,
            )));
        }
    }
    if classes.iter().all(|(c, _)| c.moved == 0) {
        return Err(Stop::failed(error(
            codes::NOTHING_TO_SET,
            "Taşınacak nesne yok: eski kataloğun sınıflarının nesneleri zaten yeni sınıflarının katmanında ya da yeni \
             katalogda yerleri yok."
                .into(),
            None,
        )));
    }
    Ok(Plan { old, new, classes })
}

/// The words of the objects left: each class's title, why, how many.
fn left_words(p: &Plan) -> Option<String> {
    let said: Vec<String> = p
        .classes
        .iter()
        .filter(|(c, _)| c.left > 0)
        .map(|(c, _)| {
            let title = p.old.classes.iter().find(|k| k.id == c.from).map_or(c.from.as_str(), |k| k.title.as_str());
            let why = if c.kind == K::Retired { "yeni katalogda yok" } else { "kodu yeni sınıflarda yok" };
            format!("{title} ({why}) {}", c.left)
        })
        .collect();
    (!said.is_empty()).then(|| format!("Yerinde kalan nesneler: {}.", said.join(", ")))
}

fn output(p: &Plan, layers: Vec<String>, fields: Vec<String>, revision: String) -> ProfilesCarried {
    ProfilesCarried {
        moved: p.classes.iter().map(|(c, _)| c.moved).sum(),
        left: p.classes.iter().map(|(c, _)| c.left).sum(),
        classes: p.classes.iter().map(|(c, _)| c.clone()).collect(),
        layers,
        fields,
        revision,
    }
}

fn left_warning(p: &Plan) -> Vec<CommandWarning> {
    left_words(p)
        .map(|m| CommandWarning { code: codes::CATALOG_OBJECTS_LEFT.into(), message: m, path: None })
        .into_iter()
        .collect()
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesCarry) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer without writing: no layer made yet.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesCarry) -> CommandResult<ProfilesCarried> {
    match prepare(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: output(&p, Vec::new(), Vec::new(), cx.doc.revision().to_string()),
            warnings: left_warning(&p),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesCarry) -> CommandResult<ProfilesCarried> {
    crate::call::recorded(cx, kentos_commands::CAD_PROFILES_CARRY, kentos_commands::CAD_PROFILES_CARRY_VERSION, input, write)
}

/// The carrying, one recorded step.
fn write(cx: &mut ExecutionContext<'_>, input: ProfilesCarry) -> CommandResult<ProfilesCarried> {
    let p = match prepare(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let mut warnings = Vec::new();
    let (mut made, mut fields) = (Vec::new(), Vec::new());
    let done = cx.doc.transact(STEP, |doc| -> Result<(), Stop> {
        // The receiving classes' layers and their missing fields, in the new catalog's order.
        for k in &p.new.classes {
            let receives = p.classes.iter().any(|(_, each)| each.iter().any(|(_, t)| t.as_deref() == Some(k.id.as_str())));
            if !receives {
                continue;
            }
            let one = crate::map_work_profile::narrowed(&p.new, std::slice::from_ref(&k.id))?;
            let (applied, w) = crate::profiles::write(doc, &one)?;
            made.extend(applied.layers);
            fields.extend(applied.fields);
            warnings.extend(w);
        }
        let mut changes = Vec::new();
        for (_, each) in &p.classes {
            for (slot, target) in each {
                let Some(t) = target else { continue };
                let Some(k) = p.new.classes.iter().find(|k| k.id == *t) else { continue };
                let Place::Layer(layer) = find(doc.layers().nodes(), &k.layer) else { continue };
                let Some(mut e) = doc.get(*slot).cloned() else { continue };
                e.base_mut().layer_id = layer;
                changes.push((*slot, e));
            }
        }
        doc.update_many(changes, STEP);
        Ok(())
    });
    if let Err(stop) = done {
        return stop.into();
    }
    warnings.extend(left_warning(&p));
    CommandResult::Completed { output: output(&p, made, fields, cx.doc.revision().to_string()), warnings }
}
