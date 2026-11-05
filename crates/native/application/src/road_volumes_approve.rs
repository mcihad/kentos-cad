//! `road.volumes.approve` v1 (plan/road.md Y8, docs/adr/0906 §5): an earthwork volume approved as it computes now (its
//! totals and each stretch's cut and fill kept, the day and a note), or its approval taken off; one undo step
//! “Kübaj onayı”. An approved volume's definition does not change; its sources may, and the query then says how today's
//! figures differ. An approval that ends — revoked, or replaced by a new one — goes to the volume's history as it was,
//! with the day and why (docs/adr/0950): approved quantities are never changed nor removed. The shared cases are `fixtures/commands/v1/road.volumes.approve.json`.

use kentos_commands::{CommandResult, VolumeApprovalEnd, VolumeDef, VolumesApprove, VolumesWritten};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_sections::judged;
use crate::road_terrains::fail;
use crate::road_volumes as common;

pub(crate) const STEP: &str = "Kübaj onayı";

struct Checked {
    list: Vec<VolumeDef>,
    target: String,
}

fn check(doc: &kentos_domain::Document, input: &VolumesApprove) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_volumes();
    let (at, old) = common::find(own, &input.volume, "volume")?;
    let mut list = own.to_vec();
    if input.revoke {
        if old.approved.is_none() {
            return Err(fail(
                codes::VOLUME_NOT_APPROVED,
                format!("“{}” kübajı onaylı değil; kaldırılacak onay yok.", old.name),
                "volume",
            ));
        }
        // Revoked, the approval goes to the history with the day and why: it is never removed (docs/adr/0950).
        let mut ended = old.approved.clone().expect("checked above");
        ended.ended = Some(VolumeApprovalEnd { at: input.at.clone(), revoked: true, note: input.note.clone() });
        list[at].history.push(ended);
        list[at].approved = None;
    } else {
        let Some(day) = input.at.clone() else {
            return Err(fail(codes::INVALID_VOLUME, "Onay için günü (at, YYYY-AA-GG) verin.".into(), "at"));
        };
        let (now, _) = common::compute(doc, old)?;
        let next = common::approval_of(old, &now, day.clone(), input.note.clone());
        // A new approval of an approved volume takes the old one's place; the old one goes to the history.
        if let Some(mut before) = old.approved.clone() {
            before.ended = Some(VolumeApprovalEnd { at: Some(day), revoked: false, note: None });
            list[at].history.push(before);
        }
        list[at].approved = Some(next);
    }
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.volumes = list.clone();
    judged(&road, codes::INVALID_VOLUME, "Kübaj onayı")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target: old.id.clone() })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<VolumesWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_volumes(c.list.clone(), STEP) {
        return fail(codes::INVALID_VOLUME, format!("Kübaj onayı kurallarına uymuyor: {}.", r.0), "").into();
    }
    let volume = c.list.iter().find(|v| v.id == c.target).map(|d| common::report(doc, d));
    CommandResult::Completed {
        output: VolumesWritten { volume, volumes: common::listed(&c.list), revision: doc.revision().to_string() },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &VolumesApprove) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &VolumesApprove) -> CommandResult<VolumesWritten> {
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

/// Approves the volume, or takes its approval off: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: VolumesApprove) -> CommandResult<VolumesWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_VOLUMES_APPROVE,
        kentos_commands::ROAD_VOLUMES_APPROVE_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}
