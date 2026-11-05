//! `road.hauls.approve` v1 (plan/road.md Y48, docs/adr/0954; ROAD-HAUL-08): a mass haul diagram's plan approved as
//! planned now — its objective, moment, volume moved, cost, each site's capacity and use, and what it was made from (the
//! diagram's and its volume's revisions, the volume's approval) — or its approval taken off. An approval that ends goes
//! to the diagram's history as it was, with the day and why. One undo step “Taşıma planı onayı”. A plan read later
//! against the approval says what changed (`road.hauls.plan`). The shared cases are
//! `fixtures/commands/v1/road.hauls.approve.json`.

use kentos_commands::{
    CommandResult, HaulApprovalDef, HaulApprovedSite, HaulDef, HaulObjective, HaulsApprove, HaulsPlan, HaulsWritten,
    VolumeApprovalEnd,
};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_hauls as common;
use crate::road_sections::judged;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Taşıma planı onayı";

struct Checked {
    list: Vec<HaulDef>,
    target: String,
}

fn check(doc: &kentos_domain::Document, input: &HaulsApprove) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_hauls();
    let (at, old) = common::find(own, &input.haul, "haul")?;
    let mut list = own.to_vec();
    if input.revoke {
        let Some(mut ended) = old.approved.clone() else {
            return Err(fail(
                codes::HAUL_NOT_APPROVED,
                format!("“{}” Brüknerinin onaylı taşıma planı yok; kaldırılacak onay yok.", old.name),
                "haul",
            ));
        };
        ended.ended = Some(VolumeApprovalEnd { at: input.at.clone(), revoked: true, note: input.note.clone() });
        list[at].history.push(ended);
        list[at].approved = None;
    } else {
        let Some(day) = input.at.clone() else {
            return Err(fail(codes::INVALID_HAUL, "Onay için günü (at, YYYY-AA-GG) verin.".into(), "at"));
        };
        let objective = input.objective.unwrap_or(HaulObjective::Moment);
        let (plan, _) = crate::road_hauls_plan::answer(doc, &HaulsPlan { haul: old.id.clone(), objective: Some(objective) })?;
        if !plan.feasible {
            return Err(fail(
                codes::INVALID_HAUL,
                format!(
                    "“{}” taşıma planı her şeyi yerleştiremiyor (karşılanmayan dolgu {:.3} m³, yerleşmeyen yarma {:.3} m³); kurulamayan plan onaylanmaz.",
                    old.name, plan.shortage, plan.surplus
                ),
                "haul",
            ));
        }
        let volume = doc.road_volumes().iter().find(|v| v.id == old.volume);
        let last = old.history.iter().chain(&old.approved).map(|a| a.number).max().unwrap_or(0);
        let next = HaulApprovalDef {
            number: last + 1,
            at: day.clone(),
            note: input.note.clone(),
            revision: old.revision,
            volume_revision: volume.map_or(0, |v| v.revision),
            volume_approval: volume.and_then(|v| v.approved.as_ref()).map_or(0, |a| a.number.max(1)),
            cost: objective == HaulObjective::Cost,
            moment: plan.moment,
            moved: plan.moved,
            total: plan.cost.as_ref().map(|c| c.total),
            sites: plan.sites.iter().map(|s| HaulApprovedSite { name: s.name.clone(), capacity: s.capacity, used: s.used }).collect(),
            ended: None,
        };
        // A new approval of an approved plan takes the old one's place; the old one goes to the history.
        if let Some(mut before) = old.approved.clone() {
            before.ended = Some(VolumeApprovalEnd { at: Some(day), revoked: false, note: None });
            list[at].history.push(before);
        }
        list[at].approved = Some(next);
    }
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.hauls = list.clone();
    judged(&road, codes::INVALID_HAUL, "Taşıma planı onayı")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target: old.id.clone() })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<HaulsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_hauls(c.list.clone(), STEP) {
        return fail(codes::INVALID_HAUL, format!("Taşıma planı onayı kurallarına uymuyor: {}.", r.0), "").into();
    }
    let haul = c.list.iter().find(|h| h.id == c.target).map(|d| common::report(doc, d));
    CommandResult::Completed {
        output: HaulsWritten { haul, hauls: common::listed(&c.list), revision: doc.revision().to_string() },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &HaulsApprove) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &HaulsApprove) -> CommandResult<HaulsWritten> {
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

/// Approves the plan, or takes its approval off: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: HaulsApprove) -> CommandResult<HaulsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_HAULS_APPROVE,
        kentos_commands::ROAD_HAULS_APPROVE_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}
