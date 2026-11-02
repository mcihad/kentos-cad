//! `road.superelevations.edit` v1 (plan/road.md Y6, docs/adr/0904): a superelevation schedule written (new, or the given
//! fields replacing its), rebound to its alignment's present revision, or removed; one undo step “Dever”. The shared
//! cases are `fixtures/commands/v1/road.superelevations.edit.json`.

use kentos_commands::{CommandResult, RoadEditOperation, SuperelevationDef, SuperelevationsEdit, SuperelevationsWritten};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_sections::judged;
use crate::road_superelevations as common;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Dever";

struct Checked {
    list: Vec<SuperelevationDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &SuperelevationsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_superelevations();
    let mut list = own.to_vec();
    let found = input.superelevation.as_deref().and_then(|t| common::find(own, t, "superelevation").ok());
    let alignment = match input.alignment.as_deref() {
        Some(a) => Some(alignments::find(doc.road_alignments(), a, "alignment")?.1.clone()),
        None => None,
    };
    let target = match (input.operation, found) {
        (RoadEditOperation::Remove | RoadEditOperation::Rebind, None) => {
            let name = input.superelevation.clone().unwrap_or_default();
            return Err(common::find(own, &name, "superelevation").err().unwrap_or(Stop::Cancelled));
        }
        (RoadEditOperation::Remove, Some((at, d))) => {
            let users: Vec<&str> = doc
                .road_sections()
                .iter()
                .filter(|s| s.superelevation.as_deref() == Some(&d.id))
                .map(|s| s.name.as_str())
                .collect();
            if !users.is_empty() {
                return Err(fail(
                    codes::SUPERELEVATION_IN_USE,
                    format!("“{}” dever çizelgesini şu kesit takımları kullanıyor: {}. Önce takımlardan kaldırın.", d.name, users.join(", ")),
                    "superelevation",
                ));
            }
            list.remove(at);
            None
        }
        (RoadEditOperation::Rebind, Some((at, old))) => {
            let now = doc.road_alignments().iter().find(|a| a.id == old.alignment).map_or(0, |a| a.revision);
            let mut d = old.clone();
            if d.alignment_revision != now {
                d.alignment_revision = now;
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, Some((at, old))) => {
            if input.name.is_none()
                && alignment.is_none()
                && input.normal_left.is_none()
                && input.normal_right.is_none()
                && input.method.is_none()
            {
                return Err(fail(
                    codes::INVALID_SUPERELEVATION,
                    "Değiştirilecek bir şey yok: name, alignment, normalLeft, normalRight ya da method verin.".into(),
                    "",
                ));
            }
            let mut d = old.clone();
            if let Some(n) = &input.name {
                d.name = n.clone();
            }
            if let Some(a) = &alignment {
                d.alignment = a.id.clone();
                d.alignment_revision = a.revision;
            }
            if let Some(v) = input.normal_left {
                d.normal_left = v;
            }
            if let Some(v) = input.normal_right {
                d.normal_right = v;
            }
            if let Some(m) = &input.method {
                d.method = m.clone();
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
                ("alignment", alignment.is_none()),
                ("normalLeft", input.normal_left.is_none()),
                ("normalRight", input.normal_right.is_none()),
                ("method", input.method.is_none()),
            ]
            .into_iter()
            .find(|(_, m)| *m);
            let (Some(name), Some(a), Some(nl), Some(nr), Some(method), None) =
                (&input.name, alignment, input.normal_left, input.normal_right, &input.method, missing)
            else {
                return Err(fail(
                    codes::INVALID_SUPERELEVATION,
                    "Yeni dever çizelgesi için name, alignment, normalLeft, normalRight ve method verilmeli.".into(),
                    missing.map_or("", |(f, _)| f),
                ));
            };
            let id = input.superelevation.clone().unwrap_or_else(|| {
                (1..).map(|n| format!("d-{n}")).find(|id| own.iter().all(|s| &s.id != id)).unwrap_or_default()
            });
            list.push(SuperelevationDef {
                id: id.clone(),
                name: name.clone(),
                alignment: a.id.clone(),
                alignment_revision: a.revision,
                revision: 1,
                normal_left: nl,
                normal_right: nr,
                method: method.clone(),
            });
            Some(id)
        }
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.superelevations = list.clone();
    judged(&road, codes::INVALID_SUPERELEVATION, "Dever çizelgesi")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|s| &s.id == id)) {
        let built = common::alignment_of(doc, d)?;
        common::schedule(d, &built)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<SuperelevationsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_superelevations(c.list.clone(), STEP) {
        return fail(codes::INVALID_SUPERELEVATION, format!("Dever çizelgesi kurallarına uymuyor: {}.", r.0), "").into();
    }
    let all: Vec<_> = c.list.iter().map(|d| common::report(doc, d)).collect();
    CommandResult::Completed {
        output: SuperelevationsWritten {
            superelevation: c.target.as_ref().and_then(|id| all.iter().find(|s| &s.id == id)).cloned(),
            superelevations: all,
            revision: doc.revision().to_string(),
        },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &SuperelevationsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &SuperelevationsEdit) -> CommandResult<SuperelevationsWritten> {
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

/// Writes the schedule: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SuperelevationsEdit) -> CommandResult<SuperelevationsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_SUPERELEVATIONS_EDIT,
        kentos_commands::ROAD_SUPERELEVATIONS_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}
