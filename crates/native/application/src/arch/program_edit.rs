//! `cad.program.edit` v1 (docs/adr/0773): the building program written — its buildings, wings, units, common and
//! outdoor areas, design brief and fixtures' clearances — and spaces given to its groups, as one undo step “Bina
//! programı”, all or none. A group spaces still name is not taken away unless the same call gives them elsewhere; a
//! building or a wing groups still name is not taken away either (docs/adr/0784 §1), and a building's or a wing's levels
//! are the project's. The
//! shared cases are `fixtures/commands/v1/cad.program.edit.json` (`scripts/fixtures/program_cases.py`).

use kentos_commands::{CommandResult, Entity, ProgramEdit, ProgramEdited};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::spaces::fail;

pub const STEP: &str = "Bina programı";

struct Work {
    program: kentos_commands::BuildingProgram,
    units: Vec<(Slot, Option<String>)>,
}

fn check(doc: &Document, input: &ProgramEdit) -> Result<Work, Stop> {
    if input.program.is_none() && input.assign.is_empty() {
        return Err(fail(
            codes::NOTHING_TO_SET,
            "Değiştirilecek bir şey verilmedi: programı ya da bölümlere verilecek mahalleri verin.".into(),
            "",
        ));
    }
    let program = input.program.clone().unwrap_or_else(|| doc.program());
    // A building or a wing taken away that groups still name (docs/adr/0784 §1): the groups said by their numbers.
    let before = doc.program();
    let kept: Vec<&str> =
        program.buildings.iter().flat_map(|b| std::iter::once(b.id.as_str()).chain(b.wings.iter().map(|w| w.id.as_str()))).collect();
    for b in &before.buildings {
        let gone = std::iter::once((b.id.as_str(), format!("“{}” binası", b.name)))
            .chain(b.wings.iter().map(|w| (w.id.as_str(), format!("“{}” kanadı", w.name))));
        for (id, what) in gone {
            if kept.contains(&id) {
                continue;
            }
            let users: Vec<String> = program.users_of(id).iter().map(|u| format!("“{}”", u.number)).collect();
            if !users.is_empty() {
                return Err(fail(
                    codes::BUILDING_IN_USE,
                    format!(
                        "{what} kaldırılamaz: {} bölümü ona bağlı; önce bölümleri başka binaya ya da kanada verin.",
                        users.join(", ")
                    ),
                    "program.buildings",
                ));
            }
        }
    }
    if let Some((at, why)) = program.problem() {
        return Err(fail(codes::INVALID_PROGRAM, why, &format!("program.{at}")));
    }
    // A building's or a wing's levels are the project's (docs/adr/0784 §1).
    let levels: Vec<String> =
        doc.settings().arch.as_ref().map(|a| a.levels.iter().map(|l| l.id.clone()).collect()).unwrap_or_default();
    for (i, b) in program.buildings.iter().enumerate() {
        let lists = std::iter::once((format!("buildings[{i}]"), &b.levels))
            .chain(b.wings.iter().enumerate().map(|(k, w)| (format!("buildings[{i}].wings[{k}]"), &w.levels)));
        for (at, list) in lists {
            if let Some((k, l)) = list.iter().enumerate().find(|(_, l)| !levels.contains(l)) {
                return Err(fail(
                    codes::LEVEL_NOT_FOUND,
                    format!("“{l}” katı projede yok; önce Proje ayarları › Mimari'de katı ekleyin."),
                    &format!("program.{at}.levels[{k}]"),
                ));
            }
        }
    }
    let mut units: Vec<(Slot, Option<String>)> = Vec::new();
    for (i, a) in input.assign.iter().enumerate() {
        let under = format!("assign[{i}].");
        checks::uids(&a.uids, "Bölüme verilecek mahal verilmedi.").map_err(|s| prefixed(s, &under))?;
        let found = checks::named(doc, &a.uids).map_err(|s| prefixed(s, &under))?;
        for (k, slot, e, _) in found {
            if !matches!(e, Entity::Space(_)) {
                return Err(fail(
                    codes::NOT_A_SPACE,
                    "Nesne mahal değil; yalnız mahaller bölüme verilir.".into(),
                    &format!("assign[{i}].uids[{k}]"),
                ));
            }
            units.retain(|(s, _)| *s != slot);
            units.push((slot, a.unit.clone()));
        }
        if let Some(u) = &a.unit
            && program.unit(u).is_none()
        {
            return Err(fail(
                codes::UNIT_NOT_FOUND,
                format!("“{u}” bölümü bina programında yok; önce programa ekleyin."),
                &format!("assign[{i}].unit"),
            ));
        }
    }
    // A group taken away that spaces still name, after this call's assignments.
    let mut stranded: Vec<String> = Vec::new();
    for e in doc.entities() {
        let Entity::Space(s) = e else { continue };
        let slot = Slot(s.base.id);
        let unit = match units.iter().find(|(x, _)| *x == slot) {
            Some((_, u)) => u.clone(),
            None => s.space.unit.clone(),
        };
        if let Some(u) = unit
            && before.unit(&u).is_some()
            && program.unit(&u).is_none()
        {
            stranded.push(doc.uid(slot).map(|u| u.to_string()).unwrap_or_default());
        }
    }
    if !stranded.is_empty() {
        let mut e = error(
            codes::UNIT_IN_USE,
            format!(
                "Kaldırılan bölümde {} mahal kalıyor; önce onları başka bölüme verin ya da bölümden çıkarın.",
                stranded.len()
            ),
            Some("program.units".into()),
        );
        e.uids = stranded;
        return Err(Stop::failed(e));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    for (slot, _) in &units {
        let Some(e) = doc.get(*slot) else { continue };
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc.layers().get(layer).map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!("“{name}” katmanı kilitli; üzerindeki mahal bölüme verilemez. Kilidi Katmanlar panelinden açın."),
                "assign",
            ));
        }
    }
    Ok(Work { program, units })
}

/// A refusal at a path under `pre` (an assignment's).
fn prefixed(stop: Stop, pre: &str) -> Stop {
    match stop {
        Stop::Failed(mut e) => {
            e.path = Some(format!("{pre}{}", e.path.unwrap_or_default()));
            Stop::Failed(e)
        }
        other => other,
    }
}

fn assigned(doc: &Document, work: &Work) -> Vec<String> {
    let mut out: Vec<(Slot, String)> = work
        .units
        .iter()
        .filter(|(slot, unit)| matches!(doc.get(*slot), Some(Entity::Space(s)) if s.space.unit != *unit))
        .filter_map(|(slot, _)| doc.uid(*slot).map(|u| (*slot, u.to_string())))
        .collect();
    out.sort_by_key(|(s, _)| s.0);
    out.into_iter().map(|(_, u)| u).collect()
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ProgramEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ProgramEdit) -> CommandResult<ProgramEdited> {
    match check(cx.doc, input) {
        Ok(work) => CommandResult::Completed {
            output: ProgramEdited {
                assigned: assigned(cx.doc, &work),
                program: work.program,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ProgramEdit) -> CommandResult<ProgramEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_PROGRAM_EDIT,
        kentos_commands::CAD_PROGRAM_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ProgramEdit) -> CommandResult<ProgramEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let changed = assigned(cx.doc, &work);
    if let Err(e) = cx.doc.set_program(work.program.clone(), work.units, STEP) {
        return CommandResult::Failed { error: error(codes::INVALID_PROGRAM, e.0, Some("program".into())) };
    }
    CommandResult::Completed {
        output: ProgramEdited { program: work.program, assigned: changed, revision: cx.doc.revision().to_string() },
        warnings: Vec::new(),
    }
}
