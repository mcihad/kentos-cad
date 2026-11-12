//! `road.corridors.query` v1 (plan/road.md Y7, docs/adr/0905): every corridor built now, or one with its feature lines'
//! points and its top surface's triangles. Reads only. The shared cases are
//! `fixtures/commands/v1/road.corridors.query.json`.

use kentos_commands::{CommandResult, CommandWarning, CorridorsQueried, CorridorsQuery};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors as common;
use crate::road_terrains::fail;

fn answer(doc: &kentos_domain::Document, input: &CorridorsQuery) -> Result<(CorridorsQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = doc.road_corridors();
    let Some(name) = input.corridor.as_deref() else {
        if input.points || input.triangles {
            return Err(fail(
                codes::INVALID_ROAD_CORRIDOR,
                "Noktalar ve üçgenler bir koridorda sorulur: koridorun kimliğini ya da adını (corridor) verin.".into(),
                "corridor",
            ));
        }
        let mut warnings = Vec::new();
        let mut corridors = Vec::new();
        for c in own {
            match common::make(doc, c, false) {
                Ok(m) => {
                    warnings.extend(m.warnings);
                    corridors.push(m.report);
                }
                Err(stop) => corridors.push(common::broken(doc, c, common::words(stop))),
            }
        }
        return Ok((CorridorsQueried { corridors, triangles: vec![] }, warnings));
    };
    let (_, def) = common::find(own, name, "corridor")?;
    let m = common::make(doc, def, input.points)?;
    let triangles = if input.triangles { m.built.triangles } else { Vec::new() };
    Ok((CorridorsQueried { corridors: vec![m.report], triangles }, m.warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &CorridorsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &CorridorsQuery) -> CommandResult<CorridorsQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CorridorsQuery) -> CommandResult<CorridorsQueried> {
    plan(cx, &input)
}
