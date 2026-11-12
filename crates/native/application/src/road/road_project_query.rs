//! `road.project.query` v1 (plan/road.md Y13.1, docs/adr/0911 §2): the road project's tree, its sources and drawings
//! with how they stand now. Reads only. The shared cases are `fixtures/commands/v1/road.project.query.json`.

use kentos_commands::{CommandResult, RoadProjectQuery, RoadProjectTree};

use crate::ExecutionContext;

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoadProjectQuery) -> CommandResult<()> {
    match crate::road_project::tree(cx.doc, input.deep) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoadProjectQuery) -> CommandResult<RoadProjectTree> {
    match crate::road_project::tree(cx.doc, input.deep) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadProjectQuery) -> CommandResult<RoadProjectTree> {
    plan(cx, &input)
}

/// The tree for a panel, or why it does not read.
pub fn preview(doc: &kentos_domain::Document, deep: bool) -> Result<RoadProjectTree, String> {
    crate::road_project::tree(doc, deep).map_err(crate::road_corridors::words)
}
