//! `road.routes.find` v1 (plan/road.md Y12.1, docs/adr/0910 §1): the least-cost route between two places over a
//! terrain. Reads only. The shared cases are `fixtures/commands/v1/road.routes.find.json`.

use kentos_commands::{CommandResult, RouteFound, RoutesFind};

use crate::ExecutionContext;

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoutesFind) -> CommandResult<()> {
    match crate::road_routes::find(cx.doc, &input.ask) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoutesFind) -> CommandResult<RouteFound> {
    match crate::road_routes::find(cx.doc, &input.ask) {
        Ok(output) => CommandResult::Completed { output, warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RoutesFind) -> CommandResult<RouteFound> {
    plan(cx, &input)
}

/// The route for a window, or why there is none.
pub fn preview(doc: &kentos_domain::Document, input: &RoutesFind) -> Result<RouteFound, String> {
    crate::road_routes::find(doc, &input.ask).map_err(crate::road_corridors::words)
}
