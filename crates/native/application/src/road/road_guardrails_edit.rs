//! `road.guardrails.edit` v1 (plan/road.md Y11.1, docs/adr/0909 §1): a guardrail written (new, or the given fields
//! replacing its), rebound to its corridor's present revision, or removed; one undo step “Otokorkuluk”. The shared
//! cases are `fixtures/commands/v1/road.guardrails.edit.json`.

use kentos_commands::{CommandResult, GuardrailDef, GuardrailsEdit, GuardrailsWritten, RoadEditOperation};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_corridors as corridors;
use crate::road_guardrails as common;
use crate::road_sections::judged;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Otokorkuluk";

struct Checked {
    list: Vec<GuardrailDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &GuardrailsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_guardrails();
    let mut list = own.to_vec();
    let found = input.guardrail.as_deref().and_then(|t| common::find(own, t, "guardrail").ok());
    if input.operation != RoadEditOperation::Set && found.is_none() {
        let name = input.guardrail.clone().unwrap_or_default();
        return Err(common::find(own, &name, "guardrail").err().unwrap_or(Stop::Cancelled));
    }
    let corridor = match input.corridor.as_deref() {
        Some(c) => Some(corridors::find(doc.road_corridors(), c, "corridor")?.1.clone()),
        None => None,
    };
    let target = match (input.operation, found) {
        (RoadEditOperation::Remove, Some((at, _))) => {
            list.remove(at);
            None
        }
        (RoadEditOperation::Rebind, Some((at, old))) => {
            let now = doc.road_corridors().iter().find(|c| c.id == old.corridor).map_or(0, |c| c.revision);
            let mut d = old.clone();
            d.corridor_revision = now;
            if d != *old {
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, Some((at, old))) => {
            if input.name.is_none()
                && corridor.is_none()
                && input.rule.is_none()
                && input.step.is_none()
                && input.offset.is_none()
            {
                return Err(fail(
                    codes::INVALID_GUARDRAIL,
                    "Değiştirilecek bir şey yok: name, corridor, rule, step ya da offset verin.".into(),
                    "",
                ));
            }
            let mut d = old.clone();
            if let Some(n) = &input.name {
                d.name = n.clone();
            }
            if let Some(c) = &corridor {
                d.corridor = c.id.clone();
                d.corridor_revision = c.revision;
            }
            if let Some(r) = &input.rule {
                d.rule = r.clone();
            }
            if let Some(s) = input.step {
                d.step = s;
            }
            if let Some(o) = input.offset {
                d.offset = o;
            }
            if d != *old {
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, None) => {
            let missing = [
                ("name", input.name.is_none()),
                ("corridor", corridor.is_none()),
                ("rule", input.rule.is_none()),
                ("step", input.step.is_none()),
            ]
            .into_iter()
            .find(|(_, m)| *m);
            let (Some(name), Some(c), Some(rule), Some(step), None) = (&input.name, &corridor, &input.rule, input.step, missing)
            else {
                return Err(fail(
                    codes::INVALID_GUARDRAIL,
                    "Yeni otokorkuluk için name, corridor, rule ve step verilmeli.".into(),
                    missing.map_or("", |(f, _)| f),
                ));
            };
            let id = input.guardrail.clone().unwrap_or_else(|| {
                (1..).map(|n| format!("o-{n}")).find(|id| own.iter().all(|g| &g.id != id)).unwrap_or_default()
            });
            list.push(GuardrailDef {
                id: id.clone(),
                name: name.clone(),
                corridor: c.id.clone(),
                corridor_revision: c.revision,
                revision: 1,
                rule: rule.clone(),
                step,
                offset: input.offset.unwrap_or(0.0),
            });
            Some(id)
        }
        _ => return Err(Stop::Cancelled),
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.guardrails = list.clone();
    judged(&road, codes::INVALID_GUARDRAIL, "Otokorkuluk")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|g| &g.id == id)) {
        common::read(doc, d)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<GuardrailsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_guardrails(c.list.clone(), STEP) {
        return fail(codes::INVALID_GUARDRAIL, format!("Otokorkuluk kurallarına uymuyor: {}.", r.0), "").into();
    }
    let guardrail = match c.target.as_ref().and_then(|id| c.list.iter().find(|g| &g.id == id)) {
        Some(d) => match common::read(doc, d) {
            Ok(r) => Some(r.report),
            Err(stop) => return stop.into(),
        },
        None => None,
    };
    CommandResult::Completed {
        output: GuardrailsWritten { guardrail, guardrails: common::listed(&c.list), revision: doc.revision().to_string() },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &GuardrailsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &GuardrailsEdit) -> CommandResult<GuardrailsWritten> {
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

/// Writes the guardrail: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: GuardrailsEdit) -> CommandResult<GuardrailsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_GUARDRAILS_EDIT,
        kentos_commands::ROAD_GUARDRAILS_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}

/// The guardrail `input` would write, read, for a window (no step written): its report and its warnings, or why not.
pub fn preview(
    doc: &kentos_domain::Document,
    input: &GuardrailsEdit,
) -> Result<(kentos_commands::GuardrailReport, Vec<kentos_commands::CommandWarning>), String> {
    let c = check(doc, input).map_err(crate::road_corridors::words)?;
    let mut copy = doc.reading_copy();
    copy.set_road_guardrails(c.list.clone(), STEP).map_err(|r| r.0)?;
    let def = c.target.as_ref().and_then(|id| c.list.iter().find(|g| &g.id == id)).ok_or("Otokorkuluk yok.")?;
    common::read(&copy, def).map(|r| (r.report, r.warnings)).map_err(crate::road_corridors::words)
}
