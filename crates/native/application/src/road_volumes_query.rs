//! `road.volumes.query` v1 (plan/road.md Y8, docs/adr/0906): every earthwork volume computed now, or one. Reads only.
//! The shared cases are `fixtures/commands/v1/road.volumes.query.json`.

use kentos_commands::{CommandResult, CommandWarning, VolumesQueried, VolumesQuery};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::road_corridors::words;
use crate::road_volumes as common;

fn answer(doc: &kentos_domain::Document, input: &VolumesQuery) -> Result<(VolumesQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = doc.road_volumes();
    let Some(name) = input.volume.as_deref() else {
        let mut warnings = Vec::new();
        let mut volumes = Vec::new();
        for v in own {
            match common::compute(doc, v) {
                Ok((mut r, w)) => {
                    warnings.extend(w);
                    // Docs/adr/1037, 1040: what it measures, how and from what.
                    r.basis = common::basis_of(&r);
                    r.sources = common::sources_of(doc, v);
                    volumes.push(r);
                }
                Err(stop) => volumes.push(common::broken(v, words(stop))),
            }
        }
        return Ok((VolumesQueried { volumes }, warnings));
    };
    let (_, def) = common::find(own, name, "volume")?;
    let (mut r, w) = common::compute(doc, def)?;
    r.basis = common::basis_of(&r);
    r.sources = common::sources_of(doc, def);
    Ok((VolumesQueried { volumes: vec![r] }, w))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &VolumesQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &VolumesQuery) -> CommandResult<VolumesQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: VolumesQuery) -> CommandResult<VolumesQueried> {
    plan(cx, &input)
}
