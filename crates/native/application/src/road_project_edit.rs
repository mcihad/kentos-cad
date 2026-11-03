//! `road.project.edit` v1 (plan/road.md Y13.2, docs/adr/0911 §3): drawing the stale road drawings again by itself, on
//! or off; one undo step “Kendiliğinden yeniden üret”. The shared cases are `fixtures/commands/v1/road.project.edit.json`.

use kentos_commands::{CommandResult, RoadProjectEdit, RoadProjectEdited};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn check(doc: &kentos_domain::Document, input: &RoadProjectEdit) -> Result<bool, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    crate::road_project::road_project(doc)?;
    match input.auto_rebuild {
        Some(on) if on != doc.road_auto_rebuild() => Ok(on),
        _ => Err(Stop::failed(error(
            codes::NOTHING_TO_SET,
            "Değişecek bir şey yok: kendiliğinden yeniden üretme zaten böyle.".into(),
            Some("autoRebuild".into()),
        ))),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: RoadProjectEdit) -> CommandResult<RoadProjectEdited> {
    let on = match check(cx.doc, &input) {
        Ok(on) => on,
        Err(stop) => return stop.into(),
    };
    cx.doc.set_road_auto_rebuild(on, "Kendiliğinden yeniden üret");
    CommandResult::Completed {
        output: RoadProjectEdited { auto_rebuild: on, revision: cx.doc.revision().to_string() },
        warnings: Vec::new(),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoadProjectEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would set; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoadProjectEdit) -> CommandResult<RoadProjectEdited> {
    match check(cx.doc, input) {
        Ok(on) => CommandResult::Completed {
            output: RoadProjectEdited { auto_rebuild: on, revision: cx.doc.revision().to_string() },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadProjectEdit) -> CommandResult<RoadProjectEdited> {
    crate::call::recorded(cx, kentos_commands::ROAD_PROJECT_EDIT, kentos_commands::ROAD_PROJECT_EDIT_VERSION, input, write)
}
