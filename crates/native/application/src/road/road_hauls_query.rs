//! `road.hauls.query` v1 (plan/road.md Y9, docs/adr/0907): every mass haul diagram read now, or one with its
//! ordinate's points. Reads only. The shared cases are `fixtures/commands/v1/road.hauls.query.json`.

use kentos_commands::{CommandResult, CommandWarning, HaulsQueried, HaulsQuery};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors::words;
use crate::road_hauls as common;
use crate::road_terrains::fail;

fn answer(doc: &kentos_domain::Document, input: &HaulsQuery) -> Result<(HaulsQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = doc.road_hauls();
    let Some(name) = input.haul.as_deref() else {
        if input.points {
            return Err(fail(
                codes::INVALID_HAUL,
                "Noktalar bir Brüknerde sorulur: Brüknerin kimliğini ya da adını (haul) verin.".into(),
                "haul",
            ));
        }
        let mut warnings = Vec::new();
        let mut hauls = Vec::new();
        for h in own {
            match common::read(doc, h, false) {
                Ok((r, w)) => {
                    warnings.extend(w);
                    hauls.push(r);
                }
                Err(stop) => hauls.push(common::broken(h, words(stop))),
            }
        }
        return Ok((HaulsQueried { hauls }, warnings));
    };
    let (_, def) = common::find(own, name, "haul")?;
    let (r, w) = common::read(doc, def, input.points)?;
    Ok((HaulsQueried { hauls: vec![r] }, w))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &HaulsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &HaulsQuery) -> CommandResult<HaulsQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: HaulsQuery) -> CommandResult<HaulsQueried> {
    plan(cx, &input)
}
