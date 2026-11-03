//! `road.junctions.query` v1 (plan/road.md Y12.2, docs/adr/0910 §2): every junction read now, or one. Reads only. The
//! shared cases are `fixtures/commands/v1/road.junctions.query.json`.

use kentos_commands::{CommandResult, CommandWarning, JunctionsQueried, JunctionsQuery};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::road_junctions as common;

fn answer(doc: &kentos_domain::Document, input: &JunctionsQuery) -> Result<(JunctionsQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = doc.road_junctions();
    let Some(name) = input.junction.as_deref() else {
        let mut warnings = Vec::new();
        let mut junctions = Vec::new();
        for g in own {
            match common::read(doc, g) {
                Ok(r) => {
                    warnings.extend(r.1);
                    junctions.push(r.0);
                }
                Err(_) => junctions.push(common::broken(g)),
            }
        }
        return Ok((JunctionsQueried { junctions }, warnings));
    };
    let (_, def) = common::find(own, name, "junction")?;
    let r = common::read(doc, def)?;
    Ok((JunctionsQueried { junctions: vec![r.0] }, r.1))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &JunctionsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &JunctionsQuery) -> CommandResult<JunctionsQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: JunctionsQuery) -> CommandResult<JunctionsQueried> {
    plan(cx, &input)
}

/// The junctions for a window, or why they do not read.
pub fn preview(doc: &kentos_domain::Document, input: &JunctionsQuery) -> Result<(JunctionsQueried, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
