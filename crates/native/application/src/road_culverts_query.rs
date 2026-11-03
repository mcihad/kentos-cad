//! `road.culverts.query` v1 (plan/road.md Y11.2, docs/adr/0909 §2): every culvert read now, or one. Reads only. The
//! shared cases are `fixtures/commands/v1/road.culverts.query.json`.

use kentos_commands::{CommandResult, CommandWarning, CulvertsQueried, CulvertsQuery};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::road_culverts as common;

fn answer(doc: &kentos_domain::Document, input: &CulvertsQuery) -> Result<(CulvertsQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = doc.road_culverts();
    let Some(name) = input.culvert.as_deref() else {
        let mut warnings = Vec::new();
        let mut culverts = Vec::new();
        for g in own {
            match common::read(doc, g) {
                Ok(r) => {
                    warnings.extend(r.warnings);
                    culverts.push(r.report);
                }
                Err(_) => culverts.push(common::broken(g)),
            }
        }
        return Ok((CulvertsQueried { culverts }, warnings));
    };
    let (_, def) = common::find(own, name, "culvert")?;
    let r = common::read(doc, def)?;
    Ok((CulvertsQueried { culverts: vec![r.report] }, r.warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &CulvertsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &CulvertsQuery) -> CommandResult<CulvertsQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CulvertsQuery) -> CommandResult<CulvertsQueried> {
    plan(cx, &input)
}

/// The culverts for a window, or why they do not read.
pub fn preview(doc: &kentos_domain::Document, input: &CulvertsQuery) -> Result<(CulvertsQueried, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
