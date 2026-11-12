//! `road.volumes.history` v1 (plan/road.md Y44, docs/adr/0950; ROAD-VOL-17): an earthwork volume's approvals, oldest
//! first — those that ended as they were approved, with the day and why they ended, and the one that stands — each
//! against the one before it; today's figures, and today against the last approval. Approved quantities are never
//! changed nor removed: a new approval or a revoked one moves to the history. Reads only. The shared cases are
//! `fixtures/commands/v1/road.volumes.history.json`.

use kentos_commands::{CommandResult, CommandWarning, VolumeHistoryEntry, VolumeHistoryRead, VolumeToday, VolumesHistory};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::road_volumes::{self as common, Figures, change};

fn answer(doc: &Document, input: &VolumesHistory) -> Result<(VolumeHistoryRead, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_volumes(), &input.volume, "volume")?;
    Ok((history(doc, def), Vec::new()))
}

/// The history of `def` (saved or as typed).
pub(crate) fn history(doc: &Document, def: &kentos_commands::VolumeDef) -> VolumeHistoryRead {
    let all: Vec<(&kentos_commands::VolumeApprovalDef, bool)> =
        def.history.iter().map(|a| (a, false)).chain(def.approved.iter().map(|a| (a, true))).collect();
    let approvals = all
        .iter()
        .enumerate()
        .map(|(i, (a, standing))| VolumeHistoryEntry {
            approval: (*a).clone(),
            standing: *standing,
            against: (i > 0).then(|| change(&Figures::of_approval(all[i - 1].0), &Figures::of_approval(a))),
        })
        .collect();
    let (today, problem, since) = match common::compute(doc, def) {
        Ok((r, _)) => (
            Some(VolumeToday { cut: r.cut, fill: r.fill, factored_cut: r.factored_cut, factored_fill: r.factored_fill }),
            None,
            all.last().map(|(a, _)| change(&Figures::of_approval(a), &Figures::of_report(&r))),
        ),
        Err(stop) => (None, Some(crate::road_corridors::words(stop)), None),
    };
    VolumeHistoryRead { volume: def.id.clone(), name: def.name.clone(), approvals, today, problem, since }
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &VolumesHistory) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &VolumesHistory) -> CommandResult<VolumeHistoryRead> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: VolumesHistory) -> CommandResult<VolumeHistoryRead> {
    plan(cx, &input)
}

/// The history of a volume as typed (the desktop's Kübaj window).
pub fn preview_of(doc: &Document, def: &kentos_commands::VolumeDef) -> VolumeHistoryRead {
    history(doc, def)
}
