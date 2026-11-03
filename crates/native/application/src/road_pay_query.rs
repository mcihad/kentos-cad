//! `road.pay.query` v1 (plan/road.md Y11.4, docs/adr/0909 §4): the book's items' quantities now. Reads only. The shared
//! cases are `fixtures/commands/v1/road.pay.query.json`.

use kentos_commands::{CommandResult, CommandWarning, PayQueried, PayQuery};

use crate::ExecutionContext;
use crate::road_pay as common;

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &PayQuery) -> CommandResult<()> {
    match common::answer(cx.doc, input.item.as_deref()) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &PayQuery) -> CommandResult<PayQueried> {
    match common::answer(cx.doc, input.item.as_deref()) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PayQuery) -> CommandResult<PayQueried> {
    plan(cx, &input)
}

/// The quantities for a window, or why they do not read.
pub fn preview(doc: &kentos_domain::Document) -> Result<(PayQueried, Vec<CommandWarning>), String> {
    common::answer(doc, None).map_err(crate::road_corridors::words)
}
