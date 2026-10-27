//! `cad.levels.assign` v1 (“Kat ver”, docs/adr/0717): architectural objects given one of the project's levels as one
//! undo step, where they stand (Archicad's Relink Home Story). Without objects named, every one Denetim says lacks a
//! level. A room's boundary is found again among its new level's walls by the document's follower in the same step;
//! a room left unenclosed, by its own new level or by its walls leaving its level, is said (`arch_space_broken`). The
//! shared cases are `fixtures/commands/v1/cad.levels.assign.json`.

use std::collections::HashMap;

use kentos_commands::{
    ArchLevel, CommandResult, CommandWarning, Entity, LevelAssigned, LevelsAssign, LevelsAssigned,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const STEP: &str = "Kat ver";

/// Whether `e` has a level of its own (docs/adr/0715 §1): a wall, a space, a grid, a column, a stair, a ramp or a
/// room separator.
pub fn levelled(e: &Entity) -> bool {
    match e {
        Entity::Wall(_)
        | Entity::Space(_)
        | Entity::Grid(_)
        | Entity::Column(_)
        | Entity::Stair(_)
        | Entity::Ramp(_) => true,
        Entity::Polyline(p) => p.space_separator.is_some(),
        _ => false,
    }
}

/// What Denetim says lacks a level (`arch_level_missing`) while the project has `levels`: a wall, a space, a column,
/// a stair, a ramp or a room separator on no level or on a level the project does not have. A grid may stand on none
/// (it serves every level).
pub fn lacks_level(e: &Entity, levels: &[ArchLevel]) -> bool {
    !levels.is_empty()
        && levelled(e)
        && !matches!(e, Entity::Grid(_))
        && e.own_level().is_none_or(|id| !levels.iter().any(|l| l.id == id))
}

/// `e`'s own level, to be written.
fn level_mut(e: &mut Entity) -> Option<&mut Option<String>> {
    match e {
        Entity::Wall(x) => Some(&mut x.wall.arch.level),
        Entity::Space(x) => Some(&mut x.space.arch.level),
        Entity::Grid(x) => Some(&mut x.grid.arch.level),
        Entity::Column(x) => Some(&mut x.column.arch.level),
        Entity::Stair(x) => Some(&mut x.stair.arch.level),
        Entity::Ramp(x) => Some(&mut x.ramp.arch.level),
        Entity::Polyline(p) => p.space_separator.as_mut().map(|a| &mut a.level),
        _ => None,
    }
}

/// Why `e` takes no level: whose level it is on.
fn not_levelled(e: &Entity) -> String {
    match e {
        Entity::Opening(_) => {
            "Kapı, pencere ya da boşluk duvarının katındadır; kendi katı yok. Duvarına kat verin.".into()
        }
        Entity::Tag(_) => "Etiket etiketlediği nesnenin katındadır; kendi katı yok. O nesneye kat verin.".into(),
        Entity::Polyline(_) => {
            "Çoklu çizgi yalnız mahal ayırıcıyken kat alır; bu çizgi mahal ayırıcı değil.".into()
        }
        _ => format!(
            "“{}” nesnesinin kendi katı yok; kat yalnız duvar, mahal, aks, kolon, merdiven, rampa ve mahal ayırıcıya verilir.",
            e.kind()
        ),
    }
}

/// What is written: each object with the level, as the answer lists it.
struct Work {
    changes: Vec<(Slot, Entity)>,
    assigned: Vec<LevelAssigned>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &LevelsAssign) -> Result<Work, Stop> {
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.as_slice())
        .unwrap_or_default();
    // The objects: those named (their place in `uids`), or every one that lacks a level.
    let candidates: Vec<(Option<usize>, Slot, &Entity, String)> = match &input.uids {
        Some(uids) => {
            checks::uids(uids, "Kat verilecek nesne verilmedi.")?;
            let found = checks::named(doc, uids)?;
            if let Some((at, _, e, _)) = found.iter().find(|(_, _, e, _)| !levelled(e)) {
                return Err(Stop::failed(error(
                    codes::NOT_LEVELLED,
                    not_levelled(e),
                    Some(format!("uids[{at}]")),
                )));
            }
            found
                .into_iter()
                .map(|(at, slot, e, uid)| (Some(at), slot, e, uid.clone()))
                .collect()
        }
        None => doc
            .entities_with_uids()
            .filter(|(_, e)| lacks_level(e, levels))
            .map(|(uid, e)| (None, Slot(e.base().id), e, uid.to_string()))
            .collect(),
    };
    if let Some(why) = kentos_commands::level_problem(Some(&input.level), levels) {
        return Err(Stop::failed(error(
            codes::LEVEL_NOT_FOUND,
            why,
            Some("level".into()),
        )));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    // Those on a locked layer are left out and counted; all of them locked is refused (docs/adr/0278).
    let layers = doc.layers();
    let (locked, open): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|(_, _, e, _)| layers.is_locked(&e.base().layer_id));
    if open.is_empty()
        && let Some((at, _, e, _)) = locked.first()
    {
        let layer = &e.base().layer_id;
        let name = layers.get(layer).map_or(layer.as_str(), |n| n.name.as_str());
        return Err(Stop::failed(error(
            codes::LAYER_LOCKED,
            format!("“{name}” katmanı kilitli; üzerindeki nesneye kat verilemez. Kilidi Katmanlar panelinden açın."),
            at.map(|i| format!("uids[{i}]")),
        )));
    }
    let mut warnings = Vec::new();
    if !locked.is_empty() {
        warnings.push(CommandWarning {
            code: codes::LAYER_LOCKED.into(),
            message: format!(
                "{} nesne kilitli katmanda olduğu için atlandı. Kat vermek için katmanın kilidini Katmanlar panelinden açın.",
                locked.len()
            ),
            path: input.uids.as_ref().map(|_| "uids".into()),
        });
    }
    let mut changes = Vec::new();
    let mut assigned = Vec::new();
    for (_, slot, e, uid) in open {
        if e.own_level() == Some(input.level.as_str()) {
            continue;
        }
        let mut new = e.clone();
        if let Some(level) = level_mut(&mut new) {
            *level = Some(input.level.clone());
        }
        assigned.push(LevelAssigned {
            uid,
            id: slot.0,
            kind: e.kind().into(),
            from: e.own_level().map(str::to_owned),
        });
        changes.push((slot, new));
    }
    if changes.is_empty() {
        return Err(Stop::failed(error(
            codes::NOTHING_TO_ASSIGN,
            match &input.uids {
                Some(_) => "Verilen nesneler zaten bu katta; değişecek nesne yok.".into(),
                None => "Çizimde katsız ya da projede olmayan bir kattaki mimari nesne yok; değişecek nesne yok.".into(),
            },
            input.uids.as_ref().map(|_| "uids".into()),
        )));
    }
    Ok(Work {
        changes,
        assigned,
        warnings,
    })
}

/// Whether each space's boundary is broken.
fn broken_spaces(doc: &Document) -> HashMap<Slot, bool> {
    doc.entities()
        .filter_map(|e| match e {
            Entity::Space(s) => Some((Slot(e.base().id), s.space.broken.is_some())),
            _ => None,
        })
        .collect()
}

/// `arch_space_broken` for the rooms the step left unenclosed (none when every room still closes).
fn space_warning(before: &HashMap<Slot, bool>, after: &Document) -> Option<CommandWarning> {
    let lost = broken_spaces(after)
        .into_iter()
        .filter(|(slot, broken)| *broken && !before.get(slot).copied().unwrap_or(false))
        .count();
    (lost > 0).then(|| CommandWarning {
        code: codes::ARCH_SPACE_BROKEN.into(),
        message: format!(
            "{lost} mahalın sınırı yeni katında duvarlarla kapanmıyor; son sınırıyla kaldı. Duvarlarını da aynı kata verin."
        ),
        path: None,
    })
}

/// The work and every warning the step gives: a room the follower cannot close is found on a reading copy of the
/// drawing (without its history, so the followers run as the edit is made: a command's own step runs them only as it
/// ends), the same before writing and while writing.
fn prepared(doc: &Document, input: &LevelsAssign) -> Result<(Work, Vec<CommandWarning>), Stop> {
    let work = check(doc, input)?;
    let before = broken_spaces(doc);
    let mut copy = doc.reading_copy();
    copy.update_many(work.changes.clone(), STEP);
    let mut warnings = work.warnings.clone();
    warnings.extend(space_warning(&before, &copy));
    Ok((work, warnings))
}

fn answer(input: &LevelsAssign, work: &Work, doc: &Document) -> LevelsAssigned {
    LevelsAssigned {
        level: input.level.clone(),
        assigned: work.assigned.clone(),
        revision: doc.revision().to_string(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &LevelsAssign) -> CommandResult<()> {
    match prepared(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &LevelsAssign) -> CommandResult<LevelsAssigned> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what Denetim's Kata ver shows before writing.
pub fn plan_in(doc: &Document, input: &LevelsAssign) -> CommandResult<LevelsAssigned> {
    match prepared(doc, input) {
        Ok((work, warnings)) => CommandResult::Completed {
            output: answer(input, &work, doc),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: LevelsAssign) -> CommandResult<LevelsAssigned> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LEVELS_ASSIGN,
        kentos_commands::CAD_LEVELS_ASSIGN_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: LevelsAssign) -> CommandResult<LevelsAssigned> {
    let (work, warnings) = match prepared(cx.doc, &input) {
        Ok(prepared) => prepared,
        Err(stop) => return stop.into(),
    };
    cx.doc.update_many(work.changes.clone(), STEP);
    CommandResult::Completed {
        output: answer(&input, &work, cx.doc),
        warnings,
    }
}
