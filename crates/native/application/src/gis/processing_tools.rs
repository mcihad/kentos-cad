//! `cad.processing.tools` (docs/adr/0508): İşlemler's tools and their parameters, for a script or an agent to choose
//! one and give its values. It reads the drawing only for the defaults.

use kentos_commands::{CommandResult, ProcessingTools, ProcessingToolsListed};

use crate::ExecutionContext;
use crate::processing_host::host;

fn listed(
    cx: &ExecutionContext<'_>,
    input: &ProcessingTools,
) -> CommandResult<ProcessingToolsListed> {
    match host().and_then(|h| h.list(cx.doc, input)) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(error) => CommandResult::Failed { error: *error },
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ProcessingTools) -> CommandResult<()> {
    match listed(cx, input) {
        CommandResult::Completed { warnings, .. } => CommandResult::Completed {
            output: (),
            warnings,
        },
        CommandResult::Failed { error } => CommandResult::Failed { error },
        _ => CommandResult::Cancelled,
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ProcessingTools,
) -> CommandResult<ProcessingToolsListed> {
    listed(cx, input)
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ProcessingTools,
) -> CommandResult<ProcessingToolsListed> {
    listed(cx, &input)
}
