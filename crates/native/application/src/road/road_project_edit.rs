//! `road.project.edit` v1 (plan/road.md Y13.2, docs/adr/0911 §3, docs/adr/4740): drawing the stale road drawings again
//! by itself, on or off, and whether drawing again writes only what changed; one undo step. The shared cases are
//! `fixtures/commands/v1/road.project.edit.json`.

use kentos_commands::{CommandResult, RoadProjectEdit, RoadProjectEdited};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// What the edit sets: drawing again by itself and only what changed, as they will be.
struct Set {
    auto: Option<bool>,
    regional: Option<bool>,
}

fn check(doc: &kentos_domain::Document, input: &RoadProjectEdit) -> Result<Set, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    crate::road_project::road_project(doc)?;
    let auto = input.auto_rebuild.filter(|on| *on != doc.road_auto_rebuild());
    let regional = input.regional.filter(|on| *on == doc.road_rebuild_whole());
    if auto.is_none() && regional.is_none() {
        let (words, path) = if input.auto_rebuild.is_some() || input.regional.is_none() {
            ("Değişecek bir şey yok: kendiliğinden yeniden üretme zaten böyle.", "autoRebuild")
        } else {
            ("Değişecek bir şey yok: yeniden üretim zaten böyle yazıyor.", "regional")
        };
        return Err(Stop::failed(error(codes::NOTHING_TO_SET, words.into(), Some(path.into()))));
    }
    Ok(Set { auto, regional })
}

fn edited(doc: &kentos_domain::Document, set: &Set) -> RoadProjectEdited {
    RoadProjectEdited {
        auto_rebuild: set.auto.unwrap_or_else(|| doc.road_auto_rebuild()),
        regional: set.regional.unwrap_or_else(|| !doc.road_rebuild_whole()),
        revision: doc.revision().to_string(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: RoadProjectEdit) -> CommandResult<RoadProjectEdited> {
    let set = match check(cx.doc, &input) {
        Ok(set) => set,
        Err(stop) => return stop.into(),
    };
    let label = match (set.auto, set.regional) {
        (Some(_), Some(_)) => "Yeniden üretim",
        (None, Some(_)) => "Yalnız değişeni yaz",
        _ => "Kendiliğinden yeniden üret",
    };
    // Both in one step when both change.
    let step = cx.doc.begin_group(label);
    if let Some(on) = set.auto {
        cx.doc.set_road_auto_rebuild(on, label);
    }
    if let Some(on) = set.regional {
        cx.doc.set_road_rebuild_whole(!on, label);
    }
    cx.doc.end_group(step);
    CommandResult::Completed { output: edited(cx.doc, &set), warnings: Vec::new() }
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
        Ok(set) => CommandResult::Completed { output: edited(cx.doc, &set), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadProjectEdit) -> CommandResult<RoadProjectEdited> {
    crate::call::recorded(cx, kentos_commands::ROAD_PROJECT_EDIT, kentos_commands::ROAD_PROJECT_EDIT_VERSION, input, write)
}
