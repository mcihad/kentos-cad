//! `road.walls.query` v1 (plan/road.md Y11.3, docs/adr/0909 §3): every retaining wall read now, or one. Reads only. The
//! shared cases are `fixtures/commands/v1/road.walls.query.json`.

use kentos_commands::{CommandResult, CommandWarning, RetainingWallsQueried, RetainingWallsQuery};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::road_walls as common;

fn answer(doc: &kentos_domain::Document, input: &RetainingWallsQuery) -> Result<(RetainingWallsQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = doc.road_walls();
    let Some(name) = input.wall.as_deref() else {
        let mut warnings = Vec::new();
        let mut walls = Vec::new();
        for g in own {
            match common::read(doc, g) {
                Ok(r) => {
                    warnings.extend(r.warnings);
                    walls.push(r.report);
                }
                Err(_) => walls.push(common::broken(g)),
            }
        }
        return Ok((RetainingWallsQueried { walls }, warnings));
    };
    let (_, def) = common::find(own, name, "wall")?;
    let r = common::read(doc, def)?;
    Ok((RetainingWallsQueried { walls: vec![r.report] }, r.warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &RetainingWallsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &RetainingWallsQuery) -> CommandResult<RetainingWallsQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RetainingWallsQuery) -> CommandResult<RetainingWallsQueried> {
    plan(cx, &input)
}

/// The walls for a window, or why they do not read.
pub fn preview(doc: &kentos_domain::Document, input: &RetainingWallsQuery) -> Result<(RetainingWallsQueried, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
